// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The room-code format check.
//!
//! The Conic Nexus library exposes `conic_nexus_room_code_is_valid`, but the UI
//! needs the check before a session exists, so the same algorithm lives here:
//! a `U/XXXX-XXXX-XXXX-XXXX` shape over the 34-character alphabet and a modulo-7
//! checksum over the little-endian digit value of the four groups.

/// The alphabet's value for a code character, or `None` when it is not part of
/// it. `0`–`9`, `A`–`H`, `J`–`N` and `P`–`Z` skip `I` and `O`.
fn char_value(ch: char) -> Option<u32> {
    match ch {
        '0'..='9' => Some(ch as u32 - '0' as u32),
        'A'..='H' => Some(10 + ch as u32 - 'A' as u32),
        'J'..='N' => Some(18 + ch as u32 - 'J' as u32),
        'P'..='Z' => Some(23 + ch as u32 - 'P' as u32),
        _ => None,
    }
}

/// Whether `input` has the room-code shape and a valid checksum.
pub fn is_room_code_valid(input: &str) -> bool {
    let chars: Vec<char> = input.chars().collect();
    if chars.len() != 21 {
        return false;
    }
    if chars[0] != 'U' || chars[1] != '/' {
        return false;
    }
    // The three separators sit between the four groups.
    if chars[6] != '-' || chars[11] != '-' || chars[16] != '-' {
        return false;
    }

    let mut value: u128 = 0;
    let mut base: u128 = 1;
    // The digits are the four groups with the `U/` prefix and the three
    // separators dropped, concatenated in the order they are written.
    for (index, ch) in chars.iter().enumerate().skip(2) {
        if index == 6 || index == 11 || index == 16 {
            continue;
        }
        let Some(digit) = char_value(*ch) else {
            return false;
        };
        value += digit as u128 * base;
        base *= 34;
    }
    value.is_multiple_of(7)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_all_zero_code_is_valid() {
        assert!(is_room_code_valid("U/0000-0000-0000-0000"));
    }

    #[test]
    fn a_code_whose_digits_are_a_multiple_of_seven_is_valid() {
        // The little-endian digit value is 7, so the checksum passes.
        assert!(is_room_code_valid("U/7000-0000-0000-0000"));
    }

    #[test]
    fn a_code_whose_checksum_is_wrong_is_rejected() {
        // 1 * 34^0 at the first position, and 1 * 34^3 at the fourth.
        assert!(!is_room_code_valid("U/1000-0000-0000-0000"));
        assert!(!is_room_code_valid("U/0001-0000-0000-0000"));
    }

    #[test]
    fn the_shape_is_a_u_slash_and_three_separators() {
        assert!(!is_room_code_valid(""));
        assert!(!is_room_code_valid("U/0000-0000-0000-000")); // one short
        assert!(!is_room_code_valid("U/00000-000-0000-0000")); // separators moved
        assert!(!is_room_code_valid("X/0000-0000-0000-0000")); // wrong prefix
        assert!(!is_room_code_valid("U0000-0000-0000-0000")); // missing slash
    }

    #[test]
    fn the_alphabet_accepts_the_letters_it_should_and_skips_i_and_o() {
        // 'E' has digit value 14, and 14 * 34^15 is a multiple of 7, so this
        // letter at the last position is a valid checksum.
        assert!(is_room_code_valid("U/0000-0000-0000-000E"));
        assert!(!is_room_code_valid("U/000I-0000-0000-0000"));
        assert!(!is_room_code_valid("U/000O-0000-0000-0000"));
        assert!(!is_room_code_valid("U/0000-0000-0000-000*"));
    }
}
