// This file is part of simple-crosshair-overlay and is licenced under the GNU GPL v3.0.
// See LICENSE file for full text.
// Copyright © 2023 Michael Ripley

//! Naive implementations of various functions that are less performant than their optimized
//! alternatives.
//!
//! These are retained for:
//!
//! 1. benchmarking comparisons
//! 2. unit testing known good output

use crate::util::image::hue_value_to_argb;

#[inline(always)]
pub fn draw_color_picker(buffer: &mut [u32]) {
    const EXPECTED_SIZE: usize = 256;
    const BUFFER_SIZE: usize = EXPECTED_SIZE * EXPECTED_SIZE;
    debug_assert_eq!(buffer.len(), BUFFER_SIZE, "draw_color_picker() passed buffer of wrong size");

    for y in 0..EXPECTED_SIZE {
        for x in 0..EXPECTED_SIZE {
            buffer[y * EXPECTED_SIZE + x] = hue_value_color_from_coordinates(x, y);
        }
    }
}

/// calculate an ARGB color from picked coordinates from a color picker.
/// this color does NOT have premultiplied alpha.
/// `x` and `y` must be within 0..255
fn hue_value_color_from_coordinates(x: usize, y: usize) -> u32 {
    hue_value_to_argb(x as u8, 255 - (y as u8))
}
