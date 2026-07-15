// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::collections::HashMap;
use std::time::Duration;

#[cfg(feature = "rich-types")]
use bigdecimal::BigDecimal;
#[cfg(feature = "rich-types")]
use chrono::{
    DateTime,
    NaiveDate,
    NaiveDateTime,
    NaiveTime,
    Utc,
};
#[cfg(feature = "rich-types")]
use num_bigint::BigInt;
use qubit_datatype::{
    DataConvertTo,
    DataConverter,
    DataTypeOf,
};
use qubit_value::{
    MultiValues,
    Value as QubitValue,
    ValueContainer,
};
use serde_json::Value as JsonValue;
#[cfg(feature = "rich-types")]
use url::Url;

use crate::{
    ConfigResult,
    Property,
    utils,
};

use super::config_parse_context::ConfigParseContext;
use super::helpers::first_scalar_string;

/// Parses a configuration [`Property`] into a target Rust type.
pub trait FromConfig: Sized {
    /// Parses `property` using `ctx`.
    ///
    /// # Parameters
    ///
    /// * `property` - Property selected by the reader.
    /// * `ctx` - Key, options, and substitution context.
    ///
    /// # Returns
    ///
    /// Parsed value, or a [`crate::ConfigError`] with key context.
    fn from_config(
        property: &Property,
        ctx: &ConfigParseContext<'_>,
    ) -> ConfigResult<Self>;
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
fn convert_first<T>(
    property: &Property,
    ctx: &ConfigParseContext<'_>,
) -> ConfigResult<T>
where
    for<'a> DataConverter<'a>: DataConvertTo<T>,
    T: DataTypeOf,
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
/// A [`ValueContainer`] with string entries substituted and its original shape
/// preserved; non-string entries are cloned unchanged.
///
/// # Errors
///
/// Returns a substitution error if any string entry cannot be resolved.
fn substituted_values(
    property: &Property,
    ctx: &ConfigParseContext<'_>,
) -> ConfigResult<ValueContainer> {
    match property.value() {
        ValueContainer::Scalar(QubitValue::String(value)) => ctx
            .substitute_string(value)
            .map(QubitValue::String)
            .map(ValueContainer::Scalar),
        ValueContainer::Collection(MultiValues::String(values)) => values
            .iter()
            .map(|value| ctx.substitute_string(value))
            .collect::<ConfigResult<Vec<_>>>()
            .map(MultiValues::String)
            .map(ValueContainer::Collection),
        values => Ok(values.clone()),
    }
}

/// Implements the `FromConfig` trait for a list of types.
///
/// # Parameters
///
/// * `($($ty:ty),+ $(,)?)` - The list of types to implement the trait for.
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
    u8,
    u16,
    u32,
    u64,
    u128,
    f32,
    f64,
    char,
    Duration,
    JsonValue,
    HashMap<String, String>,
);

#[cfg(feature = "rich-types")]
impl_from_config_via_value!(
    NaiveDate,
    NaiveTime,
    NaiveDateTime,
    DateTime<Utc>,
    Url,
    BigInt,
    BigDecimal,
);

impl FromConfig for String {
    /// Parses `property` using `ctx`.
    ///
    /// # Parameters
    ///
    /// * `property` - Property selected by the reader.
    /// * `ctx` - Key, options, and substitution context.
    ///
    /// # Returns
    ///
    /// Parsed value, or a [`crate::ConfigError`] with key context.
    fn from_config(
        property: &Property,
        ctx: &ConfigParseContext<'_>,
    ) -> ConfigResult<Self> {
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
    T: DataTypeOf,
    for<'a> DataConverter<'a>: DataConvertTo<T>,
{
    /// Parses `property` using `ctx`.
    ///
    /// # Parameters
    ///
    /// * `property` - Property selected by the reader.
    /// * `ctx` - Key, options, and substitution context.
    ///
    /// # Returns
    ///
    /// Parsed value, or a [`crate::ConfigError`] with key context.
    fn from_config(
        property: &Property,
        ctx: &ConfigParseContext<'_>,
    ) -> ConfigResult<Self> {
        substituted_values(property, ctx)?
            .to_list_with::<T>(ctx.options().conversion_options())
            .map_err(|error| utils::map_value_error(ctx.key(), error))
    }
}
