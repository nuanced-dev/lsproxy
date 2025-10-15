use lsproxy::utils::file_utils::{search_paths_parallel, search_paths_sequential, FileType};
use std::collections::HashSet;
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

    fn create_dir(&self, path: &str) {
        let full_path = self.root.join(path);
        fs::create_dir_all(full_path).unwrap();
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

/// Helper to sort and compare PathBuf vectors
fn normalize_paths(mut paths: Vec<PathBuf>) -> Vec<PathBuf> {
    paths.sort();
    paths
}

#[test]
fn test_all_implementations_return_same_results_small() {
    let test_dir = create_small_test_directory();
    let include_patterns = vec!["**/*.rs".to_string()];
    let exclude_patterns: Vec<String> = vec![];

    let seq_results = search_paths_sequential(
        test_dir.path(),
        include_patterns.clone(),
        exclude_patterns.clone(),
        false,
        FileType::File,
    )
    .unwrap();

    let par_mutex_results = search_paths_parallel(
        test_dir.path(),
        include_patterns.clone(),
        exclude_patterns.clone(),
        false,
        FileType::File,
    )
    .unwrap();

    // Normalize (sort) all results for comparison
    let seq = normalize_paths(seq_results);
    let par_mutex = normalize_paths(par_mutex_results);

    assert_eq!(seq, par_mutex, "Sequential and parallel results differ");

    // Verify we found the expected files
    assert_eq!(seq.len(), 6, "Expected 6 .rs files");
}

#[test]
fn test_all_implementations_return_same_results_medium() {
    let test_dir = create_medium_test_directory();
    let include_patterns = vec!["**/*.rs".to_string()];
    let exclude_patterns: Vec<String> = vec![];

    let seq_results = search_paths_sequential(
        test_dir.path(),
        include_patterns.clone(),
        exclude_patterns.clone(),
        false,
        FileType::File,
    )
    .unwrap();

    let par_mutex_results = search_paths_parallel(
        test_dir.path(),
        include_patterns.clone(),
        exclude_patterns.clone(),
        false,
        FileType::File,
    )
    .unwrap();

    let seq = normalize_paths(seq_results);
    let par_mutex = normalize_paths(par_mutex_results);

    assert_eq!(seq, par_mutex);

    // Should find 20 modules * 5 files + 20 mod.rs + 30 tests = 150 files
    assert_eq!(seq.len(), 150, "Expected 150 .rs files");
}

#[test]
fn test_all_implementations_return_same_results_large() {
    let test_dir = create_large_test_directory();
    let include_patterns = vec!["**/*.rs".to_string()];
    let exclude_patterns: Vec<String> = vec![];

    let seq_results = search_paths_sequential(
        test_dir.path(),
        include_patterns.clone(),
        exclude_patterns.clone(),
        false,
        FileType::File,
    )
    .unwrap();

    let par_mutex_results = search_paths_parallel(
        test_dir.path(),
        include_patterns.clone(),
        exclude_patterns.clone(),
        false,
        FileType::File,
    )
    .unwrap();

    let seq = normalize_paths(seq_results);
    let par_mutex = normalize_paths(par_mutex_results);

    assert_eq!(seq, par_mutex);

    // Should find 50 * 10 * (1 mod.rs + 3 files) + 100 tests = 2100 files
    assert_eq!(seq.len(), 2100, "Expected 2100 .rs files");
}

#[test]
fn test_with_exclude_patterns() {
    let test_dir = create_medium_test_directory();
    let include_patterns = vec!["**/*.rs".to_string()];
    let exclude_patterns = vec!["**/tests/**".to_string()];

    let seq_results = search_paths_sequential(
        test_dir.path(),
        include_patterns.clone(),
        exclude_patterns.clone(),
        false,
        FileType::File,
    )
    .unwrap();

    let par_results = search_paths_parallel(
        test_dir.path(),
        include_patterns.clone(),
        exclude_patterns.clone(),
        false,
        FileType::File,
    )
    .unwrap();

    let seq = normalize_paths(seq_results);
    let par = normalize_paths(par_results);

    assert_eq!(seq, par);

    // Should exclude 30 test files, leaving 120
    assert_eq!(
        seq.len(),
        120,
        "Expected 120 .rs files after excluding tests"
    );
}

#[test]
fn test_multiple_include_patterns() {
    let test_dir = create_small_test_directory();
    let include_patterns = vec!["**/*.rs".to_string(), "**/*.toml".to_string()];
    let exclude_patterns: Vec<String> = vec![];

    let seq_results = search_paths_sequential(
        test_dir.path(),
        include_patterns.clone(),
        exclude_patterns.clone(),
        false,
        FileType::File,
    )
    .unwrap();

    let par_results = search_paths_parallel(
        test_dir.path(),
        include_patterns.clone(),
        exclude_patterns.clone(),
        false,
        FileType::File,
    )
    .unwrap();

    let seq = normalize_paths(seq_results);
    let par = normalize_paths(par_results);

    assert_eq!(seq, par);

    // Should find 6 .rs files + 1 .toml file = 7 files
    assert_eq!(seq.len(), 7, "Expected 7 files (.rs and .toml)");
}

#[test]
fn test_directories_all_implementations_match() {
    let test_dir = create_medium_test_directory();
    let include_patterns = vec!["**/*.rs".to_string()];
    let exclude_patterns: Vec<String> = vec![];

    let seq_results = search_paths_sequential(
        test_dir.path(),
        include_patterns.clone(),
        exclude_patterns.clone(),
        true,
        FileType::Dir,
    )
    .unwrap();

    let par_mutex_results = search_paths_parallel(
        test_dir.path(),
        include_patterns.clone(),
        exclude_patterns.clone(),
        true,
        FileType::Dir,
    )
    .unwrap();

    // Convert to HashSets for comparison (order doesn't matter for directories)
    let seq_set: HashSet<_> = seq_results.into_iter().collect();
    let par_mutex_set: HashSet<_> = par_mutex_results.into_iter().collect();

    assert_eq!(seq_set, par_mutex_set);
}

#[test]
fn test_empty_directory() {
    let test_dir = TestDirectory::new();
    let include_patterns = vec!["**/*.rs".to_string()];
    let exclude_patterns: Vec<String> = vec![];

    let seq_results = search_paths_sequential(
        test_dir.path(),
        include_patterns.clone(),
        exclude_patterns.clone(),
        false,
        FileType::File,
    )
    .unwrap();

    let par_results = search_paths_parallel(
        test_dir.path(),
        include_patterns.clone(),
        exclude_patterns.clone(),
        false,
        FileType::File,
    )
    .unwrap();

    assert_eq!(seq_results.len(), 0);
    assert_eq!(par_results.len(), 0);
}

#[test]
fn test_no_matching_files() {
    let test_dir = create_small_test_directory();
    let include_patterns = vec!["**/*.xyz".to_string()]; // Non-existent extension
    let exclude_patterns: Vec<String> = vec![];

    let seq_results = search_paths_sequential(
        test_dir.path(),
        include_patterns.clone(),
        exclude_patterns.clone(),
        false,
        FileType::File,
    )
    .unwrap();

    let par_results = search_paths_parallel(
        test_dir.path(),
        include_patterns.clone(),
        exclude_patterns.clone(),
        false,
        FileType::File,
    )
    .unwrap();

    assert_eq!(seq_results.len(), 0);
    assert_eq!(par_results.len(), 0);
}

/// Performance characteristic test: Parallel should handle large directories reasonably
#[test]
fn test_parallel_handles_large_directory() {
    use std::time::Instant;

    let test_dir = create_large_test_directory();
    let include_patterns = vec!["**/*.rs".to_string()];
    let exclude_patterns: Vec<String> = vec![];

    let start = Instant::now();
    let par_results = search_paths_parallel(
        test_dir.path(),
        include_patterns.clone(),
        exclude_patterns.clone(),
        false,
        FileType::File,
    )
    .unwrap();
    let par_duration = start.elapsed();

    // Just verify it completes and returns correct count
    assert_eq!(par_results.len(), 2100);

    // Sanity check: should complete within reasonable time (< 5 seconds)
    assert!(
        par_duration.as_secs() < 5,
        "Parallel search took too long: {:?}",
        par_duration
    );
}

/// Performance characteristic test: Sequential should be fast on small directories
#[test]
fn test_sequential_fast_on_small() {
    use std::time::Instant;

    let test_dir = create_small_test_directory();
    let include_patterns = vec!["**/*.rs".to_string()];
    let exclude_patterns: Vec<String> = vec![];

    let start = Instant::now();
    let seq_results = search_paths_sequential(
        test_dir.path(),
        include_patterns.clone(),
        exclude_patterns.clone(),
        false,
        FileType::File,
    )
    .unwrap();
    let seq_duration = start.elapsed();

    assert_eq!(seq_results.len(), 6);

    // Should be very fast on small directories (< 100ms)
    assert!(
        seq_duration.as_millis() < 100,
        "Sequential search took too long: {:?}",
        seq_duration
    );
}

/// Consistency test: Run multiple times to ensure deterministic results
#[test]
fn test_deterministic_results() {
    let test_dir = create_medium_test_directory();
    let include_patterns = vec!["**/*.rs".to_string()];
    let exclude_patterns: Vec<String> = vec![];

    let mut results = vec![];
    for _ in 0..3 {
        let result = search_paths_parallel(
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
