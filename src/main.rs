use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::env;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process;

use pest_derive::Parser;

mod ast;
mod codegen;
mod lsp;
mod pipeline;
mod project;
mod runtime;

use ast::attributes::Attribute;
use ast::declarations::StructMember;
use ast::expressions::{Expression, Literal};
use ast::program::{Program, TopLevelItem};
use ast::types::TypeExpr;

use crate::pipeline::{compile_and_run, compile_and_test};
#[cfg(test)]
use ast::common::Span;
#[cfg(test)]
use ast::program::{ModuleDecl, ModulePath};
use codegen::TestFunction;
use project::Project;

#[cfg(test)]
use crate::pipeline::parse_source;

#[derive(Debug, Clone)]
pub(crate) struct CompileFailTest {
    display_name: String,
    stable_name: String,
    source_path: PathBuf,
    function_name: String,
    contains: String,
}

#[derive(Debug, Clone)]
pub(crate) struct ParserFailTest {
    display_name: String,
    stable_name: String,
    fixture_path: PathBuf,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct CollectedTests {
    runtime_tests: Vec<TestFunction>,
    compile_fail_tests: Vec<CompileFailTest>,
    parser_fail_tests: Vec<ParserFailTest>,
}

#[derive(Parser)]
#[grammar = "breom.pest"]
pub struct BreomParser;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        process::exit(1);
    }

    let result = match args[1].as_str() {
        "init" => cmd_init(&args[2..]).map(|_| 0),
        "run" => cmd_run(&args[2..]),
        "test" => cmd_test(&args[2..]),
        "build" => cmd_build(&args[2..]).map(|_| 0),
        "lsp" => cmd_lsp().await.map(|_| 0),
        "help" | "--help" | "-h" => {
            print_usage();
            Ok(0)
        }
        arg if arg.ends_with(".brm") => run_file(Path::new(arg)),
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            print_usage();
            process::exit(1);
        }
    };

    match result {
        Ok(code) => process::exit(code as i32),
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    }
}

fn print_usage() {
    println!("Breom Programming Language Compiler\n");
    println!("Usage:");
    println!("  breom init [name]           Initialize a new project");
    println!("  breom run                   Run project in current directory");
    println!("  breom run <file.brm>        Run a single file");
    println!("  breom test [target] [--filter pattern] [--verbose]");
    println!("  breom build [path]          Build project in current directory or path");
    println!("  breom lsp                   Start Language Server Protocol server");
    println!("  breom <file.brm>            Run a single file (shorthand)");
}

async fn cmd_lsp() -> Result<(), anyhow::Error> {
    lsp::run_lsp_server().await
}

fn cmd_init(args: &[String]) -> Result<(), anyhow::Error> {
    use std::fs;

    let project_name = if args.is_empty() {
        std::env::current_dir()?
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("myproject")
            .to_string()
    } else {
        args[0].clone()
    };

    let project_file = Path::new("project.breom");
    if project_file.exists() {
        return Err(anyhow::anyhow!("project.breom already exists"));
    }

    let project_content = format!(
        "breom {}\npackage {}\n\nentrypoint main.brm\n",
        env!("CARGO_PKG_VERSION"),
        project_name
    );
    fs::write(project_file, project_content)?;

    let main_file = Path::new("main.brm");
    if !main_file.exists() {
        let main_content = r#"fn main() Int {
    return 0
}
"#
        .to_string();
        fs::write(main_file, main_content)?;
    }

    println!("\nProject '{}' initialized!", project_name);
    println!("Run with: breom run");

    Ok(())
}

fn cmd_run(args: &[String]) -> Result<i64, anyhow::Error> {
    if args.is_empty() {
        let project_file = Path::new("project.breom");
        if project_file.exists() {
            run_project(project_file)
        } else {
            let main_file = Path::new("main.brm");
            if main_file.exists() {
                run_file(main_file)
            } else {
                Err(anyhow::anyhow!("No project.breom or main.brm found"))
            }
        }
    } else {
        let file_path = Path::new(&args[0]);
        if file_path.extension().is_some_and(|e| e == "brm") {
            run_file(file_path)
        } else if file_path.is_dir() {
            run_project(&file_path.join("project.breom"))
        } else {
            Err(anyhow::anyhow!("Invalid argument: {}", args[0]))
        }
    }
}

#[derive(Debug, Clone)]
enum TestTarget {
    Project(PathBuf),
    File(PathBuf),
}

fn is_path_like(s: &str) -> bool {
    s == "."
        || s == ".."
        || s.starts_with("./")
        || s.starts_with("../")
        || s.starts_with('/')
        || s.contains('/')
        || s.ends_with(".brm")
}

