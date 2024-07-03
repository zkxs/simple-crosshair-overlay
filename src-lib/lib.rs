// This file is part of simple-crosshair-overlay and is licenced under the GNU GPL v3.0.
// See LICENSE file for full text.
// Copyright © 2023 Michael Ripley

//! This library is used by the simple-crosshair-overlay application and is not intended for public
//! use. Due to limitations of criterion, I can only benchmark functions in the public library. Due
//! to limitations of crates.io, all used libraries must be published. The result is I'm forced to
//! publish my internal API publicly.
//!
//! **This library will not be following semantic-versioning** as again, it is not intended to be
//! public API.

pub mod private;
