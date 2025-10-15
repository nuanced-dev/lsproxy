use criterion::{black_box, criterion_group, criterion_main, Criterion};
use lsproxy::utils::file_utils::{search_paths, search_paths_sequential, FileType};
use std::path::Path;

/// Comprehensive benchmark for TypeScript, Go, and Rust files
fn benchmark_file_walk(c: &mut Criterion) {
    let mut group = c.benchmark_group("file_walk");

    // Configure for slower operations (300-600ms per iteration)
    // Reduce sample size to 10 for large directory benchmarks
    group.sample_size(10);
    // Set measurement time to 10s (plenty of headroom for 10 samples × 600ms = 6s)
    group.measurement_time(std::time::Duration::from_secs(10));

    // Get the benchmark path from environment variable or use current directory
    let benchmark_path = std::env::var("BENCH_PATH").unwrap_or_else(|_| ".".to_string());
    let search_path = Path::new(&benchmark_path);

    if !search_path.exists() {
        eprintln!("Benchmark path doesn't exist: {}", benchmark_path);
        return;
    }

    // TypeScript, Go, and Rust patterns
    let include_patterns = vec![
        // TypeScript & JavaScript
        "**/*.ts".to_string(),
        "**/*.tsx".to_string(),
        "**/*.js".to_string(),
        "**/*.jsx".to_string(),
        // Go
        "**/*.go".to_string(),
        // Rust
        "**/*.rs".to_string(),
    ];

    let exclude_patterns = vec![
        "**/node_modules/**".to_string(),
        "**/target/**".to_string(),
        "**/dist/**".to_string(),
        "**/build/**".to_string(),
        "**/.git/**".to_string(),
    ];

    group.bench_function("sequential", |b| {
        b.iter(|| {
            black_box(search_paths_sequential(
                search_path,
                include_patterns.clone(),
                exclude_patterns.clone(),
                true,
                FileType::File,
            ))
        })
    });

    group.bench_function("parallel", |b| {
        b.iter(|| {
            black_box(search_paths(
                search_path,
                include_patterns.clone(),
                exclude_patterns.clone(),
                true,
                FileType::File,
            ))
        })
    });

    group.finish();
}

criterion_group!(benches, benchmark_file_walk);
criterion_main!(benches);
