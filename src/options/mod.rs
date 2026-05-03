/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/

//! Read parsing options for configuration values.

mod blank_string_policy;
mod boolean_read_options;
mod collection_read_options;
mod config_read_options;
mod empty_item_policy;
mod string_read_options;

pub use blank_string_policy::BlankStringPolicy;
pub use boolean_read_options::BooleanReadOptions;
pub use collection_read_options::CollectionReadOptions;
pub use config_read_options::ConfigReadOptions;
#[cfg(coverage)]
#[doc(hidden)]
pub use config_read_options::coverage_touch_config_read_option_serde_defaults;
pub use empty_item_policy::EmptyItemPolicy;
pub use string_read_options::StringReadOptions;
