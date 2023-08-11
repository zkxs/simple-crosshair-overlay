// This file is part of simple-crosshair-overlay and is licenced under the GNU GPL v3.0.
// See LICENSE file for full text.
// Copyright © 2023 Michael Ripley

//! Image processing and color utilities

use std::{io, mem};
use std::fs::File;
use std::path::Path;

use png::ColorType;

use crate::util::numeric::{DivCeil, DivFloor};

/// in-memory image representation
pub struct Image {
    /// image width
    pub width: u32,
    /// image height
    pub height: u32,
    /// ARGB pixel color data
    pub data: Vec<u32>,
}

/// calculate an ARGB color from picked coordinates from a color picker
/// this color does NOT have premultiplied alpha
pub fn hue_value_color_from_coordinates(x: usize, y: usize, width: usize, height: usize) -> u32 {
    const EXPECTED_SIZE: usize = 256;
    debug_assert_eq!(width, EXPECTED_SIZE);
    debug_assert_eq!(height, EXPECTED_SIZE);

    hue_value_to_argb(x as u8, 255 - (y as u8))
}

/// calculate an ARGB color from picked coordinates from a color picker
/// this color does NOT have premultiplied alpha
pub fn hue_alpha_color_from_coordinates(x: usize, y: usize, width: usize, height: usize) -> u32 {
    const EXPECTED_SIZE: usize = 256;
    debug_assert_eq!(width, EXPECTED_SIZE);
    debug_assert_eq!(height, EXPECTED_SIZE);

    hue_alpha_to_argb(x as u8, 255 - (y as u8))
}

/// see https://en.wikipedia.org/wiki/HSL_and_HSV#Color_conversion_formulae
/// this is a HSV -> RGB conversion, except S is always set to 100%, which simplifies things
fn hue_value_to_argb(hue: u8, value: u8) -> u32 {
    const MAX_COLOR: u8 = 255;
    // we need the ceiling of each of the 5 boundaries between the 6 sections
    const SECTION_1: u8 = 43; // 256/6*1 = 42.667
    const SECTION_2: u8 = 86; // 256/6*2 = 85.333
    const SECTION_3: u8 = 128; // 256/6*3 = 128.000
    const SECTION_4: u8 = 171; // 256/6*4 = 170.667
    const SECTION_5: u8 = 214; // 256/6*5 = 213.333

    // convert the hue into a nice sawtooth line going from 0->255 in each of the 6 sections
    let raw_hue = hue.wrapping_mul(6);

    let [r, g, b] = match hue {
        hue if hue < SECTION_1 => [value, multiply_color_channels_u8(raw_hue, value), 0],
        hue if hue < SECTION_2 => [multiply_color_channels_u8(MAX_COLOR - raw_hue, value), value, 0],
        hue if hue < SECTION_3 => [0, value, multiply_color_channels_u8(raw_hue, value)],
        hue if hue < SECTION_4 => [0, multiply_color_channels_u8(MAX_COLOR - raw_hue, value), value],
        hue if hue < SECTION_5 => [multiply_color_channels_u8(raw_hue, value), 0, value],
        _ => [value, 0, multiply_color_channels_u8(MAX_COLOR - raw_hue, value)],
    };

    u32::from_le_bytes([b, g, r, MAX_COLOR])
}

