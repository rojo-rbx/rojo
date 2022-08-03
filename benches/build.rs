use std::path::Path;

use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use tempfile::{tempdir, TempDir};

use librojo::cli::BuildCommand;

pub fn benchmark_small_place(c: &mut Criterion) {
    bench_build_place(c, "Small Place", "test-projects/benchmark_small_place")
}

criterion_group!(benches, benchmark_small_place);
criterion_main!(benches);

fn bench_build_place(c: &mut Criterion, name: &str, path: &str) {
    let mut group = c.benchmark_group(name);

    // 'rojo build' generally takes a fair bit of time to execute.
    group.sample_size(10);
    group.bench_function("build", |b| {
        b.iter_batched(
            || place_setup(path),
            |(_dir, options)| options.run().unwrap(),
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

fn place_setup<P: AsRef<Path>>(input_path: P) -> (TempDir, BuildCommand) {
    let dir = tempdir().unwrap();
    let input = input_path.as_ref().to_path_buf();
    let output = dir.path().join("output.rbxlx");

    let options = BuildCommand {
        project: input,
        watch: false,
        output,
    };

    (dir, options)
}
