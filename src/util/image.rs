// This file is part of simple-crosshair-overlay and is licenced under the GNU GPL v3.0.
// See LICENSE file for full text.
// Copyright © 2023 Michael Ripley

//! Image processing and color utilities

use std::{io, mem};
use std::fs::File;
use std::path::Path;

use png::ColorType;

use crate::util::numeric::DivCeil;

pub struct Image {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u32>,
}

// process each pixel in a RGBA png
#[inline(always)]
fn rgba_to_argb(rgba_color: u32) -> u32 {
    let [r, g, b, a] = rgba_color.to_le_bytes(); // BE RGBA == LE ABGR
    let argb_color = u32::from_le_bytes([b, g, r, a]); // BE ARGB == LE BGRA
    premultiply_alpha(argb_color)
}

#[inline(always)]
#[cfg(target_os = "windows")]
pub fn premultiply_alpha(argb_color: u32) -> u32 {
    let [b, g, r, a] = argb_color.to_le_bytes();

    const MAX_COLOR: u16 = 255;

    let b = b as u16;
    let g = g as u16;
    let r = r as u16;
    let alpha = a as u16; // we're reusing `a` later, so give alpha a special name

    let b = (b * alpha / MAX_COLOR) as u8;
    let g = (g * alpha / MAX_COLOR) as u8;
    let r = (r * alpha / MAX_COLOR) as u8;

    u32::from_le_bytes([b, g, r, a])
}

// no-op on non-windows as this is not needed
#[inline(always)]
#[cfg(not(target_os = "windows"))]
pub fn premultiply_alpha(argb_color: u32) -> u32 {
    argb_color
}

pub fn load_png(path: &Path) -> io::Result<Image> {
    let file = File::open(path)?;
    let decoder = png::Decoder::new(file);
    let mut reader = decoder.read_info()?;

    // make a buffer of the correct size to hold the reader's data, but as u32's instead of u8's
    const RATIO: usize = mem::size_of::<u32>() / mem::size_of::<u8>();
    let mut buf: Vec<u32> = Vec::with_capacity(reader.output_buffer_size().div_ceil_placeholder(RATIO));
    #[allow(clippy::uninit_vec)]
    unsafe {
        // there is no requirement I send a zeroed buffer to the PNG decoding library.
        buf.set_len(buf.capacity());
    }

    // a little check to make sure div_ceil isn't fucked up. Which it's definitely not, because I eyeballed it really sternly.
    debug_assert!(buf.len() * RATIO >= reader.output_buffer_size(), "buffer was unexpectedly not large enough for image decode");

    // I'm just transmuting color data between u32 and [u8; 4] packing. No risk.
    let aligned_buf: &mut [u8] = unsafe {
        if let ([], aligned, []) = buf.align_to_mut() {
            aligned
        } else {
            panic!("couldn't align u32 buf to u8")
        }
    };

    let info = reader.next_frame(aligned_buf)?;

    if info.color_type != ColorType::Rgba {
        Err(io::Error::new(io::ErrorKind::InvalidInput, format!("PNG was in {:?} format. Only {:?} format is supported. Please re-save your PNG in the required format.", info.color_type, ColorType::Rgba)))?;
    }

    // post-process buf
    buf.iter_mut()
        .for_each(|pixel| *pixel = rgba_to_argb(pixel.to_owned()));

    let image = Image {
        width: info.width,
        height: info.height,
        data: buf,
    };

    Ok(image)
}