fn recursive_base_path(spec: &str) -> Option<PathBuf> {
    if spec == "..." || spec == "./..." {
        Some(PathBuf::from("."))
    } else {
        spec.strip_suffix("/...").map(PathBuf::from)
    }
}

fn is_test_source_file(path: &Path) -> bool {
    path.file_name()
        .and_then(OsStr::to_str)
        .map(|name| name.ends_with("_test.brm"))
        .unwrap_or(false)
}

fn is_parser_fail_fixture_file(path: &Path) -> bool {
    path.file_name()
        .and_then(OsStr::to_str)
        .map(|name| name.ends_with("_fail.brm"))
        .unwrap_or(false)
}

fn filter_source_paths(source_paths: Vec<PathBuf>, include_test_files: bool) -> Vec<PathBuf> {
    source_paths
        .into_iter()
        .filter(|path| {
            if is_parser_fail_fixture_file(path) {
                return false;
            }
            if include_test_files {
                true
            } else {
                !is_test_source_file(path)
            }
        })
        .collect()
}

fn string_literal_from_expr(expr: &Expression) -> Option<String> {
    match expr {
        Expression::Literal(Literal::String(value, _)) => Some(value.clone()),
        Expression::Literal(Literal::MultilineString(value, _)) => Some(value.clone()),
        _ => None,
    }
}

fn collect_test_functions(source_files: &[project::SourceFile]) -> Result<CollectedTests> {
    let mut runtime_tests = Vec::new();
    let mut compile_fail_tests = Vec::new();
    let mut parser_fail_tests = Vec::new();
    for source in source_files {
        if !is_test_source_file(&source.path) {
            continue;
        }
        for item in &source.program.items {
            if let TopLevelItem::Function(func) = item {
                let has_test = func.attributes.iter().any(|attr| attr.name == "test");
                let compile_fail_attr = func
                    .attributes
                    .iter()
                    .find(|attr| attr.name == "compile_fail");
                let parser_fail_attr = func
                    .attributes
                    .iter()
                    .find(|attr| attr.name == "parser_fail");

                if compile_fail_attr.is_some() && !has_test {
                    return Err(anyhow::anyhow!(
                        "Attribute '@compile_fail' on function '{}' requires '@test' ({})",
                        func.name,
                        source.path.display()
                    ));
                }

                if parser_fail_attr.is_some() && !has_test {
                    return Err(anyhow::anyhow!(
                        "Attribute '@parser_fail' on function '{}' requires '@test' ({})",
                        func.name,
                        source.path.display()
                    ));
                }

                if compile_fail_attr.is_some() && parser_fail_attr.is_some() {
                    return Err(anyhow::anyhow!(
                        "Function '{}' cannot use both '@compile_fail' and '@parser_fail' ({})",
                        func.name,
                        source.path.display()
                    ));
                }

                if !has_test {
                    continue;
                }

                if let Some(attr) = compile_fail_attr {
                    let contains = string_literal_from_expr(&attr.args[0]).ok_or_else(|| {
                        anyhow::anyhow!(
                            "Attribute '@compile_fail' on function '{}' requires string literal as first argument ({})",
                            func.name,
                            source.path.display()
                        )
                    })?;

                    compile_fail_tests.push(CompileFailTest {
                        display_name: format!("{}::{}", source.path.display(), func.name),
                        stable_name: func.name.clone(),
                        source_path: source.path.clone(),
                        function_name: func.name.clone(),
                        contains,
                    });
                    continue;
                }

                if let Some(attr) = parser_fail_attr {
                    let fixture_rel = string_literal_from_expr(&attr.args[0]).ok_or_else(|| {
                        anyhow::anyhow!(
                            "Attribute '@parser_fail' on function '{}' requires string literal as first argument ({})",
                            func.name,
                            source.path.display()
                        )
                    })?;
                    let fixture_path = source
                        .path
                        .parent()
                        .filter(|p| !p.as_os_str().is_empty())
                        .unwrap_or(Path::new("."))
                        .join(fixture_rel);

                    parser_fail_tests.push(ParserFailTest {
                        display_name: format!("{}::{}", source.path.display(), func.name),
                        stable_name: func.name.clone(),
                        fixture_path,
                    });
                    continue;
                }

                if !func.params.is_empty() {
                    return Err(anyhow::anyhow!(
                        "Test function '{}' must have zero parameters ({})",
                        func.name,
                        source.path.display()
                    ));
                }
                if func.throws {
                    return Err(anyhow::anyhow!(
                        "Test function '{}' must not use throws ({})",
                        func.name,
                        source.path.display()
                    ));
                }
                let returns_void = match &func.return_type {
                    None => true,
                    Some(TypeExpr::Base(base)) if base.name == "Void" => true,
                    _ => false,
                };
                if !returns_void {
                    return Err(anyhow::anyhow!(
                        "Test function '{}' must return Void ({})",
                        func.name,
                        source.path.display()
                    ));
                }

                runtime_tests.push(TestFunction {
                    display_name: format!("{}::{}", source.path.display(), func.name),
                    stable_name: func.name.clone(),
                    function_name: format!("{}.{}", source.package, func.name),
                });
            }
        }
    }
    runtime_tests.sort_by(|a, b| a.display_name.cmp(&b.display_name));
    compile_fail_tests.sort_by(|a, b| a.display_name.cmp(&b.display_name));
    parser_fail_tests.sort_by(|a, b| a.display_name.cmp(&b.display_name));
    Ok(CollectedTests {
        runtime_tests,
        compile_fail_tests,
        parser_fail_tests,
    })
}

