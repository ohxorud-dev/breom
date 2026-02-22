use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, InitializeParams, Location, SymbolInformation, SymbolKind,
    Url,
};

use crate::ast::program::TopLevelItem;
use crate::lsp::analysis::ast_span_to_range;
use crate::lsp::references::ReferenceFinder;
use crate::project::Project;

const CURRENT_BREOM_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StdSource {
    BreomHomeSrc,
    MissingBreomHome,
}

#[derive(Debug, Clone)]
pub struct WorkspaceSnapshot {
    pub known_packages: HashSet<String>,
    pub std_packages: HashSet<String>,
    pub std_source: StdSource,
}

#[derive(Debug, Default)]
pub struct WorkspaceIndex {
    configured_roots: Vec<PathBuf>,
    cache: HashMap<PathBuf, WorkspaceSnapshot>,
    symbols: HashMap<PathBuf, CachedSymbols>,
    references: HashMap<(PathBuf, String), CachedReferences>,
}

#[derive(Debug, Clone)]
struct GlobalSymbol {
    name: String,
    package: String,
    kind: SymbolKind,
    location: Location,
}

#[derive(Debug, Clone)]
struct CachedSymbols {
    revision: u64,
    items: Vec<GlobalSymbol>,
}

#[derive(Debug, Clone)]
struct CachedReferences {
    revision: u64,
    locations: Vec<Location>,
}

impl WorkspaceIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn configure_from_initialize(&mut self, params: &InitializeParams) {
        self.configured_roots.clear();

        if let Some(root_uri) = &params.root_uri {
            if let Ok(path) = root_uri.to_file_path() {
                self.configured_roots.push(path);
            }
        }

        if let Some(folders) = &params.workspace_folders {
            for folder in folders {
                if let Ok(path) = folder.uri.to_file_path() {
                    self.configured_roots.push(path);
                }
            }
        }

