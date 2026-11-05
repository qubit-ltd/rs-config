// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Demonstrates structured object, collection, and byte deserialization.

use qubit_config::Config;
use serde::Deserialize;
use serde::de;
use serde::de::Deserializer;
use serde::de::Visitor;

/// Server settings selected from a structured configuration section.
#[derive(Debug, Deserialize, PartialEq)]
struct Server {
    /// Server host name.
    host: String,
    /// TCP service port.
    port: u16,
}

/// Byte sequence deserialized from a string value using serde bytes.
#[derive(Debug, PartialEq)]
struct Bytes(Vec<u8>);

impl<'de> Deserialize<'de> for Bytes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        /// Accepts byte buffers produced by the configured deserializer.
        struct BytesVisitor;

        impl Visitor<'_> for BytesVisitor {
            type Value = Bytes;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("bytes")
            }

            fn visit_byte_buf<E>(self, value: Vec<u8>) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Bytes(value))
            }
        }

        deserializer.deserialize_bytes(BytesVisitor)
    }
}

/// Reads a server object, numeric vector, and text as bytes.
fn main() -> Result<(), qubit_config::ConfigError> {
    let mut config = Config::new();
    config.set("server.host", "localhost")?;
    config.set("server.port", 8080)?;
    config.set("numbers", vec![65_u8, 66_u8])?;
    config.set("text", "AB")?;

    let server: Server = config.deserialize("server")?;
    assert_eq!(server.host, "localhost");
    assert_eq!(server.port, 8080);

    let numbers: Vec<u8> = config.deserialize("numbers")?;
    assert_eq!(numbers, vec![65_u8, 66_u8]);

    let bytes: Bytes = config.deserialize("text")?;
    assert_eq!(bytes.0, b"AB");
    Ok(())
}
