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

/// Create a small test directory (< 100 files)
fn create_small_test_directory() -> TestDirectory {
    let test_dir = TestDirectory::new();

    // Create a simple structure
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

/// Create a medium test directory (100-500 files)
fn create_medium_test_directory() -> TestDirectory {
    let test_dir = TestDirectory::new();

    // Create multiple modules
    for i in 0..20 {
        test_dir.create_file(&format!("src/module_{}/mod.rs", i));
        for j in 0..5 {
            test_dir.create_file(&format!("src/module_{}/file_{}.rs", i, j));
        }
    }

    // Add some tests
    for i in 0..30 {
        test_dir.create_file(&format!("tests/test_{}.rs", i));
    }

    // Add config files
    test_dir.create_file("Cargo.toml");
    test_dir.create_file("README.md");
    test_dir.create_file(".gitignore");

    test_dir
}

/// Create a large test directory (> 500 files)
fn create_large_test_directory() -> TestDirectory {
    let test_dir = TestDirectory::new();

    // Create many modules with deep nesting
    for i in 0..50 {
        for j in 0..10 {
            test_dir.create_file(&format!("src/module_{}/submodule_{}/mod.rs", i, j));
            for k in 0..3 {
                test_dir.create_file(&format!("src/module_{}/submodule_{}/file_{}.rs", i, j, k));
            }
        }
    }

    // Add tests
    for i in 0..100 {
        test_dir.create_file(&format!("tests/integration/test_{}.rs", i));
    }

    test_dir
}

/// Helper to sort PathBuf vectors
fn normalize_paths(mut paths: Vec<PathBuf>) -> Vec<PathBuf> {
    paths.sort();
    paths
}

#[test]
fn test_search_files_counts() {
    // Small directory (< 100 files)
    let test_dir = create_small_test_directory();
    let results = search_paths(
        test_dir.path(),
        vec!["**/*.rs".to_string()],
        vec![],
        false,
        FileType::File,
    )
    .unwrap();
    assert_eq!(results.len(), 6, "Expected 6 .rs files in small directory");

    // Medium directory (100-500 files)
    let test_dir = create_medium_test_directory();
    let results = search_paths(
        test_dir.path(),
        vec!["**/*.rs".to_string()],
        vec![],
        false,
        FileType::File,
    )
    .unwrap();
    // 20 modules * 5 files + 20 mod.rs + 30 tests = 150 files
    assert_eq!(results.len(), 150, "Expected 150 .rs files in medium directory");

    // Large directory (> 500 files) with performance check
    let test_dir = create_large_test_directory();
    let start = std::time::Instant::now();
    let results = search_paths(
        test_dir.path(),
        vec!["**/*.rs".to_string()],
        vec![],
        false,
        FileType::File,
    )
    .unwrap();
    let duration = start.elapsed();
    // 50 * 10 * (1 mod.rs + 3 files) + 100 tests = 2100 files
    assert_eq!(results.len(), 2100, "Expected 2100 .rs files in large directory");
    // Sanity check: should complete within reasonable time (< 5 seconds)
    assert!(
        duration.as_secs() < 5,
        "Search took too long: {:?}",
        duration
    );
}

#[test]
fn test_pattern_features() {
    let test_dir = create_medium_test_directory();

    // Exclude patterns
    let results = search_paths(
        test_dir.path(),
        vec!["**/*.rs".to_string()],
        vec!["**/tests/**".to_string()],
        false,
        FileType::File,
    )
    .unwrap();
    // Should exclude 30 test files, leaving 120
    assert_eq!(
        results.len(),
        120,
        "Expected 120 .rs files after excluding tests"
    );

    // Multiple include patterns
    let test_dir = create_small_test_directory();
    let results = search_paths(
        test_dir.path(),
        vec!["**/*.rs".to_string(), "**/*.toml".to_string()],
        vec![],
        false,
        FileType::File,
    )
    .unwrap();
    // Should find 6 .rs files + 1 .toml file = 7 files
    assert_eq!(results.len(), 7, "Expected 7 files (.rs and .toml)");
}

#[test]
fn test_file_type_directory() {
    let test_dir = create_medium_test_directory();
    let results = search_paths(
        test_dir.path(),
        vec!["**/*.rs".to_string()],
        vec![],
        true,
        FileType::Dir,
    )
    .unwrap();
    // Should find directories containing .rs files
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
    let test_dir = create_small_test_directory();
    let results = search_paths(
        test_dir.path(),
        vec!["**/*.xyz".to_string()], // Non-existent extension
        vec![],
        false,
        FileType::File,
    )
    .unwrap();
    assert_eq!(results.len(), 0, "Expected no files with non-existent extension");
}

#[test]
fn test_deterministic_results() {
    let test_dir = create_medium_test_directory();
    let include_patterns = vec!["**/*.rs".to_string()];
    let exclude_patterns: Vec<String> = vec![];

    let mut results = vec![];
    for _ in 0..3 {
        let result = search_paths(
            test_dir.path(),
            include_patterns.clone(),
            exclude_patterns.clone(),
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
