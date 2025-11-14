use lsproxy::utils::file_utils::{search_paths, FileType};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Helper to create a test directory structure
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

    test_dir.create_file("src/main.rs");
    test_dir.create_file("src/lib.rs");
    test_dir.create_file("src/utils/mod.rs");
    test_dir.create_file("src/utils/helpers.rs");
    test_dir.create_file("tests/test1.rs");
    test_dir.create_file("tests/test2.rs");
    test_dir.create_file("Cargo.toml");
    test_dir.create_file("README.md");

    test_dir
}

/// Helper to sort PathBuf vectors
fn normalize_paths(mut paths: Vec<PathBuf>) -> Vec<PathBuf> {
    paths.sort();
    paths
}

#[test]
fn test_search_functionality() {
    let test_dir = create_test_directory();

    // Basic file search
    let results = search_paths(
        test_dir.path(),
        vec!["**/*.rs".to_string()],
        vec![],
        false,
        FileType::File,
    )
    .unwrap();
    assert_eq!(results.len(), 6, "Expected 6 .rs files");

    // Multiple include patterns
    let results = search_paths(
        test_dir.path(),
        vec!["**/*.rs".to_string(), "**/*.toml".to_string()],
        vec![],
        false,
        FileType::File,
    )
    .unwrap();
    assert_eq!(results.len(), 7, "Expected 7 files (.rs and .toml)");

    // Exclude patterns
    let results = search_paths(
        test_dir.path(),
        vec!["**/*.rs".to_string()],
        vec!["**/tests/**".to_string()],
        false,
        FileType::File,
    )
    .unwrap();
    assert_eq!(results.len(), 4, "Expected 4 .rs files after excluding tests");

    // Directory search
    let results = search_paths(
        test_dir.path(),
        vec!["**/*.rs".to_string()],
        vec![],
        true,
        FileType::Dir,
    )
    .unwrap();
    assert!(!results.is_empty(), "Expected to find directories");
}

#[test]
fn test_edge_cases() {
    // Empty directory
    let test_dir = TestDirectory::new();
    let results = search_paths(
        test_dir.path(),
        vec!["**/*.rs".to_string()],
        vec![],
        false,
        FileType::File,
    )
    .unwrap();
    assert_eq!(results.len(), 0, "Expected no files in empty directory");

    // No matching files
    let test_dir = create_test_directory();
    let results = search_paths(
        test_dir.path(),
        vec!["**/*.xyz".to_string()],
        vec![],
        false,
        FileType::File,
    )
    .unwrap();
    assert_eq!(results.len(), 0, "Expected no files with non-existent extension");
}

#[test]
fn test_deterministic_results() {
    let test_dir = create_test_directory();

    let mut results = vec![];
    for _ in 0..3 {
        let result = search_paths(
            test_dir.path(),
            vec!["**/*.rs".to_string()],
            vec![],
            false,
            FileType::File,
        )
        .unwrap();
        results.push(normalize_paths(result));
    }

    // All runs should return identical results
    assert_eq!(results[0], results[1]);
    assert_eq!(results[1], results[2]);
}