        self.configured_roots.sort();
        self.configured_roots.dedup();
        self.cache.clear();
        self.symbols.clear();
        self.references.clear();
    }

    pub fn snapshot_for_uri(&mut self, uri: &Url) -> Option<WorkspaceSnapshot> {
        let path = uri.to_file_path().ok()?;
        let root = self.find_root_for_path(&path);

        if let Some(snapshot) = self.cache.get(&root) {
            return Some(snapshot.clone());
        }

        let snapshot = self.build_snapshot(&root);
        self.cache.insert(root.clone(), snapshot.clone());
        Some(snapshot)
    }

    fn find_root_for_path(&self, file_path: &Path) -> PathBuf {
        let mut best: Option<PathBuf> = None;

        for root in &self.configured_roots {
            if file_path.starts_with(root) {
                let better = best
                    .as_ref()
                    .map(|current| root.components().count() > current.components().count())
                    .unwrap_or(true);
                if better {
                    best = Some(root.clone());
                }
            }
        }

        if let Some(root) = best {
            return root;
        }

        let mut current = file_path.parent();
        while let Some(dir) = current {
            if dir.join("project.breom").exists() {
                return dir.to_path_buf();
            }
            current = dir.parent();
        }

        file_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
    }

    fn build_snapshot(&self, root: &Path) -> WorkspaceSnapshot {
        let mut known_packages = collect_project_packages(root);
        let (std_packages, std_source) = resolve_std_packages(root);

        for pkg in &std_packages {
            known_packages.insert(pkg.clone());
        }

        WorkspaceSnapshot {
            known_packages,
            std_packages,
            std_source,
        }
    }

    pub fn find_global_definitions(&mut self, uri: &Url, name: &str) -> Vec<Location> {
        let Ok(path) = uri.to_file_path() else {
            return Vec::new();
        };
        let root = self.find_root_for_path(&path);
        self.symbols_for_root(&root)
            .into_iter()
            .filter(|symbol| symbol.name == name)
            .map(|symbol| symbol.location)
            .collect()
    }

    pub fn definition_packages(&mut self, uri: &Url, name: &str) -> HashSet<String> {
        let Ok(path) = uri.to_file_path() else {
            return HashSet::new();
        };
        let root = self.find_root_for_path(&path);
        self.symbols_for_root(&root)
            .into_iter()
            .filter(|symbol| symbol.name == name)
            .map(|symbol| symbol.package)
            .collect()
    }

    pub fn query_workspace_symbols(&mut self, uri: &Url, query: &str) -> Vec<SymbolInformation> {
        let Ok(path) = uri.to_file_path() else {
            return Vec::new();
        };
        let root = self.find_root_for_path(&path);
        let q = query.trim().to_ascii_lowercase();

        #[allow(deprecated)]
        self.symbols_for_root(&root)
            .into_iter()
            .filter(|symbol| {
                if q.is_empty() {
                    true
                } else {
                    symbol.name.to_ascii_lowercase().contains(&q)
                        || symbol.package.to_ascii_lowercase().contains(&q)
                }
            })
            .map(|symbol| SymbolInformation {
                name: symbol.name,
                kind: symbol.kind,
                tags: None,
                deprecated: None,
                location: symbol.location,
                container_name: Some(symbol.package),
            })
            .collect()
    }

    pub fn find_global_references(
        &mut self,
        uri: &Url,
        name: &str,
        target_packages: Option<&HashSet<String>>,
    ) -> Vec<Location> {
        let Ok(path) = uri.to_file_path() else {
            return Vec::new();
        };
        let root = self.find_root_for_path(&path);
        let package_key = package_set_key(target_packages);
        let cache_key = (root.clone(), format!("{name}::{package_key}"));
        let files = collect_workspace_brm_files(&root);
        let revision = workspace_revision(&files);
        let root_package = read_root_package_name(&root).or_else(|| {
            root.file_name()
                .and_then(|n| n.to_str())
                .map(str::to_string)
        });

        if let Some(cached) = self.references.get(&cache_key) {
            if cached.revision == revision {
                return cached.locations.clone();
            }
        }

        let mut locations = Vec::new();

        for file in files {
            let Ok(content) = fs::read_to_string(&file) else {
                continue;
            };
            let Ok(program) = Project::parse_breom(&content) else {
                continue;
            };
            if let Some(targets) = target_packages {
                let package = package_for_workspace_file(&root, &file, root_package.as_deref())
                    .unwrap_or_default();
                if !file_can_reference_targets(&program, &package, targets) {
                    continue;
                }
            }
            let Some(file_uri) = Url::from_file_path(&file).ok() else {
                continue;
            };

            let finder = ReferenceFinder::new(&content, &program);
            let mut refs = finder.find_references_by_name(name);
            for location in &mut refs {
                location.uri = file_uri.clone();
            }
            locations.extend(refs);
        }

        self.references.insert(
            cache_key,
            CachedReferences {
                revision,
                locations: locations.clone(),
            },
        );

        locations
    }

    fn symbols_for_root(&mut self, root: &Path) -> Vec<GlobalSymbol> {
        let files = collect_workspace_brm_files(root);
        let revision = workspace_revision(&files);

        if let Some(cached) = self.symbols.get(root) {
            if cached.revision == revision {
                return cached.items.clone();
            }
        }

        let symbols = build_workspace_symbols(root, files);
        self.symbols.insert(
            root.to_path_buf(),
            CachedSymbols {
                revision,
                items: symbols.clone(),
            },
        );
        self.references
            .retain(|(cached_root, _), _| cached_root != root);
        symbols
    }
}

