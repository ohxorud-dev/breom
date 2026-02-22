use anyhow::Result;
use std::collections::HashMap;

use crate::ast::program::{Program, TopLevelItem};
use crate::codegen::{CodeGen, CompileMode};
use crate::project;
use crate::{BreomParser, CollectedTests, CompileFailTest, Rule};
use pest::Parser;

pub(crate) fn parse_source(source: &str) -> Result<Program, anyhow::Error> {
    let pairs = BreomParser::parse(Rule::program, source)
        .map_err(|e| anyhow::anyhow!("Parse error:\n{}", e))?;

    let pair = pairs
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("No program found"))?;

    Ok(Program::from_pair(pair))
}

pub(crate) fn compile_and_run(
    programs: Vec<&Program>,
    run: bool,
    entry_package: &str,
) -> Result<i64, anyhow::Error> {
    let mut codegen = CodeGen::new()?;
    codegen.set_entry_package(entry_package);

    for program in &programs {
        codegen.preprocess_program(program)?;
    }

    for program in &programs {
        codegen.declare_program(program)?;
    }

    for program in &programs {
        codegen.compile_program_bodies(program)?;
    }

    codegen.finalize()?;

    if run {
        let result = codegen.run_main()?;
        Ok(result)
    } else {
        Ok(0)
    }
}

pub(crate) fn compile_programs_in_test_mode(
    programs: Vec<&Program>,
    entry_package: &str,
) -> Result<(), anyhow::Error> {
    let mut codegen = CodeGen::new()?;
    codegen.set_entry_package(entry_package);
    codegen.set_compile_mode(CompileMode::Test);

    for program in &programs {
        codegen.preprocess_program(program)?;
    }

    for program in &programs {
        codegen.declare_program(program)?;
    }

    for program in &programs {
        codegen.compile_program_bodies(program)?;
    }

    codegen.finalize()?;
    Ok(())
}

pub(crate) fn build_test_programs(
    source_files: &[project::SourceFile],
    sorted_packages: &[String],
    compile_fail_target: Option<&CompileFailTest>,
) -> (Vec<Program>, bool) {
    let mut package_map: HashMap<String, Vec<Program>> = HashMap::new();
    let mut found_target = false;

    for source in source_files {
        let mut program = source.program.clone();
        program.items.retain(|item| {
            if let TopLevelItem::Function(func) = item {
                let has_compile_fail = func
                    .attributes
                    .iter()
                    .any(|attr| attr.name == "compile_fail");
                if !has_compile_fail {
                    return true;
                }

                if let Some(target) = compile_fail_target {
                    if source.path == target.source_path && func.name == target.function_name {
                        found_target = true;
                        return true;
                    }
                }
                return false;
            }
            true
        });
        package_map
            .entry(source.package.clone())
            .or_default()
            .push(program);
    }

    let mut programs = Vec::new();
    for package in sorted_packages {
        if let Some(pkg_programs) = package_map.get(package) {
            programs.extend(pkg_programs.iter().cloned());
        }
    }

    (programs, found_target)
}