fn collect_declared_attributes(
    source_files: &[project::SourceFile],
) -> Result<HashMap<String, usize>> {
    let mut known_attributes = HashMap::new();
    known_attributes.insert("test".to_string(), 0);
    known_attributes.insert("compile_fail".to_string(), 1);
    known_attributes.insert("parser_fail".to_string(), 1);
    known_attributes.insert("resolve_inherit".to_string(), 2);
    known_attributes.insert("inherit_from".to_string(), 1);

    for source in source_files {
        for item in &source.program.items {
            if let TopLevelItem::AttributeDecl(decl) = item {
                let arity = decl.params.len();
                if let Some(existing) = known_attributes.get(&decl.name) {
                    if *existing != arity {
                        return Err(anyhow::anyhow!(
                            "Attribute '{}' declared with conflicting parameter count in {} (expected {}, got {})",
                            decl.name,
                            source.path.display(),
                            existing,
                            arity
                        ));
                    }
                } else {
                    known_attributes.insert(decl.name.clone(), arity);
                }
            }
        }
    }

    Ok(known_attributes)
}

fn validate_source_attributes(
    source: &project::SourceFile,
    known_attributes: &HashMap<String, usize>,
) -> Result<()> {
    let validate_attr = |attr: &Attribute, owner_desc: &str| -> Result<()> {
        if attr.name.is_empty() {
            return Err(anyhow::anyhow!("Invalid empty attribute on {}", owner_desc));
        }
        let expected_arity = match known_attributes.get(&attr.name) {
            Some(arity) => *arity,
            None => {
                return Err(anyhow::anyhow!(
                    "Unknown attribute '@{}' on {}",
                    attr.name,
                    owner_desc
                ));
            }
        };
        if attr.args.len() != expected_arity {
            return Err(anyhow::anyhow!(
                "Attribute '@{}' on {} expects {} argument(s), got {}",
                attr.name,
                owner_desc,
                expected_arity,
                attr.args.len()
            ));
        }
        Ok(())
    };

    for item in &source.program.items {
        if let TopLevelItem::Function(func) = item {
            let owner = format!("function '{}' ({})", func.name, source.path.display());
            for attr in &func.attributes {
                validate_attr(attr, &owner)?;
            }
        }

        if let TopLevelItem::Struct(struct_decl) = item {
            let owner = format!("struct '{}' ({})", struct_decl.name, source.path.display());
            for attr in &struct_decl.attributes {
                validate_attr(attr, &owner)?;
            }

            for member in &struct_decl.members {
                if let StructMember::Method(method) = member {
                    let owner = format!(
                        "method '{}.{}' ({})",
                        struct_decl.name,
                        method.name,
                        source.path.display()
                    );
                    for attr in &method.attributes {
                        validate_attr(attr, &owner)?;
                    }
                }
            }
        }
    }
    Ok(())
}

