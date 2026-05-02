/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/

//! Parsing support for typed configuration reads.

mod config_parse_context;
mod from_config;
mod helpers;
mod into_config_default;

pub use config_parse_context::ConfigParseContext;
pub use from_config::FromConfig;
pub use into_config_default::IntoConfigDefault;

pub(crate) use helpers::{is_effectively_missing, parse_property_from_reader};