pub(crate) fn compile_and_test(
    source_files: &[project::SourceFile],
    sorted_packages: &[String],
    tests: CollectedTests,
    entry_package: &str,
    pattern: Option<&str>,
    verbose: bool,
) -> Result<i64, anyhow::Error> {
    let mut runtime_exit = 3;
    let mut ran_runtime_tests = false;

    if !tests.runtime_tests.is_empty() {
        let (runtime_programs, _) = build_test_programs(source_files, sorted_packages, None);
        let runtime_program_refs: Vec<&Program> = runtime_programs.iter().collect();
        let mut codegen = CodeGen::new()?;
        codegen.set_entry_package(entry_package);
        codegen.set_compile_mode(CompileMode::Test);
        codegen.set_tests(tests.runtime_tests.clone());

        for program in &runtime_program_refs {
            codegen.preprocess_program(program)?;
        }

        for program in &runtime_program_refs {
            codegen.declare_program(program)?;
        }

        for program in &runtime_program_refs {
            codegen.compile_program_bodies(program)?;
        }

        codegen.finalize()?;
        runtime_exit = codegen.run_tests(pattern, verbose)?;
        ran_runtime_tests = true;
    }

    let mut compile_fail_total = 0i64;
    let mut compile_fail_failed = 0i64;

    for test in &tests.compile_fail_tests {
        if let Some(filter_pattern) = pattern {
            if !test.display_name.contains(filter_pattern)
                && !test.stable_name.contains(filter_pattern)
            {
                continue;
            }
        }

        compile_fail_total += 1;

        let (compile_fail_programs, found_target) =
            build_test_programs(source_files, sorted_packages, Some(test));

        let compile_result: Result<(), anyhow::Error> = if !found_target {
            Err(anyhow::anyhow!(
                "Compile-fail target '{}' not found",
                test.display_name
            ))
        } else {
            let compile_fail_program_refs: Vec<&Program> = compile_fail_programs.iter().collect();
            compile_programs_in_test_mode(compile_fail_program_refs, entry_package)
        };

        match compile_result {
            Ok(_) => {
                compile_fail_failed += 1;
                println!(
                    "[FAIL] {} (expected compile failure containing '{}')",
                    test.display_name, test.contains
                );
            }
            Err(err) => {
                let msg = err.to_string();
                if msg.contains(&test.contains) {
                    if verbose {
                        println!("[PASS] {}", test.display_name);
                    }
                } else {
                    compile_fail_failed += 1;
                    println!(
                        "[FAIL] {} (expected error containing '{}', got '{}')",
                        test.display_name, test.contains, msg
                    );
                }
            }
        }
    }

    let mut parser_fail_total = 0i64;
    let mut parser_fail_failed = 0i64;

    for test in &tests.parser_fail_tests {
        if let Some(filter_pattern) = pattern {
            if !test.display_name.contains(filter_pattern)
                && !test.stable_name.contains(filter_pattern)
            {
                continue;
            }
        }

        parser_fail_total += 1;

        let content = match std::fs::read_to_string(&test.fixture_path) {
            Ok(content) => content,
            Err(err) => {
                parser_fail_failed += 1;
                println!(
                    "[FAIL] {} (failed to read parser fixture '{}': {})",
                    test.display_name,
                    test.fixture_path.display(),
                    err
                );
                continue;
            }
        };

        let compile_result: Result<(), anyhow::Error> = match parse_source(&content) {
            Ok(program) => compile_programs_in_test_mode(vec![&program], "main"),
            Err(err) => Err(err),
        };

        match compile_result {
            Ok(_) => {
                parser_fail_failed += 1;
                println!(
                    "[FAIL] {} (expected parser/compile failure from fixture '{}')",
                    test.display_name,
                    test.fixture_path.display()
                );
            }
            Err(_) => {
                if verbose {
                    println!("[PASS] {}", test.display_name);
                }
            }
        }
    }

    if compile_fail_total > 0 {
        let compile_fail_passed = compile_fail_total - compile_fail_failed;
        println!(
            "\n{} compile-fail passed, {} compile-fail failed, {} total",
            compile_fail_passed, compile_fail_failed, compile_fail_total
        );
    }

    if parser_fail_total > 0 {
        let parser_fail_passed = parser_fail_total - parser_fail_failed;
        println!(
            "{} parser-fail passed, {} parser-fail failed, {} total",
            parser_fail_passed, parser_fail_failed, parser_fail_total
        );
    }

    let has_any_tests = (runtime_exit != 3) || (compile_fail_total > 0) || (parser_fail_total > 0);
    if !has_any_tests {
        if !ran_runtime_tests {
            println!("No tests found");
        }
        return Ok(3);
    }

    if runtime_exit == 1 || compile_fail_failed > 0 || parser_fail_failed > 0 {
        Ok(1)
    } else {
        Ok(0)
    }
}
