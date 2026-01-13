//! Project detection and auto-discovery
//!
//! Provides functionality for:
//! - Detecting project roots by marker files (.git, Cargo.toml, etc.)
//! - Auto-discovering projects in common locations
//! - Managing project lifecycle (LRU eviction, storage limits)

use std::collections::{HashSet, VecDeque};
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Markers that indicate a project root directory
pub const PROJECT_MARKERS: &[&str] = &[
    // Version control
    ".git",
    ".hg",
    ".svn",
    // Rust
    "Cargo.toml",
    // Node.js
    "package.json",
    // Python
    "pyproject.toml",
    "setup.py",
    "requirements.txt",
    // Go
    "go.mod",
    // Ruby
    "Gemfile",
    // Java/JVM
    "pom.xml",
    "build.gradle",
    "build.gradle.kts",
    // PHP
    "composer.json",
    // .NET
    "*.sln",
    "*.csproj",
    // Generic
    "Makefile",
    "CMakeLists.txt",
];

/// Common directories to scan for projects
pub const DISCOVERY_PATHS: &[&str] = &[
    "~/code",
    "~/Code",
    "~/projects",
    "~/Projects",
    "~/src",
    "~/dev",
    "~/Development",
    "~/workspace",
    "~/repos",
    "~/git",
    "~/GitHub",
];

/// Shell history files to inspect for recent directories
pub const HISTORY_FILES: &[&str] = &[
    "~/.zsh_history",
    "~/.bash_history",
    "~/.config/fish/fish_history",
];

const MAX_HISTORY_LINES: usize = 5_000;

/// Canonicalize a path, returning the original if canonicalization fails
fn canonicalize_or_self(path: PathBuf) -> PathBuf {
    path.canonicalize().unwrap_or(path)
}

/// Canonicalize a path reference, returning a PathBuf
fn canonicalize_ref(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

/// Project information
#[derive(Debug, Clone)]
pub struct Project {
    /// Root directory path
    pub path: PathBuf,
    /// Type of project (detected from marker)
    pub project_type: ProjectType,
    /// Last access time (unix timestamp)
    pub last_accessed: u64,
    /// Whether the project is currently being watched
    pub is_watching: bool,
}

/// Project type based on detected markers
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectType {
    Rust,
    Node,
    Python,
    Go,
    Ruby,
    Java,
    Dotnet,
    Php,
    Generic,
    Unknown,
}

impl ProjectType {
    /// Get the primary marker file for this project type
    pub fn primary_marker(&self) -> &'static str {
        match self {
            ProjectType::Rust => "Cargo.toml",
            ProjectType::Node => "package.json",
            ProjectType::Python => "pyproject.toml",
            ProjectType::Go => "go.mod",
            ProjectType::Ruby => "Gemfile",
            ProjectType::Java => "pom.xml",
            ProjectType::Dotnet => "*.csproj",
            ProjectType::Php => "composer.json",
            ProjectType::Generic | ProjectType::Unknown => ".git",
        }
    }
}

/// Find the project root from a given path by walking up the directory tree
///
/// Returns the first directory containing a project marker, or None if no
/// project root is found before reaching the filesystem root.
pub fn find_project_root(start: &Path) -> Option<PathBuf> {
    let mut current = if start.is_file() {
        start.parent()?.to_path_buf()
    } else {
        start.to_path_buf()
    };

    // Walk up the directory tree
    loop {
        if is_project_root(&current) {
            return Some(current);
        }

        match current.parent() {
            Some(parent) => current = parent.to_path_buf(),
            None => return None,
        }
    }
}

/// Check if a directory is a project root (contains any project marker)
pub fn is_project_root(path: &Path) -> bool {
    if !path.is_dir() {
        return false;
    }

    for marker in PROJECT_MARKERS {
        if marker.contains('*') {
            // Glob pattern (e.g., "*.sln")
            if let Ok(entries) = std::fs::read_dir(path) {
                let pattern = marker.trim_start_matches('*');
                for entry in entries.flatten() {
                    if entry.file_name().to_string_lossy().ends_with(pattern) {
                        return true;
                    }
                }
            }
        } else {
            // Exact match
            if path.join(marker).exists() {
                return true;
            }
        }
    }

    false
}

/// Detect the project type from markers in the directory
pub fn detect_project_type(path: &Path) -> ProjectType {
    if path.join("Cargo.toml").exists() {
        ProjectType::Rust
    } else if path.join("package.json").exists() {
        ProjectType::Node
    } else if path.join("pyproject.toml").exists() || path.join("setup.py").exists() {
        ProjectType::Python
    } else if path.join("go.mod").exists() {
        ProjectType::Go
    } else if path.join("Gemfile").exists() {
        ProjectType::Ruby
    } else if path.join("pom.xml").exists()
        || path.join("build.gradle").exists()
        || path.join("build.gradle.kts").exists()
    {
        ProjectType::Java
    } else if path.join("composer.json").exists() {
        ProjectType::Php
    } else if has_dotnet_files(path) {
        ProjectType::Dotnet
    } else if path.join("Makefile").exists() || path.join("CMakeLists.txt").exists() {
        ProjectType::Generic
    } else {
        ProjectType::Unknown
    }
}

