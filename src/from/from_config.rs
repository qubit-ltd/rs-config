/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/

use std::collections::HashMap;
use std::time::Duration;

use bigdecimal::BigDecimal;
use chrono::{
    DateTime,
    NaiveDate,
    NaiveDateTime,
    NaiveTime,
    Utc,
};
use num_bigint::BigInt;
use qubit_datatype::{
    DataConvertTo,
    DataConverter,
};
use qubit_value::{
    MultiValues,
    Value as QubitValue,
};
use serde_json::Value as JsonValue;
use url::Url;

use crate::{
    ConfigResult,
    Property,
    utils,
};

use super::config_parse_context::ConfigParseContext;
use super::helpers::first_scalar_string;

/// Parses a configuration [`Property`] into a target Rust type.
///
pub trait FromConfig: Sized {
    /// Parses `property` using `ctx`.
    ///
    /// # Parameters
    ///
    /// * `property` - Non-empty property selected by the reader.
    /// * `ctx` - Key, options, and substitution context.
    ///
    /// # Returns
    ///
    /// Parsed value, or a [`crate::ConfigError`] with key context.
    fn from_config(property: &Property, ctx: &ConfigParseContext<'_>) -> ConfigResult<Self>;
}

/// Converts the first scalar string value of a property to a target type.
///
/// # Parameters
///
/// * `property` - The property to convert.
/// * `ctx` - The parsing context.
///
/// # Returns
///
/// The converted value, or a [`ConfigError`] with key context.
///
fn convert_first<T>(property: &Property, ctx: &ConfigParseContext<'_>) -> ConfigResult<T>
where
    for<'a> DataConverter<'a>: DataConvertTo<T>,
{
    if let Some(value) = first_scalar_string(property) {
        let value = ctx.substitute_string(value)?;
        QubitValue::String(value)
            .to_with::<T>(ctx.options().conversion_options())
            .map_err(|e| utils::map_value_error(ctx.key(), e))
    } else {
        property
            .value()
            .to_with::<T>(ctx.options().conversion_options())
            .map_err(|e| utils::map_value_error(ctx.key(), e))
    }
}

/// Builds a conversion source with string leaves after variable substitution.
///
/// # Parameters
///
/// * `property` - Property whose value is used as the conversion source.
/// * `ctx` - Parsing context that supplies variable substitution.
///
/// # Returns
///
/// A [`MultiValues`] value with string entries substituted; non-string entries
/// are cloned unchanged.
///
/// # Errors
///
/// Returns a substitution error if any string entry cannot be resolved.
fn substituted_values(
    property: &Property,
    ctx: &ConfigParseContext<'_>,
) -> ConfigResult<MultiValues> {
    match property.value() {
        MultiValues::String(values) => values
            .iter()
            .map(|value| ctx.substitute_string(value))
            .collect::<ConfigResult<Vec<_>>>()
            .map(MultiValues::String),
        values => Ok(values.clone()),
    }
}

/// Implements the `FromConfig` trait for a list of types.
///
/// # Parameters
///
/// * `($($ty:ty),+ $(,)?)` - The list of types to implement the trait for.
///
macro_rules! impl_from_config_via_value {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl FromConfig for $ty {
                fn from_config(
                    property: &Property,
                    ctx: &ConfigParseContext<'_>,
                ) -> ConfigResult<Self> {
                    convert_first::<$ty>(property, ctx)
                }
            }
        )+
    };
}

impl_from_config_via_value!(
    bool,
    i8,
    i16,
    i32,
    i64,
    i128,
    isize,
    u8,
    u16,
    u32,
    u64,
    u128,
    usize,
    f32,
    f64,
    char,
    NaiveDate,
    NaiveTime,
    NaiveDateTime,
    DateTime<Utc>,
    Duration,
    Url,
    BigInt,
    BigDecimal,
    JsonValue,
    HashMap<String, String>,
);

impl FromConfig for String {
    /// Parses `property` using `ctx`.
    ///
    /// # Parameters
    ///
    /// * `property` - Non-empty property selected by the reader.
    /// * `ctx` - Key, options, and substitution context.
    ///
    /// # Returns
    ///
    /// Parsed value, or a [`crate::ConfigError`] with key context.
    fn from_config(property: &Property, ctx: &ConfigParseContext<'_>) -> ConfigResult<Self> {
        if let Some(value) = first_scalar_string(property) {
            let value = ctx.substitute_string(value)?;
            QubitValue::String(value)
                .to_with::<String>(ctx.options().conversion_options())
                .map_err(|e| utils::map_value_error(ctx.key(), e))
        } else {
            property
                .value()
                .to_with::<String>(ctx.options().conversion_options())
                .map_err(|e| utils::map_value_error(ctx.key(), e))
        }
    }
}

impl<T> FromConfig for Vec<T>
where
    T: FromConfig,
{
    /// Parses `property` using `ctx`.
    ///
    /// # Parameters
    ///
    /// * `property` - Non-empty property selected by the reader.
    /// * `ctx` - Key, options, and substitution context.
    ///
    /// # Returns
    ///
    /// Parsed value, or a [`crate::ConfigError`] with key context.
    fn from_config(property: &Property, ctx: &ConfigParseContext<'_>) -> ConfigResult<Self> {
        let values = substituted_values(property, ctx)?
            .to_list_with::<String>(ctx.options().conversion_options())
            .map_err(|e| utils::map_value_error(ctx.key(), e))?;

        let mut result = Vec::new();
        for item in values {
            let item_property =
                Property::with_value(ctx.key().to_string(), MultiValues::String(vec![item]));
            result.push(T::from_config(&item_property, ctx)?);
        }
        Ok(result)
    }
}