fn build_workspace_symbols(root: &Path, files: Vec<PathBuf>) -> Vec<GlobalSymbol> {
    let root_package = read_root_package_name(root).or_else(|| {
        root.file_name()
            .and_then(|n| n.to_str())
            .map(str::to_string)
    });

    let mut symbols = Vec::new();
    for file in files {
        let Ok(content) = fs::read_to_string(&file) else {
            continue;
        };
        let Ok(program) = Project::parse_breom(&content) else {
            continue;
        };
        let Some(uri) = Url::from_file_path(&file).ok() else {
            continue;
        };
        let package = package_for_workspace_file(root, &file, root_package.as_deref())
            .unwrap_or_else(|| "unknown".to_string());

        for item in program.items {
            match item {
                TopLevelItem::Function(f) => symbols.push(GlobalSymbol {
                    name: f.name,
                    package: package.clone(),
                    kind: SymbolKind::FUNCTION,
                    location: Location {
                        uri: uri.clone(),
                        range: ast_span_to_range(&content, &f.span),
                    },
                }),
                TopLevelItem::Struct(s) => symbols.push(GlobalSymbol {
                    name: s.name,
                    package: package.clone(),
                    kind: SymbolKind::STRUCT,
                    location: Location {
                        uri: uri.clone(),
                        range: ast_span_to_range(&content, &s.span),
                    },
                }),
                TopLevelItem::Interface(i) => symbols.push(GlobalSymbol {
                    name: i.name,
                    package: package.clone(),
                    kind: SymbolKind::INTERFACE,
                    location: Location {
                        uri: uri.clone(),
                        range: ast_span_to_range(&content, &i.span),
                    },
                }),
                TopLevelItem::Enum(e) => symbols.push(GlobalSymbol {
                    name: e.name,
                    package: package.clone(),
                    kind: SymbolKind::ENUM,
                    location: Location {
                        uri: uri.clone(),
                        range: ast_span_to_range(&content, &e.span),
                    },
                }),
                TopLevelItem::Define(d) => symbols.push(GlobalSymbol {
                    name: d.name,
                    package: package.clone(),
                    kind: SymbolKind::CONSTANT,
                    location: Location {
                        uri: uri.clone(),
                        range: ast_span_to_range(&content, &d.span),
                    },
                }),
                _ => {}
            }
        }
    }

    symbols
}

fn package_for_workspace_file(
    root: &Path,
    file: &Path,
    root_package: Option<&str>,
) -> Option<String> {
    let relative = file.strip_prefix(root).ok()?;
    let parent = relative.parent().unwrap_or_else(|| Path::new(""));
    let parts = path_components(parent);

    if parts.first().map(|s| s.as_str()) == Some("std") {
        if parts.len() > 1 {
            return Some(parts[1..].join("."));
        }
        return Some("builtin".to_string());
    }

    if parts.is_empty() {
        return root_package.map(str::to_string);
    }

    Some(parts.join("."))
}

fn collect_workspace_brm_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    collect_workspace_brm_recursive(root, &mut out);
    out.sort();
    out
}

fn workspace_revision(files: &[PathBuf]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for file in files {
        file.hash(&mut hasher);
        if let Ok(meta) = fs::metadata(file) {
            meta.len().hash(&mut hasher);
            if let Ok(modified) = meta.modified() {
                if let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH) {
                    duration.as_secs().hash(&mut hasher);
                    duration.subsec_nanos().hash(&mut hasher);
                }
            }
        }
    }
    hasher.finish()
}

fn package_set_key(target_packages: Option<&HashSet<String>>) -> String {
    let Some(targets) = target_packages else {
        return "*".to_string();
    };
    let mut packages = targets.iter().cloned().collect::<Vec<_>>();
    packages.sort();
    packages.join("|")
}

fn file_can_reference_targets(
    program: &crate::ast::program::Program,
    file_package: &str,
    targets: &HashSet<String>,
) -> bool {
    if targets.contains(file_package) {
        return true;
    }

    for dep in &program.depends {
        let imported = dep.path.segments.join(".");
        if targets.contains(&imported) {
            return true;
        }
    }

    false
}

fn collect_workspace_brm_recursive(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if is_ignored_dir(&path) {
                continue;
            }
            collect_workspace_brm_recursive(&path, out);
            continue;
        }

        if path.extension().and_then(|e| e.to_str()) == Some("brm") {
            out.push(path);
        }
    }
}

fn is_ignored_dir(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|s| s.to_str()),
        Some(".git") | Some("target") | Some("node_modules")
    )
}

