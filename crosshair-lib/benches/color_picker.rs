// This file is part of simple-crosshair-overlay and is licenced under the GNU GPL v3.0.
// See LICENSE file for full text.
// Copyright © 2023 Michael Ripley

//! Color picker benchmarks.

use criterion::{BatchSize, black_box, Criterion};

use crosshair_lib::util::image;

pub fn bench_color_picker(c: &mut Criterion) {
    let mut group = c.benchmark_group("Color Picker Implementations");

    group.bench_function("Naive", |bencher| {
        bencher.iter_batched_ref(|| vec![0; 256 * 256], |buffer| image::_draw_color_picker_naive(black_box(buffer.as_mut_slice())), BatchSize::SmallInput)
    });

    group.bench_function("Optimized", |bencher| {
        bencher.iter_batched_ref(|| vec![0; 252 * 252], |buffer| image::draw_color_picker_optimized(black_box(buffer.as_mut_slice())), BatchSize::SmallInput)
    });

    group.finish();
}

pub fn bench_hsv_argb(c: &mut Criterion) {
    let mut group = c.benchmark_group("HSV -> ARGB conversion implementations");

    group.bench_function("Precise HSV", |bencher| {
        bencher.iter(|| image::_hsv_to_argb_precise(black_box(0xFF), black_box(0xFF), black_box(0xFF)));
    });

    group.bench_function("Optimized HV", |bencher| {
        bencher.iter(|| image::hue_value_to_argb(black_box(0xFF), black_box(0xFF)));
    });

    group.bench_function("Optimized HA", |bencher| {
        bencher.iter(|| image::hue_alpha_to_argb(black_box(0xFF), black_box(0xFF)));
    });

    group.finish();
}

pub fn bench_multiply_color_channel(c: &mut Criterion) {
    let mut group = c.benchmark_group("Color channel multiply implementations");

    group.bench_function("Precise", |bencher| {
        bencher.iter(|| image::_multiply_color_channels_u8_precise(black_box(0xFF), black_box(0x7F)));
    });

    group.bench_function("Optimized", |bencher| {
        bencher.iter(|| image::multiply_color_channels_u8(black_box(0xFF), black_box(0x7F)));
    });

    group.finish();
}
