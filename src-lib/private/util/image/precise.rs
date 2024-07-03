// This file is part of simple-crosshair-overlay and is licenced under the GNU GPL v3.0.
// See LICENSE file for full text.
// Copyright © 2023 Michael Ripley

//! Precise implementations of various functions that are MUCH less performant than their optimized
//! alternatives. The theme here is to use floating point numbers instead over worrying about
//! implementing proper integer math rounding.
//!
//! These are retained for:
//!
//! 1. benchmarking comparisons
//! 2. unit testing known good output

/// see https://en.wikipedia.org/wiki/HSL_and_HSV#Color_conversion_formulae
/// this is a HSV -> RGB conversion
pub fn hsv_to_argb(hue: u8, saturation: u8, value: u8) -> u32 {
    const HUE_RATIO: f64 = 360.0 / 255.0;
    let hue = hue as f64 * HUE_RATIO;
    let saturation = saturation as f64 / 255.0;
    let value = value as f64 / 255.0;

    let hue_over_60 = hue / 60.0;
    let chroma = value * saturation;
    let intermediate_color = chroma * (1.0 - (hue_over_60 % 2.0 - 1.0).abs());

    let [r, g, b] = match hue_over_60 {
        h if h < 1.0 => [chroma, intermediate_color, 0.0],
        h if h < 2.0 => [intermediate_color, chroma, 0.0],
        h if h < 3.0 => [0.0, chroma, intermediate_color],
        h if h < 4.0 => [0.0, intermediate_color, chroma],
        h if h < 5.0 => [intermediate_color, 0.0, chroma],
        _ => [chroma, 0.0, intermediate_color],
    };

    let r = (r * 255.0).round() as u8;
    let g = (g * 255.0).round() as u8;
    let b = (b * 255.0).round() as u8;

    u32::from_le_bytes([b, g, r, 255])
}

/// alpha premultiply implemented with f64 precision and rounding to nearest int
pub fn multiply_color_channels_u8(c: u8, a: u8) -> u8 {
    (c as f64 * a as f64 / 255f64).round() as u8
}
