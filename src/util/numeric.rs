// This file is part of simple-crosshair-overlay and is licenced under the GNU GPL v3.0.
// See LICENSE file for full text.
// Copyright © 2023 Michael Ripley

pub trait DivCeil {
    /// Intentionally _not_ named `div_ceil` to avoid name conflicts with an
    /// [unstable feature I can't use](https://github.com/rust-lang/rust/issues/88581). Thanks Rust.
    /// Very cool that **unstable** features can conflict with stable names and win.
    ///
    /// This does an integer ceiling division.
    fn div_ceil_placeholder(&self, rhs: Self) -> Self;
}

impl DivCeil for usize {
    fn div_ceil_placeholder(&self, rhs: usize) -> usize {
        let quotient = self / rhs;
        let remainder = self % rhs;
        if remainder > 0 {
            quotient + 1
        } else {
            quotient
        }
    }
}

impl DivCeil for u32 {
    fn div_ceil_placeholder(&self, rhs: u32) -> u32 {
        let quotient = self / rhs;
        let remainder = self % rhs;
        if remainder > 0 {
            quotient + 1
        } else {
            quotient
        }
    }
}

impl DivCeil for u64 {
    fn div_ceil_placeholder(&self, rhs: u64) -> u64 {
        let quotient = self / rhs;
        let remainder = self % rhs;
        if remainder > 0 {
            quotient + 1
        } else {
            quotient
        }
    }
}