fn collect_recursive_test_targets(base: &Path) -> Result<Vec<TestTarget>, anyhow::Error> {
    if !base.exists() {
        return Err(anyhow::anyhow!("Path not found: {}", base.display()));
    }
    if !base.is_dir() {
        return Err(anyhow::anyhow!(
            "Recursive test target must be a directory: {}",
            base.display()
        ));
    }

    let mut project_roots = Vec::<PathBuf>::new();
    let mut standalone_files = Vec::<PathBuf>::new();
    let mut stack = vec![base.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let dir = dir.canonicalize().unwrap_or(dir);
        let project_file = dir.join("project.breom");
        if project_file.exists() {
            project_roots.push(project_file);
            continue;
        }

        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                let name = path.file_name().and_then(OsStr::to_str).unwrap_or_default();
                if name == ".git" || name == "target" {
                    continue;
                }
                stack.push(path);
            } else if path.extension().and_then(OsStr::to_str) == Some("brm")
                && is_test_source_file(&path)
            {
                standalone_files.push(path);
            }
        }
    }

    project_roots.sort();
    project_roots.dedup();

    let project_dirs: Vec<PathBuf> = project_roots
        .iter()
        .filter_map(|p| p.parent().map(Path::to_path_buf))
        .collect();

    let mut filtered_files = Vec::new();
    for file in standalone_files {
        let canonical = file.canonicalize().unwrap_or(file.clone());
        if project_dirs.iter().any(|root| canonical.starts_with(root)) {
            continue;
        }
        filtered_files.push(canonical);
    }

    filtered_files.sort();
    filtered_files.dedup();

    let mut targets = Vec::new();
    targets.extend(project_roots.into_iter().map(TestTarget::Project));
    targets.extend(filtered_files.into_iter().map(TestTarget::File));
    Ok(targets)
}

fn run_test_target(
    target: &TestTarget,
    filter: Option<&str>,
    verbose: bool,
) -> Result<i64, anyhow::Error> {
    match target {
        TestTarget::Project(project_file) => test_project(project_file, filter, verbose),
        TestTarget::File(file_path) => test_file(file_path, filter, verbose),
    }
}

fn normalize_single_test_exit(code: i64) -> i64 {
    if code == 3 {
        1
    } else {
        code
    }
}

fn finalize_single_test_result(result: Result<i64, anyhow::Error>) -> Result<i64, anyhow::Error> {
    match result {
        Ok(code) => Ok(normalize_single_test_exit(code)),
        Err(e) => {
            eprintln!("[ERROR] {}", e);
            Ok(2)
        }
    }
}

fn cmd_test(args: &[String]) -> Result<i64, anyhow::Error> {
    let mut filter: Option<String> = None;
    let mut verbose = false;
    let mut positional: Vec<String> = Vec::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--verbose" | "-v" => {
                verbose = true;
                i += 1;
            }
            "--filter" | "-f" => {
                if i + 1 >= args.len() {
                    return Err(anyhow::anyhow!("--filter requires a pattern"));
                }
                filter = Some(args[i + 1].clone());
                i += 2;
            }
            other => {
                positional.push(other.to_string());
                i += 1;
            }
        }
    }

    let mut target: Option<String> = None;
    if let Some(first) = positional.first() {
        if recursive_base_path(first).is_some() {
            target = Some(first.clone());
            if positional.len() > 1 {
                if filter.is_none() {
                    filter = Some(positional[1].clone());
                }
                if positional.len() > 2 {
                    return Err(anyhow::anyhow!("Unexpected argument: {}", positional[2]));
                }
            }
        } else {
            let p = Path::new(first);
            if p.exists() || is_path_like(first) {
                target = Some(first.clone());
                if positional.len() > 1 {
                    if filter.is_none() {
                        filter = Some(positional[1].clone());
                    }
                    if positional.len() > 2 {
                        return Err(anyhow::anyhow!("Unexpected argument: {}", positional[2]));
                    }
                }
            } else if filter.is_none() {
                filter = Some(first.clone());
                if positional.len() > 1 {
                    return Err(anyhow::anyhow!("Unexpected argument: {}", positional[1]));
                }
            }
        }
    }

    if let Some(target) = target {
        if let Some(base) = recursive_base_path(&target) {
            let targets = collect_recursive_test_targets(&base)?;
            if targets.is_empty() {
                println!("No test targets found under {}", base.display());
                return Ok(1);
            }

            let mut failed = 0usize;
            let mut no_tests = 0usize;
            let mut errored = 0usize;
            for t in &targets {
                match t {
                    TestTarget::Project(p) => println!("\n==> Testing project: {}", p.display()),
                    TestTarget::File(p) => println!("\n==> Testing file: {}", p.display()),
                }
                match run_test_target(t, filter.as_deref(), verbose) {
                    Ok(code) => {
                        if code == 3 {
                            no_tests += 1;
                        } else if code != 0 {
                            failed += 1;
                        }
                    }
                    Err(e) => {
                        errored += 1;
                        eprintln!("[ERROR] {}", e);
                    }
                }
            }

            println!(
                "\nTargets: {} total, {} failed, {} errored",
                targets.len(),
                failed,
                errored
            );
            println!("Targets with no matching tests: {}", no_tests);

            if errored > 0 {
                return Ok(2);
            }
            return Ok(if failed > 0 { 1 } else { 0 });
        }

        let target_path = Path::new(&target);
        if target_path.is_dir() {
            let project_file = target_path.join("project.breom");
            if project_file.exists() {
                finalize_single_test_result(test_project(&project_file, filter.as_deref(), verbose))
            } else {
                let main_file = target_path.join("main_test.brm");
                if main_file.exists() {
                    finalize_single_test_result(test_file(&main_file, filter.as_deref(), verbose))
                } else {
                    Err(anyhow::anyhow!(
                        "No project.breom or main_test.brm found in {}",
                        target_path.display()
                    ))
                }
            }
        } else if target_path.extension().and_then(OsStr::to_str) == Some("brm") {
            if !target_path.exists() {
                Err(anyhow::anyhow!("File not found: {}", target_path.display()))
            } else if !is_test_source_file(target_path) {
                Err(anyhow::anyhow!(
                    "Test file must end with '_test.brm': {}",
                    target_path.display()
                ))
            } else {
                finalize_single_test_result(test_file(target_path, filter.as_deref(), verbose))
            }
        } else if is_path_like(&target) {
            Err(anyhow::anyhow!("Path not found: {}", target))
        } else {
            Err(anyhow::anyhow!("Invalid test target: {}", target))
        }
    } else {
        let project_file = Path::new("project.breom");
        if project_file.exists() {
            finalize_single_test_result(test_project(project_file, filter.as_deref(), verbose))
        } else {
            let main_file = Path::new("main_test.brm");
            if main_file.exists() {
                finalize_single_test_result(test_file(main_file, filter.as_deref(), verbose))
            } else {
                Err(anyhow::anyhow!("No project.breom or main_test.brm found"))
            }
        }
    }
}

