use anyhow::{anyhow, Result};
use pest::Parser;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Component;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::ast::common::Span;
use crate::ast::program::{ModuleDecl, ModulePath, Program};
use crate::{BreomParser, Rule};

#[derive(Debug)]
pub struct Project {
    #[allow(dead_code)]
    pub breom_version: Version,
    pub name: String,
    pub root_path: PathBuf,
    pub entrypoint: PathBuf,
    pub dependencies: Vec<Dependency>,
}

#[derive(Debug, Clone)]
pub struct Dependency {
    pub repo: String,
    pub tag: String,
}

#[derive(Debug, Clone)]
pub struct ResolvedDependency {
    pub root_path: PathBuf,
    pub package: String,
    pub source_paths: Vec<PathBuf>,
}

#[derive(Debug)]
pub struct SourceFile {
    pub path: PathBuf,
    pub package: String,
    pub imports: Vec<String>,
    pub program: Program,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LockDependency {
    repo: String,
    tag: String,
    commit: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct LockFile {
    dependencies: Vec<LockDependency>,
}

impl Project {
    const SUPPORTED_BREOM_VERSION: &str = env!("CARGO_PKG_VERSION");

    pub fn load(project_file: &Path) -> Result<Self> {
        let content = fs::read_to_string(project_file)
            .map_err(|e| anyhow!("Failed to read project.breom: {}", e))?;
        let (breom_version, name, entrypoint_rel, dependencies) =
            Self::parse_project_file(&content)?;

        if breom_version != Self::supported_breom_version()? {
            return Err(anyhow!(
                "Unsupported breom version '{}'. This compiler supports '{}'",
                breom_version,
                Self::SUPPORTED_BREOM_VERSION,
            ));
        }

        let root_path = project_file
            .parent()
            .map(|p| {
                if p.as_os_str().is_empty() {
                    Path::new(".")
                } else {
                    p
                }
            })
            .ok_or_else(|| anyhow!("Invalid project file path"))?
            .canonicalize()
            .map_err(|e| anyhow!("Failed to canonicalize project root: {}", e))?;
        let entrypoint = root_path.join(&entrypoint_rel);

        Ok(Project {
            breom_version,
            name,
            root_path,
            entrypoint,
            dependencies,
        })
    }

    pub fn from_single_file(file_path: &Path) -> Result<Self> {
        let root_path = match file_path.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
            _ => Path::new(".").to_path_buf(),
        };
        let root_path = root_path.canonicalize().unwrap_or(root_path);

        let name = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("main")
            .to_string();

        Ok(Project {
            breom_version: Self::supported_breom_version()?,
            name,
            root_path,
            entrypoint: file_path.to_path_buf(),
            dependencies: Vec::new(),
        })
    }

    pub fn parse_breom(content: &str) -> Result<Program> {
        let pairs = BreomParser::parse(Rule::program, content)
            .map_err(|e| anyhow!("Parse error: {}", e))?;

        let pair = pairs
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("No program found"))?;

        Ok(Program::from_pair(pair))
    }

    fn parse_project_file(content: &str) -> Result<(Version, String, String, Vec<Dependency>)> {
        Self::reject_project_import_declaration(content)?;

        let pairs = BreomParser::parse(Rule::project_file, content)
            .map_err(|e| anyhow!("Project parse error: {}", e))?;
        let pair = pairs
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("No project file found"))?;

        let mut breom_version = None;
        let mut name = None;
        let mut entrypoint = None;
        let mut dependencies = Vec::new();