/// this is a HSV -> RGB conversion, except S and V are always set to 100%, which simplifies things
fn hue_alpha_to_argb(hue: u8, alpha: u8) -> u32 {
    const MAX_COLOR: u8 = 255;
    // we need the ceiling of each of the 5 boundaries between the 6 sections
    const SECTION_1: u8 = 43; // 256/6*1 = 42.667
    const SECTION_2: u8 = 86; // 256/6*2 = 85.333
    const SECTION_3: u8 = 128; // 256/6*3 = 128.000
    const SECTION_4: u8 = 171; // 256/6*4 = 170.667
    const SECTION_5: u8 = 214; // 256/6*5 = 213.333

    // convert the hue into a nice sawtooth line going from 0->255 in each of the 6 sections
    let raw_hue = hue.wrapping_mul(6);

    let [r, g, b] = match hue {
        hue if hue < SECTION_1 => [MAX_COLOR, raw_hue, 0],
        hue if hue < SECTION_2 => [MAX_COLOR - raw_hue, MAX_COLOR, 0],
        hue if hue < SECTION_3 => [0, MAX_COLOR - raw_hue, raw_hue],
        hue if hue < SECTION_4 => [0, MAX_COLOR - raw_hue, MAX_COLOR],
        hue if hue < SECTION_5 => [raw_hue, 0, MAX_COLOR],
        _ => [MAX_COLOR, 0, MAX_COLOR - raw_hue],
    };

    u32::from_le_bytes([b, g, r, alpha])
}

/// Convert BE RGBA to LE ARGB, premultiplying alpha where required by the target platform.
#[inline(always)]
#[cfg(target_os = "windows")]
fn rgba_to_argb(rgba_color: u32) -> u32 {
    // OPTIMIZATION NOTE: this could benefit from SIMD. However, it only happens when the user loads
    // a PNG from disk. So not only is this infrequent, the latency of doing all the number crunching
    // is going to be completely overshadowed by the incredible slowness of reading from disk. Not
    // worth shaving microseconds off a millisecond-latency operation.

    // The PNG data is currently laid out as RGBA in BE order.
    // From a LE perspective, this means the actual data in the u32 is ABGR
    // Therefore, if we read this in LE order the bytes go RGBA.
    let [r, g, b, a] = rgba_color.to_le_bytes();

    // We want to pack the data back into ARGB. Provided in LE order that's BGRA.
    u32::from_le_bytes(
        [
            multiply_color_channels_u8(b, a),
            multiply_color_channels_u8(g, a),
            multiply_color_channels_u8(r, a),
            a
        ]
    )
}

/// Convert BE RGBA to LE ARGB, premultiplying alpha where required by the target platform.
#[inline(always)]
#[cfg(not(target_os = "windows"))]
fn rgba_to_argb(rgba_color: u32) -> u32 {
    // The PNG data is currently laid out as RGBA in BE order.
    // From a LE perspective, this means the actual data in the u32 is ABGR
    // Therefore, if we read this in LE order the bytes go RGBA.
    let [r, g, b, a] = rgba_color.to_le_bytes();

    // We want to pack the data back into ARGB. Provided in LE order that's BGRA.
    u32::from_le_bytes([b, g, r, a])
}

/// Premultiply alpha if required by current platform. On this platform this performs the premultiplication.
#[cfg(target_os = "windows")]
pub fn premultiply_alpha(color: u32) -> u32 {
    let [b, g, r, a] = color.to_le_bytes();
    u32::from_le_bytes(
        [
            multiply_color_channels_u8(b, a),
            multiply_color_channels_u8(g, a),
            multiply_color_channels_u8(r, a),
            a
        ]
    )
}

/// Premultiply alpha if required by current platform. On this platform this is a no-op.
#[cfg(not(target_os = "windows"))]
pub fn premultiply_alpha(color: u32) -> u32 {
    color
}

/// calculates `a * b / 255`
///
/// Note that this cannot be done with u8 precision alone, an intermediate step in the math can be
/// up to 255 * 255 == 65025 inclusive. Example code on how to do this conversion casts to floats
/// for the intermediate step, but that seems excessive when a u16 would do perfectly well and will
/// even truncate towards zero just like a float -> u8 conversion. It's possible that using a wider
/// type (like u32) might give more optimal assembly, but that's really the compiler's problem to
/// worry about.
///
/// - "Integer division rounds towards zero" [source](https://doc.rust-lang.org/reference/expressions/operator-expr.html#arithmetic-and-logical-binary-operators)
/// - "Casting from a float to an integer will round the float towards zero" [source](https://doc.rust-lang.org/reference/expressions/operator-expr.html#numeric-cast)
///
/// Finally, we can round to nearest int by simply adding 255 / 2 ~= 127 to the dividend
#[inline(always)]
fn multiply_color_channels_u8(a: u8, b: u8) -> u8 {
    const MAX_COLOR: u16 = 255;
    const HALF_COLOR: u16 = 127;

    ((a as u16 * b as u16 + HALF_COLOR) / MAX_COLOR) as u8
}