fn cmd_build(args: &[String]) -> Result<(), anyhow::Error> {
    let project_file = if args.is_empty() {
        Path::new("project.breom").to_path_buf()
    } else {
        let path = Path::new(&args[0]);
        if path.is_dir() {
            path.join("project.breom")
        } else {
            path.to_path_buf()
        }
    };

    if !project_file.exists() {
        return Err(anyhow::anyhow!(
            "project.breom not found at {:?}",
            project_file
        ));
    }

    let project = Project::load(&project_file)?;
    println!("Building project: {}", project.name);

    let (source_files, entry_package) = collect_project_sources(&project, false)?;
    println!("Found {} source file(s)", source_files.len());

    let known_attributes = collect_declared_attributes(&source_files)?;
    for source in &source_files {
        validate_source_attributes(source, &known_attributes)?;
    }

    let graph = project::DependencyGraph::from_sources(&source_files);
    let sorted_packages = graph
        .topological_sort(&entry_package)
        .map_err(|e| anyhow::anyhow!("Dependency error: {}", e))?;

    let mut package_map: HashMap<String, Vec<&Program>> = HashMap::new();
    for source in &source_files {
        package_map
            .entry(source.package.clone())
            .or_default()
            .push(&source.program);
    }

    let mut programs = Vec::new();
    for package in sorted_packages {
        if let Some(pkg_programs) = package_map.get(&package) {
            for program in pkg_programs {
                programs.push(*program);
            }
        }
    }

    compile_and_run(programs, false, &entry_package)?;

    println!("Build successful!");
    Ok(())
}

fn run_project(project_file: &Path) -> Result<i64, anyhow::Error> {
    let project = Project::load(project_file)?;
    if !project.entrypoint.exists() {
        return Err(anyhow::anyhow!(
            "Entrypoint not found: {:?}",
            project.entrypoint
        ));
    }

    let (source_files, entry_package) = collect_project_sources(&project, false)?;

    let known_attributes = collect_declared_attributes(&source_files)?;
    for source in &source_files {
        validate_source_attributes(source, &known_attributes)?;
    }

    let graph = project::DependencyGraph::from_sources(&source_files);
    let sorted_packages = graph
        .topological_sort(&entry_package)
        .map_err(|e| anyhow::anyhow!("Dependency error: {}", e))?;

    let mut package_map: std::collections::HashMap<String, Vec<&Program>> =
        std::collections::HashMap::new();
    for source in &source_files {
        package_map
            .entry(source.package.clone())
            .or_default()
            .push(&source.program);
    }

    let mut programs = Vec::new();
    for package in sorted_packages {
        if let Some(pkg_programs) = package_map.get(&package) {
            programs.extend(pkg_programs);
        }
    }

    compile_and_run(programs, true, &entry_package)
}

