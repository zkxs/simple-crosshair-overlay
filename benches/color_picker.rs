use criterion::{criterion_group, criterion_main, Criterion, BatchSize, black_box};
use simple_crosshair_overlay::util::image;

pub fn bench_color_picker(c: &mut Criterion) {
    let mut group = c.benchmark_group("Color Picker Implementations");

    group.bench_function("Naive", |bencher| {
        bencher.iter_batched_ref(|| vec![0; 256 * 256], |buffer| image::draw_color_picker(black_box(buffer.as_mut_slice())), BatchSize::SmallInput)
    });

    group.bench_function("Optimized", |bencher| {
        bencher.iter_batched_ref(|| vec![0; 252 * 252], |buffer| image::_draw_color_picker_optimized(black_box(buffer.as_mut_slice())), BatchSize::SmallInput)
    });

    group.finish();
}

criterion_group!(benches, bench_color_picker);
criterion_main!(benches);