        for inner in pair.into_inner() {
            match inner.as_rule() {
                Rule::module_decl => {
                    if let Some(path) = inner.into_inner().next() {
                        name = Some(path.as_str().to_string());
                    }
                }
                Rule::breom_decl => {
                    if let Some(version) = inner.into_inner().next() {
                        let parsed = Version::parse(version.as_str()).map_err(|e| {
                            anyhow!("Invalid breom version '{}': {}", version.as_str(), e)
                        })?;
                        breom_version = Some(parsed);
                    }
                }
                Rule::entrypoint_decl => {
                    if let Some(path) = inner.into_inner().next() {
                        entrypoint = Some(path.as_str().to_string());
                    }
                }
                Rule::dep_decl => {
                    let mut parts = inner.into_inner();
                    let repo_raw = parts
                        .next()
                        .ok_or_else(|| anyhow!("Invalid dep declaration: missing repo"))?;
                    let tag_raw = parts
                        .next()
                        .ok_or_else(|| anyhow!("Invalid dep declaration: missing tag"))?;
                    let repo = Self::parse_string_literal(repo_raw.as_str())?;
                    let tag = Self::parse_string_literal(tag_raw.as_str())?;
                    dependencies.push(Dependency { repo, tag });
                }
                Rule::EOI => {}
                _ => {}
            }
        }

