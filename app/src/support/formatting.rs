// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Human-readable formatting shared by the views.
//!
//! These are presentation concerns, not domain ones, so they live beside the
//! UI instead of in a crate. `bytes` is used by `multiplayer`; `launch` still
//! carries its own `format_bytes` copy. Nothing else has been duplicated enough
//! to earn a place here.

/// Human-readable byte sizes: binary units, two decimals, and a bare `B` below a
/// kibibyte.
pub(crate) fn bytes(value: u64) -> String {
    const UNITS: [&str; 4] = ["KB", "MB", "GB", "TB"];
    if value < 1024 {
        return format!("{value} B");
    }
    let mut scaled = value as f64 / 1024.0;
    let mut unit = 0;
    while scaled >= 1024.0 && unit < UNITS.len() - 1 {
        scaled /= 1024.0;
        unit += 1;
    }
    format!("{scaled:.2} {}", UNITS[unit])
}

#[cfg(test)]
mod tests {
    use super::bytes;

    #[test]
    fn below_a_kibibyte_is_plain_bytes() {
        assert_eq!(bytes(0), "0 B");
        assert_eq!(bytes(1023), "1023 B");
    }

    #[test]
    fn scales_through_the_binary_units() {
        assert_eq!(bytes(1024), "1.00 KB");
        assert_eq!(bytes(1024 * 1024), "1.00 MB");
        assert_eq!(bytes(1024 * 1024 * 1024), "1.00 GB");
        assert_eq!(bytes(1024_u64.pow(4)), "1.00 TB");
    }

    #[test]
    fn stops_at_the_largest_unit() {
        assert_eq!(bytes(1024_u64.pow(5)), "1024.00 TB");
    }
}