fn collect_project_packages(root: &Path) -> HashSet<String> {
    let mut packages = HashSet::new();
    let root_package = read_root_package_name(root).or_else(|| {
        root.file_name()
            .and_then(|n| n.to_str())
            .map(str::to_string)
    });

    collect_brm_packages_recursive(root, root, root_package.as_deref(), false, &mut packages);

    if let Some(root_pkg) = root_package {
        packages.insert(root_pkg);
    }

    packages
}

fn collect_brm_packages_recursive(
    root: &Path,
    dir: &Path,
    root_package: Option<&str>,
    in_std: bool,
    out: &mut HashSet<String>,
) {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let is_std_dir = in_std || path.file_name().and_then(|n| n.to_str()) == Some("std");
            collect_brm_packages_recursive(root, &path, root_package, is_std_dir, out);
            continue;
        }

        if path.extension().and_then(|e| e.to_str()) != Some("brm") {
            continue;
        }

        if let Some(pkg) = package_from_path(root, &path, root_package, in_std) {
            out.insert(pkg);
        }
    }
}

fn package_from_path(
    root: &Path,
    file: &Path,
    root_package: Option<&str>,
    in_std: bool,
) -> Option<String> {
    let relative = file.strip_prefix(root).ok()?;
    let parent = relative.parent()?;

    if in_std {
        let parts = path_components(parent);
        if parts.first().map(|s| s.as_str()) == Some("std") {
            if parts.len() > 1 {
                return Some(parts[1..].join("."));
            }
            return None;
        }
    }

    let parts = path_components(parent);
    if parts.is_empty() {
        return root_package.map(str::to_string);
    }

    Some(parts.join("."))
}

fn path_components(path: &Path) -> Vec<String> {
    path.components()
        .filter_map(|component| {
            let val = component.as_os_str().to_string_lossy();
            if val.is_empty() {
                None
            } else {
                Some(val.to_string())
            }
        })
        .collect()
}

fn read_root_package_name(root: &Path) -> Option<String> {
    let project_file = root.join("project.breom");
    let content = fs::read_to_string(project_file).ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("package ") {
            let value = rest.trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

fn resolve_std_packages(root: &Path) -> (HashSet<String>, StdSource) {
    let _ = root;
    let Ok(home) = env::var("BREOM_HOME") else {
        return (HashSet::new(), StdSource::MissingBreomHome);
    };

    let std_src_root = resolve_std_src_root(Path::new(&home));
    let packages = collect_std_packages(&std_src_root);
    (packages, StdSource::BreomHomeSrc)
}

fn resolve_std_src_root(home: &Path) -> PathBuf {
    let versioned_std = home.join(CURRENT_BREOM_VERSION).join("std");
    if versioned_std.exists() && versioned_std.is_dir() {
        return versioned_std;
    }

    let versioned_src_legacy = home.join(CURRENT_BREOM_VERSION).join("src");
    if versioned_src_legacy.exists() && versioned_src_legacy.is_dir() {
        return versioned_src_legacy;
    }

    let legacy_std = home.join("std");
    if legacy_std.exists() && legacy_std.is_dir() {
        return legacy_std;
    }

    let legacy_src = home.join("src");
    if legacy_src.exists() && legacy_src.is_dir() {
        return legacy_src;
    }

    versioned_std
}

fn collect_std_packages(std_root: &Path) -> HashSet<String> {
    if !std_root.exists() || !std_root.is_dir() {
        return HashSet::new();
    }

    let mut packages = HashSet::new();
    collect_std_recursive(std_root, std_root, &mut packages);
    packages
}

fn collect_std_recursive(std_root: &Path, dir: &Path, packages: &mut HashSet<String>) {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_std_recursive(std_root, &path, packages);
            continue;
        }

        if path.extension().and_then(|e| e.to_str()) != Some("brm") {
            continue;
        }

        let Some(relative) = path.strip_prefix(std_root).ok() else {
            continue;
        };
        let Some(parent) = relative.parent() else {
            continue;
        };
        let parts = path_components(parent);
        if parts.is_empty() {
            continue;
        }
        packages.insert(parts.join("."));
    }
}

