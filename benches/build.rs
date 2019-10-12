use std::path::{Path, PathBuf};

use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use tempfile::{tempdir, TempDir};

use librojo::commands::{build, BuildOptions};

fn place_setup<P: AsRef<Path>>(input_path: P) -> (TempDir, BuildOptions) {
    let dir = tempdir().unwrap();
    let input = input_path.as_ref().to_path_buf();
    let output_file = dir.path().join("output.rbxlx");

    let options = BuildOptions {
        fuzzy_project_path: input,
        output_file,
        output_kind: None,
    };

    (dir, options)
}

pub fn benchmark_small_place_0_6_0(c: &mut Criterion) {
    let mut group = c.benchmark_group("Small place");
    group.sample_size(10);
    group.bench_function("build", |b| {
        b.iter_batched(
            || place_setup("test-projects/benchmark_project_0.6.0"),
            |(_dir, options)| build(&options).unwrap(),
            BatchSize::SmallInput,
        )
    });
    group.finish();
}

criterion_group!(benches, benchmark_small_place_0_6_0);
criterion_main!(benches);
