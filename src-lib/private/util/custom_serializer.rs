// This file is part of simple-crosshair-overlay and is licenced under the GNU GPL v3.0.
// See LICENSE file for full text.
// Copyright © 2023 Michael Ripley

/// Serialize a u32-packed ARGB color as a hex string, because editing a decimal u32 by hand is fucked.
pub mod argb_color {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(color: &u32, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{color:08X}"))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<u32, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        u32::from_str_radix(&s, 16).map_err(serde::de::Error::custom)
    }
}
