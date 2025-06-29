// This file is part of simple-crosshair-overlay and is licenced under the GNU GPL v3.0.
// See LICENSE file for full text.
// Copyright © 2023 Michael Ripley

//! Numeric utilities

use std::time::Duration;

pub fn fps_to_tick_interval(fps: u32) -> Duration {
    let millis = 1000.div_ceil_placeholder(fps);
    Duration::from_millis(millis as u64)
}

pub trait DivCeil {
    /// Intentionally _not_ named `div_ceil` to avoid name conflicts with an
    /// [unstable feature I can't use](https://github.com/rust-lang/rust/issues/88581). Thanks Rust.
    /// Very cool that **unstable** features can conflict with stable names and win.
    ///
    /// This does an integer ceiling division.
    fn div_ceil_placeholder(&self, rhs: Self) -> Self;
}

impl DivCeil for usize {
    // implementation comes from https://github.com/rust-lang/rust/pull/88582/files
    fn div_ceil_placeholder(&self, rhs: Self) -> Self {
        let quotient = self / rhs;
        let remainder = self % rhs;
        if remainder > 0 { quotient + 1 } else { quotient }
    }
}

impl DivCeil for u32 {
    // implementation comes from https://github.com/rust-lang/rust/pull/88582/files
    fn div_ceil_placeholder(&self, rhs: Self) -> Self {
        let quotient = self / rhs;
        let remainder = self % rhs;
        if remainder > 0 { quotient + 1 } else { quotient }
    }
}

impl DivCeil for u64 {
    // implementation comes from https://github.com/rust-lang/rust/pull/88582/files
    fn div_ceil_placeholder(&self, rhs: u64) -> u64 {
        let quotient = self / rhs;
        let remainder = self % rhs;
        if remainder > 0 { quotient + 1 } else { quotient }
    }
}

pub trait DivFloor {
    /// Intentionally _not_ named `div_floor` to avoid name conflicts with an
    /// [unstable feature I can't use](https://github.com/rust-lang/rust/issues/88581). Thanks Rust.
    /// Very cool that **unstable** features can conflict with stable names and win.
    ///
    /// This does an integer floor division.
    fn div_floor_placeholder(&self, rhs: Self) -> Self;
}

impl DivFloor for i32 {
    // implementation comes from https://github.com/rust-lang/rust/pull/88582/files
    fn div_floor_placeholder(&self, rhs: Self) -> Self {
        let d = self / rhs;
        let r = self % rhs;
        if (r > 0 && rhs < 0) || (r < 0 && rhs > 0) {
            d - 1
        } else {
            d
        }
    }
}

#[cfg(test)]
mod test_div_rounding {
    use super::*;

    /// this is obvious, but I included it for completeness with the following test
    #[test]
    fn positive_div_rounds_down() {
        assert_eq!(101 / 2, 50);
    }

    /// rust integer division always rounds towards zero, this test is just to document that because we actually care about rounding towards -Infinity for some pixel math
    #[test]
    fn negative_div_rounds_up() {
        assert_eq!(-101 / 2, -50);
    }

    #[test]
    fn div_ceil_usize_no_round() {
        assert_eq!(100usize.div_ceil_placeholder(2), 50);
    }

    #[test]
    fn div_ceil_u64_no_round() {
        assert_eq!(100u64.div_ceil_placeholder(2), 50);
    }

    #[test]
    fn div_ceil_usize_rounds_up() {
        assert_eq!(101usize.div_ceil_placeholder(2), 51);
    }

    #[test]
    fn div_ceil_u64_rounds_up() {
        assert_eq!(101u64.div_ceil_placeholder(2), 51);
    }

    #[test]
    fn positive_div_floor_rounds_down() {
        assert_eq!(101.div_floor_placeholder(2), 50);
    }

    #[test]
    fn positive_div_floor_no_round() {
        assert_eq!(100.div_floor_placeholder(2), 50);
    }

    #[test]
    fn negative_div_floor_rounds_down() {
        assert_eq!((-101).div_floor_placeholder(2), -51);
    }

    #[test]
    fn negative_div_floor_no_round() {
        assert_eq!((-100).div_floor_placeholder(2), -50);
    }
}
