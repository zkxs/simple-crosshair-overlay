use criterion::{criterion_group, criterion_main};

use color_picker::*;
use icon::*;

mod color_picker;
mod icon;

criterion_group!(benches, bench_color_picker, bench_hsv_argb, bench_multiply_color_channel, bench_icon);
criterion_main!(benches);
