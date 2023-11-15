// This file is part of simple-crosshair-overlay and is licenced under the GNU GPL v3.0.
// See LICENSE file for full text.
// Copyright © 2023 Michael Ripley

//! Benchmarks for the application icon generation

use std::time::Duration;
use criterion::{BenchmarkId, Criterion};

use crosshair_lib::util::image;

pub fn bench_icon(c: &mut Criterion) {
    if cfg!(feature = "slow-benchmark") {
        let mut group = c.benchmark_group("Icon Implementations");
        group.measurement_time(Duration::from_secs(10));

        for size in [4, 8, 16, 24, 32, 48, 64, 128, 256, 512, 1024, 1536, 2048, 2560, 3072, 3584, 4096] {
            // it may be slightly misleading to call this implementation "Naive", as the only naive thing about it
            // is that it visits every pixel in the icon. The compiler should easily be able to
            // generate branchless, vectorized assembly for this.
            group.bench_with_input(BenchmarkId::new("Naive", size), &size, |bencher, size| {
                bencher.iter_with_large_drop(|| image::generate_icon_rgba(*size))
            });

            // A potentially faster implementation would be to start with a zeroed buffer and only visit
            // pixels inside the disc. This could be done using a variation of Bresenham's algorithm,
            // but it's questionable if this would actually save us any time, especially as it would be
            // more challenging for the compiler to vectorize (it could only vectorize per-row vs over
            // the entire buffer).

            // ... also this isn't worth spending time on, as we only go up to 64x64 presently which
            // takes all of 11 microseconds. And it's done once... at compile time. So it's NEVER done
            // at runtime. But hey, it's fun to think about.
        }

        group.finish();
    }
}