/// Check for .NET project files
fn has_dotnet_files(path: &Path) -> bool {
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.ends_with(".sln") || name_str.ends_with(".csproj") {
                return true;
            }
        }
    }
    false
}

/// Expand a path that may contain ~ to the user's home directory
pub fn expand_path(path: &str) -> Option<PathBuf> {
    if path.starts_with("~/") {
        dirs::home_dir().map(|home| home.join(&path[2..]))
    } else if path == "~" {
        dirs::home_dir()
    } else {
        Some(PathBuf::from(path))
    }
}

/// Discover projects in common locations
///
/// Scans DISCOVERY_PATHS for directories that contain project markers.
/// Only searches one level deep in each discovery path.
pub fn discover_projects() -> Vec<Project> {
    let mut projects = Vec::new();
    let mut seen: HashSet<PathBuf> = HashSet::new();

    for discovery_path in DISCOVERY_PATHS {
        if let Some(expanded) = expand_path(discovery_path) {
            if expanded.is_dir() {
                // Scan immediate subdirectories
                if let Ok(entries) = std::fs::read_dir(&expanded) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir() && is_project_root(&path) {
                            // Canonicalize to avoid duplicates
                            if let Ok(canonical) = path.canonicalize() {
                                if !seen.contains(&canonical) {
                                    seen.insert(canonical.clone());
                                    projects.push(Project {
                                        path: canonical,
                                        project_type: detect_project_type(&path),
                                        last_accessed: get_current_timestamp(),
                                        is_watching: false,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    for root in discover_history_roots() {
        if let Ok(canonical) = root.canonicalize() {
            if !seen.contains(&canonical) {
                seen.insert(canonical.clone());
                projects.push(Project {
                    path: canonical,
                    project_type: detect_project_type(&root),
                    last_accessed: get_current_timestamp(),
                    is_watching: false,
                });
            }
        }
    }

    projects
}

fn discover_history_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    let mut seen: HashSet<PathBuf> = HashSet::new();

    for history_path in history_files() {
        if let Ok(file) = File::open(&history_path) {
            if let Ok(lines) = tail_lines(file, MAX_HISTORY_LINES) {
                for line in lines {
                    if let Some(path) = extract_cd_path_from_history_line(&line) {
                        if !path.exists() {
                            continue;
                        }
                        if let Some(root) = find_project_root(&path) {
                            if let Ok(canonical) = root.canonicalize() {
                                if !seen.contains(&canonical) {
                                    seen.insert(canonical.clone());
                                    roots.push(canonical);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    roots
}

fn history_files() -> Vec<PathBuf> {
    HISTORY_FILES
        .iter()
        .filter_map(|path| expand_path(path))
        .collect()
}

fn tail_lines(file: File, max_lines: usize) -> io::Result<Vec<String>> {
    let reader = BufReader::new(file);
    let mut buffer: VecDeque<String> = VecDeque::with_capacity(max_lines);
    for line in reader.lines() {
        let line = line?;
        if buffer.len() == max_lines {
            buffer.pop_front();
        }
        buffer.push_back(line);
    }
    Ok(buffer.into_iter().collect())
}

fn extract_cd_path_from_history_line(line: &str) -> Option<PathBuf> {
    let command = history_line_to_command(line)?;
    extract_cd_path(command)
}

fn history_line_to_command(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    if trimmed.is_empty() {
        return None;
    }

    if let Some(rest) = trimmed.strip_prefix('#') {
        if rest.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
    }

    if trimmed.starts_with(':') {
        if let Some(idx) = trimmed.find(';') {
            return Some(trimmed[idx + 1..].trim());
        }
    }

    if let Some(rest) = trimmed.strip_prefix("- cmd:") {
        return Some(rest.trim());
    }

    if let Some(rest) = trimmed.strip_prefix("cmd:") {
        return Some(rest.trim());
    }

    if trimmed.starts_with("when:") {
        return None;
    }

    Some(trimmed)
}

fn extract_cd_path(command: &str) -> Option<PathBuf> {
    let trimmed = command.trim_start();
    let rest = if trimmed == "cd" || trimmed.starts_with("cd ") || trimmed.starts_with("cd\t") {
        &trimmed[2..]
    } else if trimmed == "pushd" || trimmed.starts_with("pushd ") || trimmed.starts_with("pushd\t")
    {
        &trimmed[5..]
    } else {
        return None;
    };

    let mut rest = rest.trim_start();
    if rest.starts_with("--") {
        rest = rest[2..].trim_start();
    }
    if rest.is_empty() || rest.starts_with('-') {
        return None;
    }

    let token = parse_shell_token(rest)?;
    let expanded = expand_path(&token)?;
    if !expanded.is_absolute() {
        return None;
    }
    Some(expanded)
}

fn parse_shell_token(input: &str) -> Option<String> {
    let trimmed = input.trim_start();
    let mut chars = trimmed.chars();
    let first = chars.next()?;

    if first == '"' || first == '\'' {
        let mut token = String::new();
        for c in chars {
            if c == first {
                return Some(token);
            }
            token.push(c);
        }
        return None;
    }

    let mut token = String::new();
    token.push(first);
    for c in chars {
        if c.is_whitespace() || c == ';' || c == '&' {
            break;
        }
        token.push(c);
    }
    Some(token)
}

/// Get current unix timestamp
pub(crate) fn get_current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Project manager for tracking and managing discovered projects
#[derive(Debug, Default)]
pub struct ProjectManager {
    /// Known projects (path -> project)
    projects: std::collections::HashMap<PathBuf, Project>,
    /// Maximum number of projects to track
    max_projects: usize,
    /// Maximum age in seconds before a project is considered stale
    stale_threshold_secs: u64,
}

impl ProjectManager {
    /// Create a new project manager with default settings
    pub fn new() -> Self {
        Self {
            projects: std::collections::HashMap::new(),
            max_projects: 100,
            stale_threshold_secs: 30 * 24 * 60 * 60, // 30 days
        }
    }

    /// Create with custom limits
    pub fn with_limits(max_projects: usize, stale_threshold_secs: u64) -> Self {
        Self {
            projects: std::collections::HashMap::new(),
            max_projects,
            stale_threshold_secs,
        }
    }

    /// Add or update a project
    pub fn add_project(&mut self, path: PathBuf) -> &Project {
        let now = get_current_timestamp();

        // Canonicalize the path to avoid duplicates from relative paths like "."
        let canonical_path = canonicalize_or_self(path);

        self.projects
            .entry(canonical_path.clone())
            .and_modify(|p| p.last_accessed = now)
            .or_insert_with(|| Project {
                project_type: detect_project_type(&canonical_path),
                path: canonical_path,
                last_accessed: now,
                is_watching: false,
            })
    }

    /// Mark a project as being watched
    pub fn set_watching(&mut self, path: &Path, watching: bool) {
        // Canonicalize for consistent lookup
        let canonical_path = canonicalize_ref(path);
        if let Some(project) = self.projects.get_mut(&canonical_path) {
            project.is_watching = watching;
            project.last_accessed = get_current_timestamp();
        }
    }

    /// Touch a project (update last_accessed time)
    pub fn touch(&mut self, path: &Path) {
        // Canonicalize for consistent lookup
        let canonical_path = canonicalize_ref(path);
        if let Some(project) = self.projects.get_mut(&canonical_path) {
            project.last_accessed = get_current_timestamp();
        }
    }

    /// Get a project by path
    pub fn get(&self, path: &Path) -> Option<&Project> {
        // Canonicalize for consistent lookup
        let canonical_path = canonicalize_ref(path);
        self.projects.get(&canonical_path)
    }

    /// Get all projects sorted by last_accessed (most recent first)
    pub fn all_projects(&self) -> Vec<&Project> {
        let mut projects: Vec<_> = self.projects.values().collect();
        projects.sort_by(|a, b| b.last_accessed.cmp(&a.last_accessed));
        projects
    }

    /// Get projects currently being watched
    pub fn watched_projects(&self) -> Vec<&Project> {
        self.projects.values().filter(|p| p.is_watching).collect()
    }

    /// Remove stale projects (not accessed within stale_threshold_secs)
    ///
    /// Returns the paths of removed projects
    pub fn evict_stale(&mut self) -> Vec<PathBuf> {
        let now = get_current_timestamp();
        let threshold = now.saturating_sub(self.stale_threshold_secs);

        let stale: Vec<PathBuf> = self
            .projects
            .iter()
            .filter(|(_, p)| !p.is_watching && p.last_accessed < threshold)
            .map(|(path, _)| path.clone())
            .collect();

        for path in &stale {
            self.projects.remove(path);
        }

        stale
    }

    /// Evict least recently used projects if over max_projects limit
    ///
    /// Returns the paths of removed projects
    pub fn evict_lru(&mut self) -> Vec<PathBuf> {
        if self.projects.len() <= self.max_projects {
            return Vec::new();
        }

        // Sort by last_accessed (oldest first), excluding watching projects
        let mut candidates: Vec<_> = self
            .projects
            .iter()
            .filter(|(_, p)| !p.is_watching)
            .map(|(path, p)| (path.clone(), p.last_accessed))
            .collect();

        candidates.sort_by_key(|(_, ts)| *ts);

        let num_to_evict = self.projects.len() - self.max_projects;
        let evicted: Vec<PathBuf> = candidates
            .into_iter()
            .take(num_to_evict)
            .map(|(path, _)| path)
            .collect();

        for path in &evicted {
            self.projects.remove(path);
        }

        evicted
    }

    /// Get the number of tracked projects
    pub fn count(&self) -> usize {
        self.projects.len()
    }

    /// Run discovery and add found projects
    pub fn run_discovery(&mut self) {
        for project in discover_projects() {
            self.projects.entry(project.path.clone()).or_insert(project);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_find_project_root_with_cargo() {
        let temp = TempDir::new().unwrap();
        let project_dir = temp.path().join("my_project");
        fs::create_dir(&project_dir).unwrap();
        fs::write(project_dir.join("Cargo.toml"), "[package]").unwrap();

        let subdir = project_dir.join("src");
        fs::create_dir(&subdir).unwrap();

        let found = find_project_root(&subdir);
        assert_eq!(found, Some(project_dir));
    }

    #[test]
    fn test_find_project_root_with_git() {
        let temp = TempDir::new().unwrap();
        let project_dir = temp.path().join("my_project");
        fs::create_dir(&project_dir).unwrap();
        fs::create_dir(project_dir.join(".git")).unwrap();

        let found = find_project_root(&project_dir);
        assert_eq!(found, Some(project_dir));
    }

    #[test]
    fn test_find_project_root_no_marker() {
        let temp = TempDir::new().unwrap();
        let found = find_project_root(temp.path());
        // May find something if run inside a real project, or None
        // Just ensure it doesn't panic
        let _ = found;
    }

    #[test]
    fn test_is_project_root() {
        let temp = TempDir::new().unwrap();

        // Not a project root initially
        assert!(!is_project_root(temp.path()));

        // Add Cargo.toml
        fs::write(temp.path().join("Cargo.toml"), "").unwrap();
        assert!(is_project_root(temp.path()));
    }

    #[test]
    fn test_detect_project_type() {
        let temp = TempDir::new().unwrap();

        // Default is Unknown
        assert_eq!(detect_project_type(temp.path()), ProjectType::Unknown);

        // Rust project
        fs::write(temp.path().join("Cargo.toml"), "").unwrap();
        assert_eq!(detect_project_type(temp.path()), ProjectType::Rust);
    }

    #[test]
    fn test_is_project_root_with_glob_marker() {
        let temp = TempDir::new().unwrap();
        let project_dir = temp.path().join("dotnet_project");
        fs::create_dir(&project_dir).unwrap();
        fs::write(project_dir.join("app.sln"), "").unwrap();

        assert!(is_project_root(&project_dir));
    }

    #[test]
    fn test_detect_project_type_variants() {
        let cases = [
            ("package.json", ProjectType::Node),
            ("pyproject.toml", ProjectType::Python),
            ("go.mod", ProjectType::Go),
            ("Gemfile", ProjectType::Ruby),
            ("pom.xml", ProjectType::Java),
            ("project.csproj", ProjectType::Dotnet),
            ("composer.json", ProjectType::Php),
            ("Makefile", ProjectType::Generic),
        ];

        for (marker, expected) in cases {
            let temp = TempDir::new().unwrap();
            fs::write(temp.path().join(marker), "").unwrap();
            assert_eq!(
                detect_project_type(temp.path()),
                expected,
                "marker {marker} should map to {expected:?}"
            );
        }
    }

    #[test]
    fn test_project_manager_add_and_get() {
        let mut manager = ProjectManager::new();
        let path = PathBuf::from("/test/project");

        manager.add_project(path.clone());

        let project = manager.get(&path).unwrap();
        assert_eq!(project.path, path);
    }

    #[test]
    fn test_project_manager_lru_eviction() {
        let mut manager = ProjectManager::with_limits(2, 30 * 24 * 60 * 60);

        // Add 3 projects
        manager.add_project(PathBuf::from("/project1"));
        manager.add_project(PathBuf::from("/project2"));
        manager.add_project(PathBuf::from("/project3"));

        assert_eq!(manager.count(), 3);

        // Evict LRU (should remove 1)
        let evicted = manager.evict_lru();
        assert_eq!(evicted.len(), 1);
        assert_eq!(manager.count(), 2);
    }

    #[test]
    fn test_project_manager_watching_prevents_eviction() {
        let mut manager = ProjectManager::with_limits(1, 30 * 24 * 60 * 60);

        let path1 = PathBuf::from("/project1");
        let path2 = PathBuf::from("/project2");

        manager.add_project(path1.clone());
        manager.set_watching(&path1, true);
        manager.add_project(path2.clone());

        // Even though we're over limit, watching project shouldn't be evicted
        let _evicted = manager.evict_lru();

        // Project1 is watching, so only project2 can be evicted
        // But we're at limit so project2 gets evicted
        assert!(manager.get(&path1).is_some());
    }

    #[test]
    fn test_expand_path() {
        // Just test that it doesn't panic
        let expanded = expand_path("~/code");
        // Should expand if home dir exists
        if dirs::home_dir().is_some() {
            assert!(expanded.is_some());
            assert!(!expanded.unwrap().to_string_lossy().contains('~'));
        }
    }

    #[test]
    fn test_history_line_to_command() {
        assert_eq!(
            history_line_to_command(": 1680000000:0;cd /tmp").unwrap(),
            "cd /tmp"
        );
        assert_eq!(
            history_line_to_command("- cmd: cd /tmp").unwrap(),
            "cd /tmp"
        );
        assert!(history_line_to_command("#1680000000").is_none());
        assert!(history_line_to_command("when: 1680000000").is_none());
    }

    #[test]
    fn test_extract_cd_path() {
        let temp = TempDir::new().unwrap();
        let project = temp.path().join("my project");
        fs::create_dir(&project).unwrap();

        let command = format!("cd \"{}\"", project.display());
        let extracted = extract_cd_path(&command).unwrap();
        assert_eq!(extracted, project);

        assert!(extract_cd_path("cd -").is_none());
        assert!(extract_cd_path("ls /tmp").is_none());
    }

    #[test]
    fn test_extract_cd_path_from_history_line() {
        let temp = TempDir::new().unwrap();
        let project = temp.path().join("project");
        fs::create_dir(&project).unwrap();

        let line = format!(": 1680000000:0;cd {}", project.display());
        let extracted = extract_cd_path_from_history_line(&line).unwrap();
        assert_eq!(extracted, project);
    }

    #[test]
    fn test_project_type_primary_marker() {
        assert_eq!(ProjectType::Rust.primary_marker(), "Cargo.toml");
        assert_eq!(ProjectType::Node.primary_marker(), "package.json");
        assert_eq!(ProjectType::Python.primary_marker(), "pyproject.toml");
        assert_eq!(ProjectType::Go.primary_marker(), "go.mod");
        assert_eq!(ProjectType::Ruby.primary_marker(), "Gemfile");
        assert_eq!(ProjectType::Java.primary_marker(), "pom.xml");
        assert_eq!(ProjectType::Dotnet.primary_marker(), "*.csproj");
        assert_eq!(ProjectType::Php.primary_marker(), "composer.json");
        assert_eq!(ProjectType::Generic.primary_marker(), ".git");
        assert_eq!(ProjectType::Unknown.primary_marker(), ".git");
    }

    #[test]
    fn test_project_manager_touch() {
        let mut manager = ProjectManager::new();
        let path = PathBuf::from("/test/project");

        manager.add_project(path.clone());
        let initial_ts = manager.get(&path).unwrap().last_accessed;

        // Sleep briefly to ensure timestamp changes
        std::thread::sleep(std::time::Duration::from_millis(10));

        manager.touch(&path);
        let updated_ts = manager.get(&path).unwrap().last_accessed;

        assert!(updated_ts >= initial_ts);
    }

    #[test]
    fn test_project_manager_all_projects_sorted() {
        let mut manager = ProjectManager::new();

        // Add projects with different timestamps by manipulating last_accessed directly
        manager.add_project(PathBuf::from("/project_a"));
        manager.add_project(PathBuf::from("/project_b"));
        manager.add_project(PathBuf::from("/project_c"));

        // Modify timestamps to ensure known order
        if let Some(p) = manager.projects.get_mut(&PathBuf::from("/project_a")) {
            p.last_accessed = 100;
        }
        if let Some(p) = manager.projects.get_mut(&PathBuf::from("/project_b")) {
            p.last_accessed = 300;
        }
        if let Some(p) = manager.projects.get_mut(&PathBuf::from("/project_c")) {
            p.last_accessed = 200;
        }

        let projects = manager.all_projects();
        assert_eq!(projects.len(), 3);
        // Should be sorted by last_accessed descending (most recent first)
        assert_eq!(projects[0].last_accessed, 300);
        assert_eq!(projects[1].last_accessed, 200);
        assert_eq!(projects[2].last_accessed, 100);
    }

    #[test]
    fn test_project_manager_watched_projects() {
        let mut manager = ProjectManager::new();

        let path1 = PathBuf::from("/project1");
        let path2 = PathBuf::from("/project2");
        let path3 = PathBuf::from("/project3");

        manager.add_project(path1.clone());
        manager.add_project(path2.clone());
        manager.add_project(path3.clone());

        // Initially none are watching
        assert_eq!(manager.watched_projects().len(), 0);

        // Set some as watching
        manager.set_watching(&path1, true);
        manager.set_watching(&path3, true);

        let watched = manager.watched_projects();
        assert_eq!(watched.len(), 2);
        assert!(watched.iter().any(|p| p.path == path1));
        assert!(watched.iter().any(|p| p.path == path3));
        assert!(!watched.iter().any(|p| p.path == path2));
    }

    #[test]
    fn test_project_manager_evict_stale() {
        // Create manager with 100-second stale threshold
        let mut manager = ProjectManager::with_limits(10, 100);

        let path1 = PathBuf::from("/project1");
        let path2 = PathBuf::from("/project2");
        let path3 = PathBuf::from("/project3");

        manager.add_project(path1.clone());
        manager.add_project(path2.clone());
        manager.add_project(path3.clone());

        // Get current timestamp to compute stale time
        let now = get_current_timestamp();

        // Make project1 stale (older than threshold)
        if let Some(p) = manager.projects.get_mut(&path1) {
            p.last_accessed = now.saturating_sub(200); // 200 seconds ago
        }

        // Make project2 recent (within threshold)
        if let Some(p) = manager.projects.get_mut(&path2) {
            p.last_accessed = now.saturating_sub(50); // 50 seconds ago
        }

        // Make project3 stale but watching (should not be evicted)
        if let Some(p) = manager.projects.get_mut(&path3) {
            p.last_accessed = now.saturating_sub(200); // 200 seconds ago
            p.is_watching = true;
        }

        // Evict stale projects
        let evicted = manager.evict_stale();

        // Only project1 should be evicted (stale and not watching)
        assert_eq!(evicted.len(), 1);
        assert!(evicted.contains(&path1));

        // project2 should remain (recent)
        assert!(manager.get(&path2).is_some());

        // project3 should remain (watching protects from eviction)
        assert!(manager.get(&path3).is_some());

        // project1 should be gone
        assert!(manager.get(&path1).is_none());
    }

    #[test]
    fn test_detect_project_type_all_types() {
        // Test all project type detection
        let temp = TempDir::new().unwrap();

        // Node project
        let node_dir = temp.path().join("node_project");
        fs::create_dir(&node_dir).unwrap();
        fs::write(node_dir.join("package.json"), "{}").unwrap();
        assert_eq!(detect_project_type(&node_dir), ProjectType::Node);

        // Python project (pyproject.toml)
        let py_dir = temp.path().join("python_project");
        fs::create_dir(&py_dir).unwrap();
        fs::write(py_dir.join("pyproject.toml"), "").unwrap();
        assert_eq!(detect_project_type(&py_dir), ProjectType::Python);

        // Python project (setup.py)
        let py_setup_dir = temp.path().join("python_setup_project");
        fs::create_dir(&py_setup_dir).unwrap();
        fs::write(py_setup_dir.join("setup.py"), "").unwrap();
        assert_eq!(detect_project_type(&py_setup_dir), ProjectType::Python);

        // Go project
        let go_dir = temp.path().join("go_project");
        fs::create_dir(&go_dir).unwrap();
        fs::write(go_dir.join("go.mod"), "").unwrap();
        assert_eq!(detect_project_type(&go_dir), ProjectType::Go);

        // Ruby project
        let ruby_dir = temp.path().join("ruby_project");
        fs::create_dir(&ruby_dir).unwrap();
        fs::write(ruby_dir.join("Gemfile"), "").unwrap();
        assert_eq!(detect_project_type(&ruby_dir), ProjectType::Ruby);

        // Java project (pom.xml)
        let java_dir = temp.path().join("java_project");
        fs::create_dir(&java_dir).unwrap();
        fs::write(java_dir.join("pom.xml"), "").unwrap();
        assert_eq!(detect_project_type(&java_dir), ProjectType::Java);

        // Java project (build.gradle)
        let gradle_dir = temp.path().join("gradle_project");
        fs::create_dir(&gradle_dir).unwrap();
        fs::write(gradle_dir.join("build.gradle"), "").unwrap();
        assert_eq!(detect_project_type(&gradle_dir), ProjectType::Java);

        // Java project (build.gradle.kts)
        let gradle_kts_dir = temp.path().join("gradle_kts_project");
        fs::create_dir(&gradle_kts_dir).unwrap();
        fs::write(gradle_kts_dir.join("build.gradle.kts"), "").unwrap();
        assert_eq!(detect_project_type(&gradle_kts_dir), ProjectType::Java);

        // PHP project
        let php_dir = temp.path().join("php_project");
        fs::create_dir(&php_dir).unwrap();
        fs::write(php_dir.join("composer.json"), "{}").unwrap();
        assert_eq!(detect_project_type(&php_dir), ProjectType::Php);

        // Generic project (Makefile)
        let make_dir = temp.path().join("make_project");
        fs::create_dir(&make_dir).unwrap();
        fs::write(make_dir.join("Makefile"), "").unwrap();
        assert_eq!(detect_project_type(&make_dir), ProjectType::Generic);

        // Generic project (CMakeLists.txt)
        let cmake_dir = temp.path().join("cmake_project");
        fs::create_dir(&cmake_dir).unwrap();
        fs::write(cmake_dir.join("CMakeLists.txt"), "").unwrap();
        assert_eq!(detect_project_type(&cmake_dir), ProjectType::Generic);
    }

    #[test]
    fn test_detect_project_type_dotnet() {
        let temp = TempDir::new().unwrap();

        // .NET project (.sln)
        let sln_dir = temp.path().join("dotnet_sln");
        fs::create_dir(&sln_dir).unwrap();
        fs::write(sln_dir.join("MyProject.sln"), "").unwrap();
        assert_eq!(detect_project_type(&sln_dir), ProjectType::Dotnet);

        // .NET project (.csproj)
        let csproj_dir = temp.path().join("dotnet_csproj");
        fs::create_dir(&csproj_dir).unwrap();
        fs::write(csproj_dir.join("MyProject.csproj"), "").unwrap();
        assert_eq!(detect_project_type(&csproj_dir), ProjectType::Dotnet);
    }

    #[test]
    fn test_is_project_root_with_glob_patterns() {
        let temp = TempDir::new().unwrap();

        // Test .sln pattern detection
        let sln_dir = temp.path().join("sln_project");
        fs::create_dir(&sln_dir).unwrap();
        assert!(!is_project_root(&sln_dir)); // No marker yet
        fs::write(sln_dir.join("Solution.sln"), "").unwrap();
        assert!(is_project_root(&sln_dir));

        // Test .csproj pattern detection
        let csproj_dir = temp.path().join("csproj_project");
        fs::create_dir(&csproj_dir).unwrap();
        assert!(!is_project_root(&csproj_dir));
        fs::write(csproj_dir.join("Project.csproj"), "").unwrap();
        assert!(is_project_root(&csproj_dir));
    }

    #[test]
    fn test_find_project_root_from_file() {
        let temp = TempDir::new().unwrap();
        let project_dir = temp.path().join("my_project");
        fs::create_dir(&project_dir).unwrap();
        fs::write(project_dir.join("Cargo.toml"), "[package]").unwrap();

        let src_dir = project_dir.join("src");
        fs::create_dir(&src_dir).unwrap();
        let file_path = src_dir.join("main.rs");
        fs::write(&file_path, "fn main() {}").unwrap();

        // Starting from a file should still find the project root
        let found = find_project_root(&file_path);
        assert_eq!(found, Some(project_dir));
    }

    #[test]
    fn test_extract_cd_path_pushd() {
        let temp = TempDir::new().unwrap();
        let project = temp.path().join("project");
        fs::create_dir(&project).unwrap();

        // Test pushd command
        let command = format!("pushd {}", project.display());
        let extracted = extract_cd_path(&command).unwrap();
        assert_eq!(extracted, project);

        // Test pushd with quoted path
        let quoted_project = temp.path().join("my project");
        fs::create_dir(&quoted_project).unwrap();
        let command = format!("pushd \"{}\"", quoted_project.display());
        let extracted = extract_cd_path(&command).unwrap();
        assert_eq!(extracted, quoted_project);
    }

    #[test]
    fn test_extract_cd_path_single_quoted() {
        let temp = TempDir::new().unwrap();
        let project = temp.path().join("my project");
        fs::create_dir(&project).unwrap();

        let command = format!("cd '{}'", project.display());
        let extracted = extract_cd_path(&command).unwrap();
        assert_eq!(extracted, project);
    }

    #[test]
    fn test_extract_cd_path_with_double_dash() {
        let temp = TempDir::new().unwrap();
        let project = temp.path().join("project");
        fs::create_dir(&project).unwrap();

        // cd -- path (double dash to stop option parsing)
        let command = format!("cd -- {}", project.display());
        let extracted = extract_cd_path(&command).unwrap();
        assert_eq!(extracted, project);
    }

    #[test]
    fn test_extract_cd_path_edge_cases() {
        // cd alone (no path) returns None
        assert!(extract_cd_path("cd").is_none());
        assert!(extract_cd_path("cd ").is_none());

        // cd with only options returns None
        assert!(extract_cd_path("cd --").is_none());

        // Not a cd command
        assert!(extract_cd_path("echo hello").is_none());
        assert!(extract_cd_path("").is_none());
    }

    #[test]
    fn test_history_line_to_command_cmd_format() {
        // cmd: format without dash prefix
        assert_eq!(
            history_line_to_command("cmd: cd /home/user").unwrap(),
            "cd /home/user"
        );
    }

    #[test]
    fn test_history_line_to_command_plain_command() {
        // Plain command without any special format
        assert_eq!(
            history_line_to_command("cd /home/user").unwrap(),
            "cd /home/user"
        );

        // Plain command with leading whitespace
        assert_eq!(
            history_line_to_command("   cd /home/user").unwrap(),
            "cd /home/user"
        );
    }

    #[test]
    fn test_history_line_to_command_empty_and_whitespace() {
        assert!(history_line_to_command("").is_none());
        assert!(history_line_to_command("   ").is_none());
    }

    #[test]
    fn test_expand_path_variants() {
        // Just "~" should expand to home dir
        if let Some(home) = dirs::home_dir() {
            let expanded = expand_path("~").unwrap();
            assert_eq!(expanded, home);
        }

        // Absolute path should remain unchanged
        let abs_path = "/usr/local/bin";
        let expanded = expand_path(abs_path).unwrap();
        assert_eq!(expanded, PathBuf::from(abs_path));

        // Relative path should remain unchanged
        let rel_path = "relative/path";
        let expanded = expand_path(rel_path).unwrap();
        assert_eq!(expanded, PathBuf::from(rel_path));
    }

    #[test]
    fn test_is_project_root_not_a_directory() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("not_a_dir.txt");
        fs::write(&file_path, "content").unwrap();

        // File should not be considered a project root
        assert!(!is_project_root(&file_path));
    }

    #[test]
    fn test_project_manager_add_updates_existing() {
        let mut manager = ProjectManager::new();
        let path = PathBuf::from("/test/project");

        // Add project first time
        manager.add_project(path.clone());
        let initial_ts = manager.get(&path).unwrap().last_accessed;

        // Wait briefly
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Add same project again
        manager.add_project(path.clone());
        let updated_ts = manager.get(&path).unwrap().last_accessed;

        // Timestamp should be updated
        assert!(updated_ts >= initial_ts);
        // But still only one project
        assert_eq!(manager.count(), 1);
    }

    #[test]
    fn test_project_manager_evict_lru_under_limit() {
        let mut manager = ProjectManager::with_limits(10, 30 * 24 * 60 * 60);

        manager.add_project(PathBuf::from("/project1"));
        manager.add_project(PathBuf::from("/project2"));

        // Under limit, nothing should be evicted
        let evicted = manager.evict_lru();
        assert!(evicted.is_empty());
        assert_eq!(manager.count(), 2);
    }

    // Tests for derive traits on Project
    #[test]
    fn test_project_debug() {
        let project = Project {
            path: PathBuf::from("/test/project"),
            project_type: ProjectType::Rust,
            last_accessed: 1234567890,
            is_watching: true,
        };
        let debug_str = format!("{project:?}");
        assert!(debug_str.contains("Project"));
        assert!(debug_str.contains("/test/project"));
        assert!(debug_str.contains("Rust"));
        assert!(debug_str.contains("1234567890"));
        assert!(debug_str.contains("true"));
    }

    #[test]
    fn test_project_clone() {
        let project = Project {
            path: PathBuf::from("/test/project"),
            project_type: ProjectType::Rust,
            last_accessed: 1234567890,
            is_watching: true,
        };
        let cloned = project.clone();
        assert_eq!(cloned.path, project.path);
        assert_eq!(cloned.project_type, project.project_type);
        assert_eq!(cloned.last_accessed, project.last_accessed);
        assert_eq!(cloned.is_watching, project.is_watching);
    }

    // Tests for derive traits on ProjectType
    #[test]
    fn test_project_type_debug() {
        let types = [
            (ProjectType::Rust, "Rust"),
            (ProjectType::Node, "Node"),
            (ProjectType::Python, "Python"),
            (ProjectType::Go, "Go"),
            (ProjectType::Ruby, "Ruby"),
            (ProjectType::Java, "Java"),
            (ProjectType::Dotnet, "Dotnet"),
            (ProjectType::Php, "Php"),
            (ProjectType::Generic, "Generic"),
            (ProjectType::Unknown, "Unknown"),
        ];
        for (pt, expected) in types {
            let debug_str = format!("{pt:?}");
            assert_eq!(debug_str, expected);
        }
    }

    #[test]
    fn test_project_type_clone() {
        let pt = ProjectType::Rust;
        let cloned = pt.clone();
        assert_eq!(cloned, pt);
    }

    #[test]
    fn test_project_type_eq() {
        assert_eq!(ProjectType::Rust, ProjectType::Rust);
        assert_ne!(ProjectType::Rust, ProjectType::Node);
        assert_eq!(ProjectType::Unknown, ProjectType::Unknown);
    }

    // Tests for derive traits on ProjectManager
    #[test]
    fn test_project_manager_debug() {
        let manager = ProjectManager::new();
        let debug_str = format!("{manager:?}");
        assert!(debug_str.contains("ProjectManager"));
        assert!(debug_str.contains("projects"));
        assert!(debug_str.contains("max_projects"));
        assert!(debug_str.contains("stale_threshold_secs"));
    }

    #[test]
    fn test_project_manager_default() {
        let manager = ProjectManager::default();
        // Default should have empty projects map
        assert_eq!(manager.count(), 0);
        // Default values from Default derive use struct field defaults (0)
        // This is different from new() which sets specific defaults
        let debug_str = format!("{manager:?}");
        assert!(debug_str.contains("projects"));
    }

    #[test]
    fn test_project_manager_debug_with_projects() {
        let mut manager = ProjectManager::new();
        manager.add_project(PathBuf::from("/test/project1"));
        manager.add_project(PathBuf::from("/test/project2"));
        let debug_str = format!("{manager:?}");
        assert!(debug_str.contains("ProjectManager"));
        // Should show the projects in debug output
        assert!(debug_str.contains("/test/project1"));
        assert!(debug_str.contains("/test/project2"));
    }
}
