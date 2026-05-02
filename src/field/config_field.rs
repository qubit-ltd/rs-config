/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/

use std::marker::PhantomData;

use crate::options::ConfigReadOptions;

use super::config_field_name_builder::ConfigFieldNameBuilder;

/// Field-level read declaration for [`crate::ConfigReader::read`].
///
/// # Author
///
/// Haixing Hu
#[derive(Debug, Clone, PartialEq)]
pub struct ConfigField<T> {
    /// The primary field name.
    pub(crate) name: String,
    /// The fallback aliases.
    pub(crate) aliases: Vec<String>,
    /// The default value.
    pub(crate) default: Option<T>,
    /// The read options.
    pub(crate) read_options: Option<ConfigReadOptions>,
}

impl<T> ConfigField<T> {
    /// Starts building a field declaration.
    ///
    /// # Returns
    ///
    /// A builder requiring a primary field name before `build` is available.
    pub fn builder() -> ConfigFieldNameBuilder<T> {
        ConfigFieldNameBuilder {
            aliases: Vec::new(),
            default: None,
            read_options: None,
            marker: PhantomData,
        }
    }
}