/// load a png file into an in-memory image
pub fn load_png(path: &Path) -> io::Result<Box<Image>> {
    let file = File::open(path)?;
    let decoder = png::Decoder::new(file);
    let mut reader = decoder.read_info()?;

    // The PNG decoder wants a u8 buffer to store its RGBA data... but winit wants ARGB u32 data.
    // Here I make a buffer of the correct size to hold the reader's data, but as u32's instead of u8's.
    // This is done because it's not safe to cast a &[u8] into a &[u32] due to possible u32 misalignment,
    // however it is completely safe to cast a &[u32] into a &[u8].
    const RATIO: usize = mem::size_of::<u32>() / mem::size_of::<u8>(); // this is going to be 4 always, but it's good practice to not use a magic number here
    let mut buf_as_u32: Vec<u32> = Vec::with_capacity(reader.output_buffer_size().div_ceil_placeholder(RATIO));
    #[allow(clippy::uninit_vec)]
    unsafe {
        // there is no requirement I send a zeroed buffer to the PNG decoding library.
        buf_as_u32.set_len(buf_as_u32.capacity());
    }

    // a little check to make sure div_ceil isn't fucked up. Which it's definitely not, because I eyeballed it really sternly.
    debug_assert!(buf_as_u32.len() * RATIO >= reader.output_buffer_size(), "buffer was unexpectedly not large enough for image decode");

    // I'm just transmuting color data between u32 and [u8; 4] packing. No risk.
    let buf_as_u8: &mut [u8] = unsafe {
        if let ([], aligned, []) = buf_as_u32.align_to_mut() {
            aligned
        } else {
            panic!("couldn't align u32 buf to u8")
        }
    };

    let info = reader.next_frame(buf_as_u8)?;

    if info.color_type != ColorType::Rgba {
        Err(io::Error::new(io::ErrorKind::InvalidInput, format!("PNG was in {:?} format. Only {:?} format is supported. Please re-save your PNG in the required format.", info.color_type, ColorType::Rgba)))?;
    }

    // post-process color layout in each pixel
    buf_as_u32.iter_mut()
        .for_each(|pixel| *pixel = rgba_to_argb(pixel.to_owned()));

    let image = Image {
        width: info.width,
        height: info.height,
        data: buf_as_u32,
    };

    Ok(Box::new(image))
}

/// calculate the coordinates of the center of a rectangle.
/// `x` and `y` are the coordinates of the top left corner.
/// `width` and `height` are the dimensions of the rectangle.
/// Rounding is done towards -Infinity.
/// I haven't thought about what happens if `width` or `height` are negative, so you'd better keep them positive.
#[inline(always)]
pub fn rectangle_center(x: i32, y: i32, width: i32, height: i32) -> (i32, i32) {
    (
        x + width.div_floor_placeholder(2),
        y + height.div_floor_placeholder(2)
    )
}

#[cfg(test)]
mod test_pixel_format {
    use super::*;

    /// simply confirm that to_le_bytes does what I expect, as the documentation is slightly vague
    #[test]
    fn test_le() {
        let b0 = 0u8;
        let b1 = 1u8;
        let b2 = 2u8;
        let b3 = 3u8;

        let u0 = b0 as u32;
        let u1 = b1 as u32;
        let u2 = b2 as u32;
        let u3 = b3 as u32;

        // a u32 made up of [b3, b2, b1, b0]
        let packed_u32 = (u3 << 24) + (u2 << 16) + (u1 << 8) + u0;

        let bytes = packed_u32.to_le_bytes();
        assert_eq!(&bytes, &[b0, b1, b2, b3]);
    }