pub fn std_status_detail(snapshot: &WorkspaceSnapshot) -> &'static str {
    match snapshot.std_source {
        StdSource::BreomHomeSrc => {
            "std resolved from BREOM_HOME/<version>/std (fallbacks: <version>/src, std, src)"
        }
        StdSource::MissingBreomHome => "BREOM_HOME is not set; std packages are unavailable",
    }
}

pub fn completion_packages(snapshot: &WorkspaceSnapshot) -> Vec<CompletionItem> {
    let mut packages = snapshot.known_packages.iter().cloned().collect::<Vec<_>>();
    packages.sort();

    packages
        .into_iter()
        .map(|pkg| CompletionItem {
            label: pkg.clone(),
            kind: Some(CompletionItemKind::MODULE),
            detail: Some(if snapshot.std_packages.contains(&pkg) {
                "std package".to_string()
            } else {
                "project package".to_string()
            }),
            ..Default::default()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
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

    #[test]
    fn resolve_std_packages_requires_breom_home() {
        let base = std::env::temp_dir().join("breom_lsp_no_std_test");
        let _ = fs::create_dir_all(&base);

        let (packages, source) = resolve_std_packages(&base);
        if std::env::var("BREOM_HOME").is_ok() {
            assert_eq!(source, StdSource::BreomHomeSrc);
        } else {
            assert_eq!(source, StdSource::MissingBreomHome);
            assert!(packages.is_empty());
        }

        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn workspace_symbols_include_top_level_items() {
        let root = unique_temp_dir("workspace_symbols");
        fs::write(root.join("project.breom"), "breom 0.1.0\npackage app\n").unwrap();
        fs::write(
            root.join("main.brm"),
            "fn main() Int { return 0 }\nstruct User {}\ndefine LIMIT Int = 1\n",
        )
        .unwrap();

        let uri = Url::from_file_path(root.join("main.brm")).unwrap();
        let mut index = WorkspaceIndex::new();
        let defs = index.find_global_definitions(&uri, "main");
        assert!(!defs.is_empty());

        let symbols = index.query_workspace_symbols(&uri, "Us");
        assert!(symbols.iter().any(|s| s.name == "User"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn workspace_references_collect_cross_file_usages() {
        let root = unique_temp_dir("workspace_references");
        fs::write(root.join("project.breom"), "breom 0.1.0\npackage app\n").unwrap();
        fs::write(
            root.join("main.brm"),
            "define LIMIT Int = 1\nfn main() { print(LIMIT) }\n",
        )
        .unwrap();
        fs::write(root.join("other.brm"), "fn helper() { print(LIMIT) }\n").unwrap();

        let uri = Url::from_file_path(root.join("main.brm")).unwrap();
        let mut index = WorkspaceIndex::new();
        let refs = index.find_global_references(&uri, "LIMIT", None);
        assert!(refs.len() >= 3);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn workspace_references_respect_target_packages() {
        let root = unique_temp_dir("workspace_references_scoped");
        fs::write(root.join("project.breom"), "breom 0.1.0\npackage app\n").unwrap();
        fs::write(
            root.join("main.brm"),
            "define LIMIT Int = 1\nfn main() { print(LIMIT) }\n",
        )
        .unwrap();
        fs::create_dir_all(root.join("other")).unwrap();
        fs::write(
            root.join("other").join("main.brm"),
            "fn helper() { LIMIT Int = 2; print(LIMIT) }\n",
        )
        .unwrap();

        let uri = Url::from_file_path(root.join("main.brm")).unwrap();
        let mut index = WorkspaceIndex::new();
        let packages = index.definition_packages(&uri, "LIMIT");
        let refs = index.find_global_references(&uri, "LIMIT", Some(&packages));

        assert!(refs.iter().any(|loc| loc.uri == uri));
        assert!(!refs
            .iter()
            .any(|loc| loc.uri.path().contains("/other/main.brm")));

        let _ = fs::remove_dir_all(root);
    }
}