fn test_project(
    project_file: &Path,
    pattern: Option<&str>,
    verbose: bool,
) -> Result<i64, anyhow::Error> {
    let project = Project::load(project_file)?;
    if !project.entrypoint.exists() {
        return Err(anyhow::anyhow!(
            "Entrypoint not found: {:?}",
            project.entrypoint
        ));
    }

    let (source_files, entry_package) = collect_project_sources(&project, true)?;

    let known_attributes = collect_declared_attributes(&source_files)?;
    for source in &source_files {
        validate_source_attributes(source, &known_attributes)?;
    }

    let graph = project::DependencyGraph::from_sources(&source_files);
    let sorted_packages = graph
        .topological_sort_all()
        .map_err(|e| anyhow::anyhow!("Dependency error: {}", e))?;

    let tests = collect_test_functions(&source_files)?;
    compile_and_test(
        &source_files,
        &sorted_packages,
        tests,
        &entry_package,
        pattern,
        verbose,
    )
}

fn collect_project_sources(
    project: &Project,
    include_test_files: bool,
) -> Result<(Vec<project::SourceFile>, String), anyhow::Error> {
    let local_source_paths = filter_source_paths(project.discover_sources()?, include_test_files);
    let resolved_dependencies = project.resolve_dependencies()?;

    let mut all_sources = Vec::new();
    let mut known_packages = HashSet::new();
    let mut entry_package = None;
    let entrypoint_canonical = project
        .entrypoint
        .canonicalize()
        .unwrap_or_else(|_| project.entrypoint.clone());

    for path in &local_source_paths {
        let source = project.load_source(path)?;
        let path_canonical = path.canonicalize().unwrap_or_else(|_| path.clone());
        if path_canonical == entrypoint_canonical {
            entry_package = Some(source.package.clone());
        }
        known_packages.insert(source.package.clone());
        all_sources.push(source);
    }

    for dep in &resolved_dependencies {
        let dep_source_paths = filter_source_paths(dep.source_paths.clone(), include_test_files);
        for path in dep_source_paths {
            let source = Project::load_source_for(&path, &dep.root_path, &dep.package)?;
            known_packages.insert(source.package.clone());
            all_sources.push(source);
        }
    }

    known_packages.insert(project.name.clone());

    for source in &all_sources {
        project.validate_source_imports(source, &known_packages)?;
    }

    let entry_package = entry_package.unwrap_or_else(|| project.name.clone());
    Ok((all_sources, entry_package))
}

