// This file is part of simple-crosshair-overlay and is licenced under the GNU GPL v3.0.
// See LICENSE file for full text.
// Copyright © 2023 Michael Ripley

//! Benchmarks for various functions

use criterion::{criterion_group, criterion_main};

use color_picker::*;
use hotkey::*;

mod color_picker;
mod hotkey;

criterion_group!(benches, bench_color_picker, bench_hsv_argb, bench_multiply_color_channel, bench_key_poll, bench_key_process);
criterion_main!(benches);
