// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Bounded, independent semantic checks for Java-properties parsing.

use qubit_config::source::ConfigSource;
use qubit_config::source::PropertiesConfigSource;

const MAX_PAYLOAD: usize = 64;

/// Checks one bounded generated document against independently constructed
/// values.
pub fn check(data: &[u8]) {
    let selector = data.first().copied().unwrap_or(0);
    let payload = data
        .get(1..)
        .unwrap_or_default()
        .iter()
        .take(MAX_PAYLOAD)
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();

    // These branches have literal expectations and exercise preservation and
    // unknown-escape behavior independently of the generated encoder below.
    if selector == 0xfe {
        check_pairs(
            "first=\\u12G4\nsecond=\\uD800\nthird=\\u12\nfourth=\\uDC00",
            vec![
                ("first".into(), r"\u12G4".into()),
                ("second".into(), r"\uD800".into()),
                ("third".into(), r"\u12".into()),
                ("fourth".into(), r"\uDC00".into()),
            ],
        );
        return;
    }
    if selector == 0xff {
        check_pairs(
            "first=a\\q\nsecond=\\uD83D\\uDE00",
            vec![("first".into(), "aq".into()), ("second".into(), "😀".into())],
        );
        return;
    }

    let separator = match selector % 3 {
        0 => "=",
        1 => ":",
        _ => " ",
    };
    let newline = match (selector / 3) % 3 {
        0 => "\n",
        1 => "\r\n",
        _ => "\r",
    };
    let indent = if selector & 0x20 == 0 { " " } else { "\t" };

    let key_a = format!("first {payload}=x");
    let key_b = format!("second-{payload}:y");
    let value_a = format!("payload {payload}\\line\n雪😀");
    let value_b = format!("tail:{payload} =\\done\t\r\u{000c}é");
    let expected = vec![(key_a.clone(), value_a.clone()), (key_b.clone(), value_b.clone())];

    let encoded_key_a = encode(&key_a);
    let encoded_key_b = encode(&key_b);
    let encoded_value_a = encode(&value_a);
    let encoded_value_b = encode(&value_b);
    let split = safe_split(&encoded_value_a);
    let input = format!(
        "# generated properties{newline} \t{encoded_key_a}{separator}{}\\{newline}{indent}{}{newline} \t{encoded_key_b}{separator}{encoded_value_b}{newline}",
        &encoded_value_a[..split],
        &encoded_value_a[split..],
    );

    check_pairs(&input, expected);
}

fn check_pairs(input: &str, expected_pairs: Vec<(String, String)>) {
    assert_eq!(PropertiesConfigSource::parse_content(input).unwrap(), expected_pairs);

    let config = PropertiesConfigSource::from_content(input).load().unwrap();
    assert_eq!(config.len(), expected_pairs.len());
    for (key, expected) in expected_pairs {
        assert_eq!(config.get::<String>(&key).unwrap(), expected);
    }
}

/// Encodes semantic characters using properties escapes, including UTF-16
/// pairs.
fn encode(value: &str) -> String {
    let mut encoded = String::new();
    for ch in value.chars() {
        match ch {
            '\\' => encoded.push_str("\\\\"),
            '\n' => encoded.push_str("\\n"),
            '\r' => encoded.push_str("\\r"),
            '\t' => encoded.push_str("\\t"),
            '\u{000c}' => encoded.push_str("\\f"),
            '=' | ':' | ' ' => {
                encoded.push('\\');
                encoded.push(ch);
            }
            ch if ch.is_ascii() => encoded.push(ch),
            ch if (ch as u32) <= 0xffff => {
                encoded.push_str(&format!("\\u{:04X}", ch as u32));
            }
            ch => {
                let scalar = ch as u32 - 0x1_0000;
                let high = 0xD800 + (scalar >> 10);
                let low = 0xDC00 + (scalar & 0x3FF);
                encoded.push_str(&format!("\\u{high:04X}\\u{low:04X}"));
            }
        }
    }
    encoded
}

/// Chooses a boundary between encoded characters, never inside an escape.
fn safe_split(encoded: &str) -> usize {
    let mut boundaries = vec![0];
    let mut index = 0;
    while index < encoded.len() {
        let bytes = encoded.as_bytes();
        let token_len = if bytes[index] == b'\\' {
            if bytes.get(index + 1) == Some(&b'u') { 6 } else { 2 }
        } else {
            encoded[index..].chars().next().unwrap().len_utf8()
        };
        index += token_len;
        boundaries.push(index);
    }
    boundaries
        .into_iter()
        .min_by_key(|boundary| boundary.abs_diff(encoded.len() / 2))
        .unwrap_or(0)
}