fn run_file(file_path: &Path) -> Result<i64, anyhow::Error> {
    let project = Project::from_single_file(file_path)?;
    let source_paths = filter_source_paths(project.discover_sources()?, false);
    let known_packages = project.collect_known_packages(&source_paths);

    let mut sources = Vec::new();
    let mut entry_package = None;
    let entrypoint_canonical = project
        .entrypoint
        .canonicalize()
        .unwrap_or_else(|_| project.entrypoint.clone());

    for path in &source_paths {
        let source = project.load_source(path)?;
        project.validate_source_imports(&source, &known_packages)?;
        let path_canonical = path.canonicalize().unwrap_or_else(|_| path.clone());
        if path_canonical == entrypoint_canonical {
            entry_package = Some(source.package.clone());
        }
        sources.push(source);
    }

    let entry_package = entry_package.unwrap_or_else(|| project.name.clone());
    let entry_dir = file_path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
        .canonicalize()
        .unwrap_or_else(|_| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let sources: Vec<_> = sources
        .into_iter()
        .filter(|source| {
            if source.package != entry_package {
                return true;
            }
            let source_dir = source
                .path
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(Path::new("."))
                .canonicalize()
                .unwrap_or_else(|_| Path::new(".").to_path_buf());
            source_dir == entry_dir
        })
        .collect();

    let known_attributes = collect_declared_attributes(&sources)?;
    for source in &sources {
        validate_source_attributes(source, &known_attributes)?;
    }

    let graph = project::DependencyGraph::from_sources(&sources);
    let sorted_packages = graph
        .topological_sort_all()
        .map_err(|e| anyhow::anyhow!("Dependency error: {}", e))?;

    let mut package_map: std::collections::HashMap<String, Vec<&Program>> =
        std::collections::HashMap::new();
    for source in &sources {
        package_map
            .entry(source.package.clone())
            .or_default()
            .push(&source.program);
    }

    let mut programs = Vec::new();
    for package in sorted_packages {
        if let Some(pkg_programs) = package_map.get(&package) {
            programs.extend(pkg_programs);
        }
    }

    compile_and_run(programs, true, &entry_package)
}

fn test_file(file_path: &Path, pattern: Option<&str>, verbose: bool) -> Result<i64, anyhow::Error> {
    let project = Project::from_single_file(file_path)?;
    let source_paths = filter_source_paths(project.discover_sources()?, true);
    let known_packages = project.collect_known_packages(&source_paths);

    let mut sources = Vec::new();
    let mut entry_package = None;
    let entrypoint_canonical = project
        .entrypoint
        .canonicalize()
        .unwrap_or_else(|_| project.entrypoint.clone());

    for path in &source_paths {
        let source = project.load_source(path)?;
        project.validate_source_imports(&source, &known_packages)?;
        let path_canonical = path.canonicalize().unwrap_or_else(|_| path.clone());
        if path_canonical == entrypoint_canonical {
            entry_package = Some(source.package.clone());
        }
        sources.push(source);
    }

    let entry_package = entry_package.unwrap_or_else(|| project.name.clone());
    let entry_dir = file_path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
        .canonicalize()
        .unwrap_or_else(|_| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let sources: Vec<_> = sources
        .into_iter()
        .filter(|source| {
            if source.package != entry_package {
                return true;
            }
            let source_dir = source
                .path
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(Path::new("."))
                .canonicalize()
                .unwrap_or_else(|_| Path::new(".").to_path_buf());
            source_dir == entry_dir
        })
        .collect();

    let known_attributes = collect_declared_attributes(&sources)?;
    for source in &sources {
        validate_source_attributes(source, &known_attributes)?;
    }

    let graph = project::DependencyGraph::from_sources(&sources);
    let sorted_packages = graph
        .topological_sort(&entry_package)
        .map_err(|e| anyhow::anyhow!("Dependency error: {}", e))?;

    let tests = collect_test_functions(&sources)?;
    compile_and_test(
        &sources,
        &sorted_packages,
        tests,
        &entry_package,
        pattern,
        verbose,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("breom_{prefix}_{nanos}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn run_git(workdir: &Path, args: &[&str]) {
        let output = Command::new("git")
            .args(args)
            .current_dir(workdir)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn init_multi_tag_repo(repo_dir: &Path) {
        fs::create_dir_all(repo_dir).unwrap();
        fs::write(
            repo_dir.join("project.breom"),
            "breom 0.1.0\npackage depv1\nentrypoint main.brm\n",
        )
        .unwrap();
        fs::write(
            repo_dir.join("main.brm"),
            "pub fn value() Int { return 10 }\n",
        )
        .unwrap();

        run_git(repo_dir, &["init"]);
        run_git(repo_dir, &["add", "."]);
        run_git(
            repo_dir,
            &[
                "-c",
                "user.name=breom-test",
                "-c",
                "user.email=breom-test@example.com",
                "commit",
                "-m",
                "v1",
            ],
        );
        run_git(repo_dir, &["tag", "v0.1.0"]);

        fs::write(
            repo_dir.join("project.breom"),
            "breom 0.1.0\npackage depv2\nentrypoint main.brm\n",
        )
        .unwrap();
        fs::write(
            repo_dir.join("main.brm"),
            "pub fn value() Int { return 20 }\n",
        )
        .unwrap();
        run_git(repo_dir, &["add", "."]);
        run_git(
            repo_dir,
            &[
                "-c",
                "user.name=breom-test",
                "-c",
                "user.email=breom-test@example.com",
                "commit",
                "-m",
                "v2",
            ],
        );
        run_git(repo_dir, &["tag", "v0.2.0"]);
    }

    #[test]
    fn parse_source_accepts_valid_program() {
        let program = parse_source("fn main() Int { return 0 }").unwrap();
        assert_eq!(program.items.len(), 1);
    }

    #[test]
    fn parse_source_rejects_invalid_program() {
        let err = parse_source("fn main( {").unwrap_err();
        assert!(err.to_string().contains("Parse error"));
    }

    #[test]
    fn parse_source_rejects_zero_arg_attribute_parentheses() {
        let err = parse_source("@test() fn main() Int { return 0 }").unwrap_err();
        assert!(err.to_string().contains("Parse error"));
    }

    #[test]
    fn compile_and_run_executes_main_and_returns_value() {
        let program = parse_source(
            r#"
fn main() Int {
    value := 40
    return value + 2
}
"#,
        )
        .unwrap();

        let result = compile_and_run(vec![&program], true, "main").unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn compile_and_run_can_compile_without_execution() {
        let program = parse_source("fn main() Int { return 7 }").unwrap();
        let result = compile_and_run(vec![&program], false, "main").unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn struct_default_override_and_fallback_work() {
        let program = parse_source(
            r#"
struct User {
    id Int
    name String

    default() {
        return User { id: 7, name: "ok" }
    }
}

struct Boxed {
    value Int
}

fn main() Int {
    u := User.default()
    b := Boxed.default()
    if u.id != 7 {
        return 1
    }
    if b.value != 0 {
        return 2
    }
    return 0
}
"#,
        )
        .unwrap();

        let result = compile_and_run(vec![&program], true, "main").unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn question_operator_requires_error_result_expression() {
        let program = parse_source(
            r#"
fn id(x Int) Int {
    return x
}

fn main() Int {
    v := id(1)?
    return v
}
"#,
        )
        .unwrap();

        let err = compile_and_run(vec![&program], false, "main").unwrap_err();
        assert!(err
            .to_string()
            .contains("'?' can only be used on expressions returning Error result"));
    }

    #[test]
    fn function_generic_constraint_is_checked_at_call_site() {
        let program = parse_source(
            r#"
fn pickInt<T: Int>(x T) Int {
    return 1
}

fn main() Int {
    s := "hello"
    return pickInt(s)
}
"#,
        )
        .unwrap();

        let err = compile_and_run(vec![&program], false, "main").unwrap_err();
        assert!(err
            .to_string()
            .contains("does not satisfy constraint 'Int' for 'T'"));
    }

    #[test]
    fn wait_arm_with_heap_value_runs() {
        let program = parse_source(
            r#"
fn main() Int {
    ch := Channel<String>.new(1)
    ch << "hello"
    mut out String = ""
    wait {
        v := << ch => {
            out = v
        }
    }
    if out == "hello" {
        return 0
    }
    return 1
}
"#,
        )
        .unwrap();

        let result = compile_and_run(vec![&program], true, "main").unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn compile_and_test_includes_tests_from_non_entry_packages() {
        let mut main_program = parse_source("fn main() Int { return 0 }").unwrap();
        main_program.module = Some(ModuleDecl {
            path: ModulePath {
                segments: vec!["main".to_string()],
                span: Span { start: 0, end: 0 },
            },
            span: Span { start: 0, end: 0 },
        });

        let mut util_program = parse_source("@test fn util_test() { }").unwrap();
        util_program.module = Some(ModuleDecl {
            path: ModulePath {
                segments: vec!["util".to_string()],
                span: Span { start: 0, end: 0 },
            },
            span: Span { start: 0, end: 0 },
        });

        let source_files = vec![
            project::SourceFile {
                path: PathBuf::from("main.brm"),
                package: "main".to_string(),
                imports: vec![],
                program: main_program,
            },
            project::SourceFile {
                path: PathBuf::from("util/main_test.brm"),
                package: "util".to_string(),
                imports: vec![],
                program: util_program,
            },
        ];

        let tests = collect_test_functions(&source_files).unwrap();
        assert_eq!(tests.runtime_tests.len(), 1);

        let sorted_packages = vec!["main".to_string(), "util".to_string()];
        let result =
            compile_and_test(&source_files, &sorted_packages, tests, "main", None, false).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn run_project_loads_and_accesses_multiple_dep_versions() {
        let dep_repo = unique_temp_dir("dep_multi_tag_repo");
        init_multi_tag_repo(&dep_repo);

        let project_root = unique_temp_dir("consumer_multi_dep_versions");
        let repo_url = format!("file://{}", dep_repo.display());
        fs::write(
            project_root.join("project.breom"),
            format!(
                "breom 0.1.0\npackage app\nentrypoint main.brm\ndep \"{}\" \"v0.1.0\"\ndep \"{}\" \"v0.2.0\"\n",
                repo_url, repo_url
            ),
        )
        .unwrap();
        fs::write(
            project_root.join("main.brm"),
            "import depv1\nimport depv2\n\nfn main() Int { return depv1.value() + depv2.value() }\n",
        )
        .unwrap();

        let result = run_project(&project_root.join("project.breom")).unwrap();
        assert_eq!(result, 30);

        let lock_content = fs::read_to_string(project_root.join("lock.breom")).unwrap();
        let v1_count = lock_content.matches("\"v0.1.0\"").count();
        let v2_count = lock_content.matches("\"v0.2.0\"").count();
        assert_eq!(v1_count, 1);
        assert_eq!(v2_count, 1);

        let _ = fs::remove_dir_all(dep_repo);
        let _ = fs::remove_dir_all(project_root);
    }
}