        let breom_version =
            breom_version.ok_or_else(|| anyhow!("project.breom must declare 'breom <x.y.z>'"))?;
        let name = name.ok_or_else(|| anyhow!("project.breom must declare 'package <name>'"))?;
        let entrypoint = entrypoint.unwrap_or_else(|| "main.brm".to_string());
        Ok((breom_version, name, entrypoint, dependencies))
    }

    fn supported_breom_version() -> Result<Version> {
        Version::parse(Self::SUPPORTED_BREOM_VERSION).map_err(|e| {
            anyhow!(
                "Invalid compiler breom version '{}': {}",
                Self::SUPPORTED_BREOM_VERSION,
                e
            )
        })
    }

    fn reject_project_import_declaration(content: &str) -> Result<()> {
        for (line_no, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if !trimmed.starts_with("import") {
                continue;
            }

            return Err(anyhow!(
                "Invalid project.breom declaration at line {}: use `dep \"<repo>\" \"<tag>\"` instead of `import ...`",
                line_no + 1
            ));
        }

        Ok(())
    }

    fn parse_string_literal(raw: &str) -> Result<String> {
        serde_json::from_str::<String>(raw)
            .map_err(|e| anyhow!("Invalid quoted string '{}': {}", raw, e))
    }

    pub fn load_source(&self, path: &Path) -> Result<SourceFile> {
        Self::load_source_for(path, &self.root_path, &self.name)
    }

    pub fn load_source_for(
        path: &Path,
        root_path: &Path,
        root_package: &str,
    ) -> Result<SourceFile> {
        let content = fs::read_to_string(path)
            .map_err(|e| anyhow!("Failed to read {}: {}", path.display(), e))?;

        let mut program = Self::parse_breom(&content)?;
        let package = Self::infer_package_name_for(path, root_path, root_package);
        program.module = Some(Self::module_decl_from_package(&package));

        let imports: Vec<String> = program
            .depends
            .iter()
            .map(|d| d.path.segments.join("."))
            .collect();

        Ok(SourceFile {
            path: path.to_path_buf(),
            package,
            imports,
            program,
        })
    }

    pub fn collect_known_packages(&self, source_paths: &[PathBuf]) -> HashSet<String> {
        let mut known_packages: HashSet<String> = source_paths
            .iter()
            .map(|path| self.infer_package_name(path))
            .collect();

        known_packages.insert(self.name.clone());

        known_packages
    }

    pub fn validate_source_imports(
        &self,
        source: &SourceFile,
        known_packages: &HashSet<String>,
    ) -> Result<()> {
        for import in &source.imports {
            if !known_packages.contains(import) {
                return Err(anyhow!(
                    "Unknown import package '{}' in '{}'. Declare it in project.breom or add matching source/std package",
                    import,
                    source.path.display()
                ));
            }
        }

        Ok(())
    }

    fn module_decl_from_package(package: &str) -> ModuleDecl {
        ModuleDecl {
            path: ModulePath {
                segments: package.split('.').map(|s| s.to_string()).collect(),
                span: Span { start: 0, end: 0 },
            },
            span: Span { start: 0, end: 0 },
        }
    }

    fn infer_package_name(&self, path: &Path) -> String {
        Self::infer_package_name_for(path, &self.root_path, &self.name)
    }

    pub fn infer_package_name_for(path: &Path, root_path: &Path, root_package: &str) -> String {
        let canonical_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

        if let Some(std_pkg) = Self::package_from_std_path(&canonical_path) {
            return std_pkg;
        }

        if let Ok(rel) = canonical_path.strip_prefix(root_path) {
            let parent = rel.parent().unwrap_or_else(|| Path::new(""));
            if parent.as_os_str().is_empty() {
                return root_package.to_string();
            }
            return Self::path_to_package(parent).unwrap_or_else(|| root_package.to_string());
        }

        canonical_path
            .parent()
            .and_then(Self::path_to_package)
            .unwrap_or_else(|| root_package.to_string())
    }

    fn package_from_std_path(path: &Path) -> Option<String> {
        let normal_components: Vec<String> = path
            .components()
            .filter_map(|component| match component {
                Component::Normal(os) => Some(os.to_string_lossy().to_string()),
                _ => None,
            })
            .collect();

        let std_index = normal_components
            .iter()
            .position(|component| component == "std")?;
        if normal_components.len() <= std_index + 2 {
            return None;
        }

        let package_parts = &normal_components[(std_index + 1)..(normal_components.len() - 1)];
        if package_parts.is_empty() {
            return None;
        }

        Some(package_parts.join("."))
    }

    fn path_to_package(path: &Path) -> Option<String> {
        let parts: Vec<String> = path
            .components()
            .filter_map(|component| match component {
                Component::Normal(os) => {
                    let s = os.to_string_lossy().to_string();
                    if s.is_empty() {
                        None
                    } else {
                        Some(s)
                    }
                }
                _ => None,
            })
            .collect();

        if parts.is_empty() {
            None
        } else {
            Some(parts.join("."))
        }
    }

    pub fn discover_sources(&self) -> Result<Vec<PathBuf>> {
        let mut sources = Vec::new();
        self.discover_recursive(&self.root_path, &mut sources)?;

        let mut std_path = if self.root_path.is_absolute() {
            self.root_path.clone()
        } else {
            std::env::current_dir()?.join(&self.root_path)
        };

        loop {
            let potential_std = std_path.join("std");
            if potential_std.exists() && potential_std.is_dir() {
                self.discover_recursive(&potential_std, &mut sources)?;
                break;
            }
            if !std_path.pop() {
                break;
            }
        }

        Ok(sources)
    }

    fn discover_recursive(&self, dir: &Path, sources: &mut Vec<PathBuf>) -> Result<()> {
        if !dir.is_dir() {
            return Ok(());
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                self.discover_recursive(&path, sources)?;
            } else if path.extension().is_some_and(|e| e == "brm") {
                sources.push(path);
            }
        }

        Ok(())
    }

    pub fn lock_file_path(&self) -> PathBuf {
        self.root_path.join("lock.breom")
    }

    pub fn resolve_dependencies(&self) -> Result<Vec<ResolvedDependency>> {
        if self.dependencies.is_empty() {
            return Ok(Vec::new());
        }

        let lock_path = self.lock_file_path();
        let existing_lock = Self::load_lock_file(&lock_path)?;

        let mut lock_commits = HashMap::new();
        for dep in existing_lock.dependencies {
            lock_commits.insert((dep.repo, dep.tag), dep.commit);
        }

        let cache_root = Self::global_cache_root()?;
        fs::create_dir_all(&cache_root).map_err(|e| {
            anyhow!(
                "Failed to create cache directory {}: {}",
                cache_root.display(),
                e
            )
        })?;

        let mut resolved = Vec::new();
        let mut lock_dependencies = Vec::new();

        for dep in &self.dependencies {
            let checkout_path = Self::dependency_checkout_path(&cache_root, &dep.repo, &dep.tag);
            let lock_key = (dep.repo.clone(), dep.tag.clone());
            let locked_commit = lock_commits.get(&lock_key).cloned();
            let commit =
                Self::materialize_dependency(dep, &checkout_path, locked_commit.as_deref())?;

            let dep_project_file = checkout_path.join("project.breom");
            if !dep_project_file.exists() {
                return Err(anyhow!(
                    "Dependency '{}'@'{}' does not contain project.breom at {}",
                    dep.repo,
                    dep.tag,
                    dep_project_file.display()
                ));
            }

            let dep_project = Self::load(&dep_project_file)?;
            let source_paths = dep_project.discover_sources()?;
            resolved.push(ResolvedDependency {
                root_path: dep_project.root_path.clone(),
                package: dep_project.name.clone(),
                source_paths,
            });

            lock_dependencies.push(LockDependency {
                repo: dep.repo.clone(),
                tag: dep.tag.clone(),
                commit,
            });
        }

        lock_dependencies.sort_by(|a, b| (&a.repo, &a.tag).cmp(&(&b.repo, &b.tag)));
        Self::write_lock_file(
            &lock_path,
            &LockFile {
                dependencies: lock_dependencies,
            },
        )?;

        Ok(resolved)
    }

    fn load_lock_file(lock_path: &Path) -> Result<LockFile> {
        if !lock_path.exists() {
            return Ok(LockFile::default());
        }

        let content = fs::read_to_string(lock_path)
            .map_err(|e| anyhow!("Failed to read {}: {}", lock_path.display(), e))?;
        serde_json::from_str::<LockFile>(&content)
            .map_err(|e| anyhow!("Failed to parse {}: {}", lock_path.display(), e))
    }

    fn write_lock_file(lock_path: &Path, lock_file: &LockFile) -> Result<()> {
        let content = serde_json::to_string_pretty(lock_file)
            .map_err(|e| anyhow!("Failed to encode lock file: {}", e))?;
        fs::write(lock_path, content)
            .map_err(|e| anyhow!("Failed to write {}: {}", lock_path.display(), e))
    }

    fn global_cache_root() -> Result<PathBuf> {
        let home = std::env::var("HOME")
            .map_err(|_| anyhow!("HOME is not set; cannot determine dependency cache path"))?;
        Ok(PathBuf::from(home).join(".breom").join("pkg"))
    }

    fn dependency_checkout_path(cache_root: &Path, repo: &str, tag: &str) -> PathBuf {
        cache_root
            .join(Self::sanitize_path_component(repo))
            .join(Self::sanitize_path_component(tag))
    }

    fn sanitize_path_component(value: &str) -> String {
        value
            .chars()
            .map(|ch| {
                if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                    ch
                } else {
                    '_'
                }
            })
            .collect()
    }

    fn materialize_dependency(
        dep: &Dependency,
        checkout_path: &Path,
        locked_commit: Option<&str>,
    ) -> Result<String> {
        let clone_url = Self::repo_clone_url(&dep.repo);

        if !checkout_path.join(".git").exists() {
            let parent = checkout_path.parent().ok_or_else(|| {
                anyhow!(
                    "Invalid dependency checkout path: {}",
                    checkout_path.display()
                )
            })?;
            fs::create_dir_all(parent)
                .map_err(|e| anyhow!("Failed to create {}: {}", parent.display(), e))?;

            let destination = checkout_path.to_string_lossy().to_string();
            if locked_commit.is_none() {
                Self::run_git(
                    None,
                    &[
                        "clone",
                        "--branch",
                        dep.tag.as_str(),
                        "--depth",
                        "1",
                        clone_url.as_str(),
                        destination.as_str(),
                    ],
                )?;
            } else {
                Self::run_git(None, &["clone", clone_url.as_str(), destination.as_str()])?;
            }
        }

        if let Some(expected_commit) = locked_commit {
            Self::run_git(Some(checkout_path), &["fetch", "--tags", "--force"])?;
            Self::run_git(
                Some(checkout_path),
                &["checkout", "--detach", expected_commit],
            )?;
            let current_commit = Self::git_stdout(Some(checkout_path), &["rev-parse", "HEAD"])?;
            if current_commit != expected_commit {
                return Err(anyhow!(
                    "Dependency '{}'@'{}' lock mismatch: expected commit {}, got {}",
                    dep.repo,
                    dep.tag,
                    expected_commit,
                    current_commit
                ));
            }
            return Ok(current_commit);
        }

        Self::run_git(Some(checkout_path), &["fetch", "--tags", "--force"])?;
        let target_ref = format!("refs/tags/{}", dep.tag);
        let commit = Self::git_stdout(Some(checkout_path), &["rev-list", "-n", "1", &target_ref])?;
        if commit.is_empty() {
            return Err(anyhow!(
                "Dependency '{}' does not have tag '{}'",
                dep.repo,
                dep.tag
            ));
        }
        Self::run_git(Some(checkout_path), &["checkout", "--detach", &commit])?;
        Ok(commit)
    }

    fn repo_clone_url(repo: &str) -> String {
        if repo.starts_with("git@") || repo.contains("://") {
            return repo.to_string();
        }
        let trimmed = repo.trim_end_matches(".git");
        format!("https://{}.git", trimmed)
    }

    fn git_stdout(workdir: Option<&Path>, args: &[&str]) -> Result<String> {
        let output = Command::new("git")
            .args(args)
            .current_dir(workdir.unwrap_or_else(|| Path::new(".")))
            .output()
            .map_err(|e| anyhow!("Failed to run git {}: {}", args.join(" "), e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let cwd = workdir.map(|p| p.display().to_string()).unwrap_or_else(|| {
                std::env::current_dir()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| ".".to_string())
            });
            return Err(anyhow!(
                "git {} failed in {}: {}",
                args.join(" "),
                cwd,
                stderr
            ));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn run_git(workdir: Option<&Path>, args: &[&str]) -> Result<()> {
        Self::git_stdout(workdir, args).map(|_| ())
    }
}

#[derive(Debug)]
pub struct DependencyGraph {
    edges: HashMap<String, HashSet<String>>,
}

impl DependencyGraph {
    pub fn new() -> Self {
        DependencyGraph {
            edges: HashMap::new(),
        }
    }

    pub fn from_sources(sources: &[SourceFile]) -> Self {
        let mut graph = Self::new();
        let source_packages: HashSet<String> = sources
            .iter()
            .map(|source| source.package.clone())
            .collect();

        for source in sources {
            let deps = graph.edges.entry(source.package.clone()).or_default();
            for import in &source.imports {
                if source_packages.contains(import) {
                    deps.insert(import.clone());
                }
            }
        }

        graph
    }

    pub fn topological_sort(&self, entry_package: &str) -> Result<Vec<String>> {
        let mut result = Vec::new();
        let mut visited = HashSet::new();
        let mut temp_visited = HashSet::new();

        if self.edges.contains_key(entry_package) {
            self.visit(entry_package, &mut visited, &mut temp_visited, &mut result)?;
        } else {
            result.push(entry_package.to_string());
        }

        Ok(result)
    }

    pub fn topological_sort_all(&self) -> Result<Vec<String>> {
        let mut result = Vec::new();
        let mut visited = HashSet::new();
        let mut temp_visited = HashSet::new();

        let mut packages: Vec<String> = self.edges.keys().cloned().collect();
        packages.sort();

        for package in packages {
            self.visit(&package, &mut visited, &mut temp_visited, &mut result)?;
        }

        Ok(result)
    }

    fn visit(
        &self,
        node: &str,
        visited: &mut HashSet<String>,
        temp_visited: &mut HashSet<String>,
        result: &mut Vec<String>,
    ) -> Result<()> {
        if temp_visited.contains(node) {
            return Err(anyhow!("Circular dependency detected: {}", node));
        }

        if visited.contains(node) {
            return Ok(());
        }

        temp_visited.insert(node.to_string());

        if let Some(deps) = self.edges.get(node) {
            for dep in deps {
                self.visit(dep, visited, temp_visited, result)?;
            }
        }

        temp_visited.remove(node);
        visited.insert(node.to_string());
        result.push(node.to_string());

        Ok(())
    }
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

    fn dummy_program() -> Program {
        Program {
            module: None,
            entrypoint: None,
            depends: vec![],
            items: vec![],
            span: Span { start: 0, end: 0 },
        }
    }

    fn test_breom_version() -> Version {
        Version::parse("0.1.0").unwrap()
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

    fn init_dep_repo_with_tag(repo_dir: &Path, tag: &str) -> String {
        fs::create_dir_all(repo_dir).unwrap();
        fs::write(
            repo_dir.join("project.breom"),
            "breom 0.1.0\npackage depdemo\nentrypoint main.brm\n",
        )
        .unwrap();
        fs::write(
            repo_dir.join("main.brm"),
            "pub fn answer() Int { return 42 }\n",
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
                "initial",
            ],
        );
        run_git(repo_dir, &["tag", tag]);

        let output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(repo_dir)
            .output()
            .unwrap();
        assert!(output.status.success());
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    #[test]
    fn parse_project_file_reads_name_entrypoint_and_deps() {
        let content = r#"
breom 0.1.0
package app.core
entrypoint src/main.brm
dep "github.com/acme/http" "v1.2.3"
dep "github.com/acme/json" "v0.9.1"
"#;

        let (breom_version, name, entrypoint, deps) = Project::parse_project_file(content).unwrap();
        assert_eq!(breom_version, Version::parse("0.1.0").unwrap());
        assert_eq!(name, "app.core");
        assert_eq!(entrypoint, "src/main.brm");
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].repo, "github.com/acme/http");
        assert_eq!(deps[0].tag, "v1.2.3");
        assert_eq!(deps[1].repo, "github.com/acme/json");
        assert_eq!(deps[1].tag, "v0.9.1");
    }

    #[test]
    fn parse_project_file_rejects_import_declaration() {
        let content = r#"
breom 0.1.0
package app.core
import std.http
"#;

        let err = Project::parse_project_file(content).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("use `dep"));
    }

    #[test]
    fn parse_project_file_uses_default_entrypoint() {
        let content = "breom 0.1.0\npackage hello.world";
        let (_, _, entrypoint, deps) = Project::parse_project_file(content).unwrap();
        assert_eq!(entrypoint, "main.brm");
        assert!(deps.is_empty());
    }

    #[test]
    fn parse_project_file_requires_breom_version() {
        let content = "package hello.world";
        let err = Project::parse_project_file(content).unwrap_err();
        assert!(err.to_string().contains("Project parse error"));
    }

    #[test]
    fn infer_package_name_from_root_and_subdir() {
        let root = unique_temp_dir("infer_pkg");
        let src_dir = root.join("foo").join("bar");
        fs::create_dir_all(&src_dir).unwrap();
        let root_file = root.join("main.brm");
        let nested_file = src_dir.join("util.brm");
        fs::write(&root_file, "fn main() Int { return 0 }").unwrap();
        fs::write(&nested_file, "fn util() Int { return 0 }").unwrap();

        let project = Project {
            breom_version: test_breom_version(),
            name: "rootpkg".to_string(),
            root_path: root.canonicalize().unwrap(),
            entrypoint: root_file.clone(),
            dependencies: vec![],
        };

        assert_eq!(project.infer_package_name(&root_file), "rootpkg");
        assert_eq!(project.infer_package_name(&nested_file), "foo.bar");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn package_from_std_path_extracts_namespace() {
        let path = Path::new("/tmp/somewhere/std/http/models/request.brm");
        assert_eq!(
            Project::package_from_std_path(path).as_deref(),
            Some("http.models")
        );
    }

    #[test]
    fn discover_sources_finds_project_and_std_files() {
        let base = unique_temp_dir("discover_sources");
        let project_root = base.join("project");
        let std_root = base.join("std").join("builtin");
        fs::create_dir_all(&project_root).unwrap();
        fs::create_dir_all(&std_root).unwrap();

        let main_file = project_root.join("main.brm");
        let helper_file = project_root.join("lib.brm");
        let std_file = std_root.join("print.brm");
        fs::write(&main_file, "fn main() Int { return 0 }").unwrap();
        fs::write(&helper_file, "fn helper() Int { return 1 }").unwrap();
        fs::write(&std_file, "fn print(v Int) {}").unwrap();

        let project = Project {
            breom_version: test_breom_version(),
            name: "demo".to_string(),
            root_path: project_root,
            entrypoint: main_file,
            dependencies: vec![],
        };

        let mut sources = project.discover_sources().unwrap();
        sources.sort();
        assert_eq!(sources.len(), 3);
        assert!(sources.iter().any(|p| p.ends_with("main.brm")));
        assert!(sources.iter().any(|p| p.ends_with("lib.brm")));
        assert!(sources.iter().any(|p| p.ends_with("print.brm")));

        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn load_source_infers_package_and_collects_imports() {
        let root = unique_temp_dir("load_source");
        let nested = root.join("pkg").join("sub");
        fs::create_dir_all(&nested).unwrap();
        let file = nested.join("main.brm");
        fs::write(
            &file,
            r#"
import std.http as http
import foo.bar

fn main() Int { return 0 }
"#,
        )
        .unwrap();

        let project = Project {
            breom_version: test_breom_version(),
            name: "demo".to_string(),
            root_path: root.canonicalize().unwrap(),
            entrypoint: file.clone(),
            dependencies: vec![],
        };
        let source = project.load_source(&file).unwrap();
        assert_eq!(source.package, "pkg.sub");
        assert_eq!(source.imports, vec!["std.http", "foo.bar"]);
        assert_eq!(
            source.program.module.unwrap().path.segments,
            vec!["pkg".to_string(), "sub".to_string()]
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn dependency_graph_topological_sort_orders_dependencies_first() {
        let sources = vec![
            SourceFile {
                path: PathBuf::from("a.brm"),
                package: "app".to_string(),
                imports: vec!["util".to_string()],
                program: dummy_program(),
            },
            SourceFile {
                path: PathBuf::from("b.brm"),
                package: "util".to_string(),
                imports: vec!["core".to_string()],
                program: dummy_program(),
            },
            SourceFile {
                path: PathBuf::from("c.brm"),
                package: "core".to_string(),
                imports: vec![],
                program: dummy_program(),
            },
        ];
        let graph = DependencyGraph::from_sources(&sources);
        let sorted = graph.topological_sort("app").unwrap();
        assert_eq!(sorted, vec!["core", "util", "app"]);
    }

    #[test]
    fn dependency_graph_detects_cycles() {
        let sources = vec![
            SourceFile {
                path: PathBuf::from("a.brm"),
                package: "a".to_string(),
                imports: vec!["b".to_string()],
                program: dummy_program(),
            },
            SourceFile {
                path: PathBuf::from("b.brm"),
                package: "b".to_string(),
                imports: vec!["a".to_string()],
                program: dummy_program(),
            },
        ];

        let graph = DependencyGraph::from_sources(&sources);
        let err = graph.topological_sort("a").unwrap_err();
        assert!(err.to_string().contains("Circular dependency"));
    }

    #[test]
    fn dependency_graph_topological_sort_all_includes_disconnected_packages() {
        let sources = vec![
            SourceFile {
                path: PathBuf::from("a.brm"),
                package: "app".to_string(),
                imports: vec!["util".to_string()],
                program: dummy_program(),
            },
            SourceFile {
                path: PathBuf::from("b.brm"),
                package: "util".to_string(),
                imports: vec![],
                program: dummy_program(),
            },
            SourceFile {
                path: PathBuf::from("c.brm"),
                package: "other".to_string(),
                imports: vec![],
                program: dummy_program(),
            },
        ];

        let graph = DependencyGraph::from_sources(&sources);
        let sorted = graph.topological_sort_all().unwrap();
        assert!(sorted.contains(&"app".to_string()));
        assert!(sorted.contains(&"util".to_string()));
        assert!(sorted.contains(&"other".to_string()));
        assert!(
            sorted.iter().position(|p| p == "util").unwrap()
                < sorted.iter().position(|p| p == "app").unwrap()
        );
    }

    #[test]
    fn validate_import_accepts_project_and_std_packages() {
        let base = unique_temp_dir("validate_import_project_std");
        let project_root = base.join("project");
        let util_dir = project_root.join("util");
        let std_dir = base.join("std").join("http").join("models");
        fs::create_dir_all(&util_dir).unwrap();
        fs::create_dir_all(&std_dir).unwrap();

        let main_file = project_root.join("main.brm");
        let util_file = util_dir.join("helper.brm");
        let std_file = std_dir.join("request.brm");

        fs::write(
            &main_file,
            r#"
import util
import http.models

fn main() Int { return 0 }
"#,
        )
        .unwrap();
        fs::write(&util_file, "fn helper() Int { return 0 }").unwrap();
        fs::write(&std_file, "fn request() Int { return 0 }").unwrap();

        let project = Project {
            breom_version: test_breom_version(),
            name: "demo".to_string(),
            root_path: project_root.canonicalize().unwrap(),
            entrypoint: main_file.clone(),
            dependencies: vec![],
        };

        let source = project.load_source(&main_file).unwrap();
        let known = project.collect_known_packages(&[main_file, util_file, std_file]);
        project.validate_source_imports(&source, &known).unwrap();

        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn parse_project_file_requires_dep_repo_and_tag() {
        let content = r#"
breom 0.1.0
package app.core
dep "github.com/acme/http"
"#;

        let err = Project::parse_project_file(content).unwrap_err();
        assert!(err.to_string().contains("Project parse error"));
    }

    #[test]
    fn validate_import_rejects_unknown_package() {
        let root = unique_temp_dir("validate_import_unknown");
        let main_file = root.join("main.brm");
        fs::write(
            &main_file,
            r#"
import missing.pkg

fn main() Int { return 0 }
"#,
        )
        .unwrap();

        let project = Project {
            breom_version: test_breom_version(),
            name: "demo".to_string(),
            root_path: root.canonicalize().unwrap(),
            entrypoint: main_file.clone(),
            dependencies: vec![],
        };

        let source = project.load_source(&main_file).unwrap();
        let known = project.collect_known_packages(&[main_file]);
        let err = project
            .validate_source_imports(&source, &known)
            .unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("Unknown import package 'missing.pkg'"));
        assert!(msg.contains("main.brm"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn collect_known_packages_includes_root_package() {
        let root = unique_temp_dir("known_packages_deps");
        let project = Project {
            breom_version: test_breom_version(),
            name: "demo".to_string(),
            root_path: root.canonicalize().unwrap_or(root.clone()),
            entrypoint: root.join("main.brm"),
            dependencies: vec![],
        };

        let known = project.collect_known_packages(&[]);
        assert!(known.contains("demo"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn resolve_dependencies_from_local_tag_writes_lock_file() {
        let dep_repo = unique_temp_dir("dep_repo");
        let expected_commit = init_dep_repo_with_tag(&dep_repo, "v0.1.0");

        let consumer_root = unique_temp_dir("dep_consumer");
        let dep_repo_url = format!("file://{}", dep_repo.display());
        let content = format!(
            "breom 0.1.0\npackage app\nentrypoint main.brm\ndep \"{}\" \"v0.1.0\"\n",
            dep_repo_url
        );
        fs::write(consumer_root.join("project.breom"), content).unwrap();
        fs::write(
            consumer_root.join("main.brm"),
            "fn main() Int { return 0 }\n",
        )
        .unwrap();

        let project = Project::load(&consumer_root.join("project.breom")).unwrap();
        let resolved = project.resolve_dependencies().unwrap();

        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].package, "depdemo");
        assert!(!resolved[0].source_paths.is_empty());

        let lock_file = Project::load_lock_file(&consumer_root.join("lock.breom")).unwrap();
        assert_eq!(lock_file.dependencies.len(), 1);
        assert_eq!(lock_file.dependencies[0].repo, dep_repo_url);
        assert_eq!(lock_file.dependencies[0].tag, "v0.1.0");
        assert_eq!(lock_file.dependencies[0].commit, expected_commit);

        let _ = fs::remove_dir_all(dep_repo);
        let _ = fs::remove_dir_all(consumer_root);
    }
}