    #[test]
    fn test_pixel_format_conversion() {
        let alpha = 255u8;
        let red = 20u8;
        let green = 40u8;
        let blue = 60u8;
        let png_data = u32::from_le_bytes([red, green, blue, alpha]); // laid out backwards in memory, so we write it forwards in LE
        let argb_data = rgba_to_argb(png_data);
        assert_eq!(argb_data.to_le_bytes(), [blue, green, red, alpha]); // laid out properly in memory, so we write it backwards in LE
    }

    /// This should be a no-op.
    #[test]
    fn test_premultiply_alpha_noop() {
        assert_eq!(multiply_color_channels_u8(255, 255), 255);
        assert_eq!(multiply_color_channels_u8(127, 255), 127);
        assert_eq!(multiply_color_channels_u8(0, 255), 0);
    }

    /// This should half the value of each color.
    #[test]
    fn test_premultiply_alpha_half() {
        assert_eq!(multiply_color_channels_u8(255, 127), 127);
        assert_eq!(multiply_color_channels_u8(127, 127), 63);
        assert_eq!(multiply_color_channels_u8(0, 127), 0);
    }

    /// This should zero all the color data.
    #[test]
    fn test_premultiply_alpha_zero() {
        assert_eq!(multiply_color_channels_u8(255, 0), 0);
        assert_eq!(multiply_color_channels_u8(127, 0), 0);
        assert_eq!(multiply_color_channels_u8(0, 0), 0);
    }

    /// alpha premultiply implemented with f64 precision and rounding to nearest int
    fn premultiply_alpha_precise_u8(c: u8, a: u8) -> u8 {
        (c as f64 * a as f64 / 255f64).round() as u8
    }

    /// make sure our alpha premultiplication always rounds to the nearest u8
    #[test]
    fn premultiply_alpha_rounding() {
        // test for some every `c` for various predefined `a`
        // what's important here is to contrive c*a/255 for results that will round in different ways while avoiding an exhaustive test, as that'd be slow
        for c in 0..=255 {
            for a in [0, 1, 127, 128, 254, 255] {
                let precise_result = premultiply_alpha_precise_u8(c, a);
                let actual_result = multiply_color_channels_u8(c, a);
                assert_eq!(actual_result, precise_result, "mismatch for c={c} a={a}")
            }
        }
    }
}

#[cfg(test)]
mod test_rectangle_center {
    use super::*;

    #[test]
    fn test_rectangle_center_0_corner() {
        assert_eq!(rectangle_center(0, 0, 100, 100), (50, 50));
    }

    #[test]
    fn test_rectangle_center_0_corner_odd_size() {
        assert_eq!(rectangle_center(0, 0, 101, 101), (50, 50));
    }

    #[test]
    fn test_rectangle_center_even_corner() {
        assert_eq!(rectangle_center(2, 2, 96, 96), (50, 50));
    }

    #[test]
    fn test_rectangle_center_even_corner_odd_size() {
        assert_eq!(rectangle_center(2, 2, 97, 97), (50, 50));
    }

    #[test]
    fn test_rectangle_center_negative_corner() {
        assert_eq!(rectangle_center(-2, -2, 104, 104), (50, 50));
    }

    #[test]
    fn test_rectangle_center_negative_corner_odd_size() {
        assert_eq!(rectangle_center(-2, -2, 105, 105), (50, 50));
    }

    /// my actual 1080p monitor setup
    #[test]
    fn test_1080p_top_centered() {
        assert_eq!(rectangle_center(397, -1080, 1920, 1080), (397 + 960, -1080 + 540));
    }
}
