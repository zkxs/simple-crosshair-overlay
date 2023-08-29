// This file is part of simple-crosshair-overlay and is licenced under the GNU GPL v3.0.
// See LICENSE file for full text.
// Copyright © 2023 Michael Ripley

//! Benchmarks for the hotkey manager

use std::hint::black_box;
use std::time::{Duration, Instant};

use criterion::Criterion;

use crosshair_lib::hotkey::KeyBindings;
use crosshair_lib::platform;
use crosshair_lib::platform::KeyboardState;

pub fn bench_key_poll(c: &mut Criterion) {
    let mut group = c.benchmark_group("Key poll");

    let mut keyboard_state = platform::generic::DeviceQueryKeyboardState::default();
    group.bench_function("device_query", |bencher| {
        bencher.iter(|| keyboard_state.poll())
    });

    let mut keyboard_state = platform::windows::WinApiKeyboardState::default();
    group.bench_function("winapi", |bencher| {
        bencher.iter(|| keyboard_state.poll());
    });

    group.finish();
}

pub fn bench_key_process(c: &mut Criterion) {
    let mut group = c.benchmark_group("Key process");

    let mut hotkey_manager = platform::generic::HotkeyManager::new(&KeyBindings::default()).unwrap();

    group.bench_function("bitmask", |bencher| {
        bencher.iter_custom(|iters| {
            let mut duration = Duration::ZERO;
            for _i in 0..iters {
                hotkey_manager.poll_keys();
                let start = Instant::now();
                platform::generic::HotkeyManager::process_keys(black_box(&mut hotkey_manager));
                duration += start.elapsed();
            }
            duration
        });
    });

    group.finish();
}
