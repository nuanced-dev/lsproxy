use lsproxy_common::utils::file_utils::{search_paths, FileType};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

struct TestDirectory {
    _temp_dir: TempDir,
    root: PathBuf,
}

impl TestDirectory {
    fn new() -> Self {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().to_path_buf();
        Self {
            _temp_dir: temp_dir,
            root,
        }
    }

    fn create_file(&self, path: &str) {
        let full_path = self.root.join(path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&full_path, "test content").unwrap();
    }

    fn path(&self) -> &Path {
        &self.root
    }
}

fn create_test_directory() -> TestDirectory {
    let test_dir = TestDirectory::new();

    for i in 0..20 {
        test_dir.create_file(&format!("src/module_{}/mod.rs", i));
        for j in 0..5 {
            test_dir.create_file(&format!("src/module_{}/file_{}.rs", i, j));
        }
    }

    for i in 0..30 {
        test_dir.create_file(&format!("tests/test_{}.rs", i));
    }

    test_dir.create_file("Cargo.toml");
    test_dir.create_file("README.md");
    test_dir.create_file(".gitignore");

    test_dir
}

#[test]
fn test_search_paths_files() {
    let test_dir = create_test_directory();
    let include_patterns = vec!["**/*.rs".to_string()];
    let exclude_patterns: Vec<String> = vec![];

    let results = search_paths(
        test_dir.path(),
        include_patterns,
        exclude_patterns,
        false,
        FileType::File,
    )
    .unwrap();

    // Should find 20 modules * 5 files + 20 mod.rs + 30 tests = 150 files
    assert_eq!(results.len(), 150);
}

#[test]
fn test_search_paths_with_exclusions() {
    let test_dir = create_test_directory();
    let include_patterns = vec!["**/*.rs".to_string()];
    let exclude_patterns = vec!["**/tests/**".to_string()];

    let results = search_paths(
        test_dir.path(),
        include_patterns,
        exclude_patterns,
        false,
        FileType::File,
    )
    .unwrap();

    // Should find 20 modules * 5 files + 20 mod.rs = 120 files (excluding tests)
    assert_eq!(results.len(), 120);
}

#[test]
fn test_search_paths_directories() {
    let test_dir = create_test_directory();
    let include_patterns = vec!["**/*.rs".to_string()];
    let exclude_patterns: Vec<String> = vec![];

    let results = search_paths(
        test_dir.path(),
        include_patterns,
        exclude_patterns,
        false,
        FileType::Dir,
    )
    .unwrap();

    // Should find unique directories containing .rs files
    assert!(results.len() > 0);
    // Verify deduplication worked
    let unique_count = results.iter().collect::<std::collections::HashSet<_>>().len();
    assert_eq!(results.len(), unique_count);
}
