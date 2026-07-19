// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::marker::PhantomData;

use crate::options::ReadOptions;

use super::config_field_name_builder::ConfigFieldNameBuilder;

/// Field-level read declaration for [`crate::ConfigReader::read`].
#[must_use = "use the field declaration with ConfigReader::read"]
#[derive(Debug, Clone, PartialEq)]
pub struct ConfigField<T> {
    /// The primary field name.
    pub(crate) name: String,
    /// The fallback aliases.
    pub(crate) aliases: Vec<String>,
    /// The default value.
    pub(crate) default: Option<T>,
    /// The read options.
    pub(crate) read_options: Option<ReadOptions>,
}

impl<T> ConfigField<T> {
    /// Starts building a field declaration.
    ///
    /// # Returns
    ///
    /// A builder requiring a primary field name before `build` is available.
    ///
    /// Discarding the builder is rejected when unused results are denied.
    ///
    /// ```compile_fail
    /// #![deny(unused_must_use)]
    /// use qubit_config::field::ConfigField;
    ///
    /// ConfigField::<String>::builder();
    /// ```
    #[must_use = "use the returned builder to declare a configuration field"]
    pub fn builder() -> ConfigFieldNameBuilder<T> {
        ConfigFieldNameBuilder {
            aliases: Vec::new(),
            default: None,
            read_options: None,
            marker: PhantomData,
        }
    }
}
