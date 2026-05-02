/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/

use crate::options::ConfigReadOptions;

use super::config_field::ConfigField;

/// Builder state after the field name is provided.
///
/// # Author
///
/// Haixing Hu
pub struct ConfigFieldBuilder<T> {
    /// The primary field name.
    pub(crate) name: String,
    /// The fallback aliases.
    pub(crate) aliases: Vec<String>,
    /// The default value.
    pub(crate) default: Option<T>,
    /// The read options.
    pub(crate) read_options: Option<ConfigReadOptions>,
}

impl<T> ConfigFieldBuilder<T> {
    /// Adds a fallback alias.
    ///
    /// # Parameters
    ///
    /// * `alias` - Fallback key checked after previous names.
    ///
    /// # Returns
    ///
    /// Updated builder.
    pub fn alias(mut self, alias: &str) -> Self {
        self.aliases.push(alias.to_string());
        self
    }

    /// Sets the default value used when all names are absent.
    ///
    /// # Parameters
    ///
    /// * `default` - Default value.
    ///
    /// # Returns
    ///
    /// Updated builder.
    pub fn default(mut self, default: T) -> Self {
        self.default = Some(default);
        self
    }

    /// Sets field-level read options.
    ///
    /// # Parameters
    ///
    /// * `read_options` - Options that override the reader's global options.
    ///
    /// # Returns
    ///
    /// Updated builder.
    pub fn read_options(mut self, read_options: ConfigReadOptions) -> Self {
        self.read_options = Some(read_options);
        self
    }

    /// Finishes the builder.
    ///
    /// # Returns
    ///
    /// A field declaration ready to be passed to [`crate::ConfigReader::read`].
    pub fn build(self) -> ConfigField<T> {
        ConfigField {
            name: self.name,
            aliases: self.aliases,
            default: self.default,
            read_options: self.read_options,
        }
    }
}
