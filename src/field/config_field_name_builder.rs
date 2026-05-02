/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/

use std::marker::PhantomData;

use crate::options::ConfigReadOptions;

use super::config_field_builder::ConfigFieldBuilder;

/// Builder state before the field name is provided.
///
pub struct ConfigFieldNameBuilder<T> {
    /// The fallback aliases.
    pub(crate) aliases: Vec<String>,
    /// The default value.
    pub(crate) default: Option<T>,
    /// The read options.
    pub(crate) read_options: Option<ConfigReadOptions>,
    /// The type marker.
    pub(crate) marker: PhantomData<T>,
}

impl<T> ConfigFieldNameBuilder<T> {
    /// Sets the primary field name and unlocks [`super::ConfigFieldBuilder::build`].
    ///
    /// # Parameters
    ///
    /// * `name` - Primary configuration key.
    ///
    /// # Returns
    ///
    /// Builder state with a primary name.
    pub fn name(self, name: &str) -> ConfigFieldBuilder<T> {
        ConfigFieldBuilder {
            name: name.to_string(),
            aliases: self.aliases,
            default: self.default,
            read_options: self.read_options,
        }
    }
}
