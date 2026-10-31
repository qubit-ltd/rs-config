// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::borrow::Cow;
use std::collections::HashMap;
use std::time::Duration;

#[cfg(feature = "bigdecimal")]
use bigdecimal::BigDecimal;
#[cfg(feature = "chrono")]
use chrono::DateTime;
#[cfg(feature = "chrono")]
use chrono::NaiveDate;
#[cfg(feature = "chrono")]
use chrono::NaiveDateTime;
#[cfg(feature = "chrono")]
use chrono::NaiveTime;
#[cfg(feature = "chrono")]
use chrono::Utc;
#[cfg(feature = "num-bigint")]
use num_bigint::BigInt;
use qubit_datatype::ConversionSession;
use qubit_datatype::DataConversionTarget;
use qubit_datatype::DataConverter;
use qubit_datatype::DataListConversionError;
use qubit_value::MultiValues;
use qubit_value::MultiValuesRef;
use qubit_value::Value as QubitValue;
use qubit_value::ValueContainer;
use qubit_value::ValueError;
use qubit_value::ValueRef;
use serde_json::Value as JsonValue;
#[cfg(feature = "url")]
use url::Url;

use super::config_parse_context::ConfigParseContext;
use super::helpers::first_scalar_string;
use crate::ConfigError;
use crate::ConfigResult;
use crate::Property;
use crate::utils;

/// Parses a configuration [`Property`] into a target Rust type.
///
/// Implement this trait for an application type by delegating to an existing
/// conversion when possible. The reader supplies [`ConfigParseContext`], so a
/// custom implementation can preserve its conversion policy and resolved key
/// in errors without constructing a context itself.
///
/// ```
/// use qubit_config::conversion::{ConfigParseContext, FromConfig};
/// use qubit_config::{Config, ConfigErrorKind, ConfigReader, ConfigResult, Property};
///
/// #[derive(Debug, PartialEq)]
/// struct ServicePort(u16);
///
/// impl FromConfig for ServicePort {
///     fn from_config(property: &Property, ctx: &ConfigParseContext<'_>) -> ConfigResult<Self> {
///         u16::from_config(property, ctx).map(Self)
///     }
/// }
///
/// fn main() -> ConfigResult<()> {
///     let mut config = Config::new();
///     config.set("server.port", "8080")?;
///     assert_eq!(config.get::<ServicePort>("server.port")?, ServicePort(8080));
///
///     config.set("server.default_port", "9090")?;
///     config.set("server.port", "${default_port}")?;
///     let server = config.section("server")?;
///     assert_eq!(server.get_interpolated::<ServicePort>("port")?, ServicePort(9090));
///     assert!(server.get::<ServicePort>("port").is_err());
///
///     config.set("server.port", "invalid")?;
///     let error = config.get::<ServicePort>("server.port").unwrap_err();
///     assert_eq!(error.kind(), ConfigErrorKind::Conversion);
///     assert_eq!(error.path(), Some("server.port"));
///     Ok(())
/// }
/// ```
///
/// `ServicePort` is an application-level example. Whether port zero is
/// permitted is a separate business validation decision.
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
    fn from_config(property: &Property, ctx: &ConfigParseContext<'_>) -> ConfigResult<Self>;
}

/// Converts a property's scalar value or selected first value to a target type.
///
/// When interpolation is enabled, a string collection's first item is
/// substituted before conversion. Remaining collection items are not read.
///
/// # Parameters
///
/// * `property` - The property to convert.
/// * `ctx` - The parsing context.
///
/// # Returns
///
/// The converted value, or a [`crate::ConfigError`] with key context.
fn convert_first<T>(property: &Property, ctx: &ConfigParseContext<'_>) -> ConfigResult<T>
where
    T: DataConversionTarget,
{
    if ctx.interpolates()
        && let ValueContainer::Collection(values) = property.value()
        && let MultiValuesRef::String(values) = values.view()
        && let Some(first) = values.first()
    {
        let expanded = ctx.substitute_string(first)?;
        let selected = ValueContainer::Collection(MultiValues::String(vec![expanded]));
        return selected
            .to_first_with::<T>(ctx.options().conversion_policy(), ctx.options().conversion_limits())
            .map_err(|error| utils::map_value_error(ctx.key(), error));
    }

    if let Some(raw) = first_scalar_string(property) {
        if !ctx.interpolates() {
            return property
                .value()
                .to_first_with::<T>(ctx.options().conversion_policy(), ctx.options().conversion_limits())
                .map_err(|e| utils::map_value_error(ctx.key(), e));
        }
        let value = ctx.scalar_text(raw)?;
        DataConverter::from(value.as_ref())
            .to_with::<T>(ctx.options().conversion_policy(), ctx.options().conversion_limits())
            .map_err(|error| utils::map_value_error(ctx.key(), error.into()))
    } else {
        property
            .value()
            .to_first_with::<T>(ctx.options().conversion_policy(), ctx.options().conversion_limits())
            .map_err(|e| utils::map_value_error(ctx.key(), e))
    }
}

/// Builds a conversion source with a scalar string substituted.
///
/// # Parameters
///
/// * `property` - Property whose value is used as the conversion source.
/// * `ctx` - Parsing context that supplies variable substitution.
///
/// # Returns
///
/// A borrowed or owned [`ValueContainer`] with its original shape preserved;
/// non-string scalar values remain borrowed.
///
/// # Errors
///
/// Returns a substitution error if the scalar string cannot be resolved.
fn substituted_values<'a>(
    property: &'a Property,
    ctx: &ConfigParseContext<'_>,
) -> ConfigResult<Cow<'a, ValueContainer>> {
    if !ctx.interpolates() {
        return Ok(Cow::Borrowed(property.value()));
    }
    match property.value() {
        ValueContainer::Scalar(value) => match value.view() {
            ValueRef::String(value) => ctx
                .substitute_string(value)
                .map(QubitValue::String)
                .map(ValueContainer::Scalar)
                .map(Cow::Owned),
            _ => Ok(Cow::Borrowed(property.value())),
        },
        ValueContainer::Collection(_) => Ok(Cow::Borrowed(property.value())),
    }
}

/// Expands and converts interpolated string collection elements in source
/// order.
///
/// Each expanded string is converted before the next element is substituted,
/// while one conversion session accumulates the collection's conversion limits.
///
/// # Parameters
///
/// * `values` - Original string collection elements.
/// * `ctx` - Parsing context that supplies substitution and conversion options.
///
/// # Returns
///
/// Converted collection values, or the first substitution or conversion error.
///
/// # Errors
///
/// Returns a substitution error or a conversion error containing the original
/// collection index and configuration key.
fn convert_interpolated_string_collection<T>(values: &[String], ctx: &ConfigParseContext<'_>) -> ConfigResult<Vec<T>>
where
    T: DataConversionTarget,
{
    let mut session = ConversionSession::new(ctx.options().conversion_policy(), ctx.options().conversion_limits());
    let mut converted = Vec::new();
    for (index, raw) in values.iter().enumerate() {
        let expanded = ctx
            .substitute_string(raw)
            .map_err(|source| ConfigError::IndexedSubstitutionError {
                source_index: index,
                source: Box::new(source),
            })?;
        let value = DataConverter::from(expanded.as_str())
            .to_in::<T>(&mut session)
            .map_err(|error| {
                utils::map_value_error(ctx.key(), ValueError::from(DataListConversionError::new(index, error)))
            })?;
        converted.push(value);
    }
    Ok(converted)
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

impl FromConfig for usize {
    /// Parses a value as `u128`, then checks that it fits this platform's
    /// `usize`.
    ///
    /// For a collection, only the first item is converted; an out-of-range
    /// error records index zero. Errors from the delegated `u128` conversion,
    /// including interpolation and conversion-budget errors, are preserved.
    ///
    /// # Errors
    ///
    /// Returns the delegated conversion error, or
    /// [`ConfigError::PlatformIntegerOutOfRange`] when the parsed integer is
    /// larger than `usize::MAX` on the current platform.
    fn from_config(property: &Property, ctx: &ConfigParseContext<'_>) -> ConfigResult<Self> {
        let value = u128::from_config(property, ctx)?;
        usize::try_from(value).map_err(|_| ConfigError::PlatformIntegerOutOfRange {
            key: ctx.key().to_owned(),
            source_index: match property.value() {
                ValueContainer::Collection(_) => Some(0),
                ValueContainer::Scalar(_) => None,
            },
        })
    }
}

#[cfg(feature = "chrono")]
impl_from_config_via_value!(NaiveDate, NaiveTime, NaiveDateTime, DateTime<Utc>);

#[cfg(feature = "url")]
impl_from_config_via_value!(Url);

#[cfg(feature = "num-bigint")]
impl_from_config_via_value!(BigInt);

#[cfg(feature = "bigdecimal")]
impl_from_config_via_value!(BigDecimal);

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
    fn from_config(property: &Property, ctx: &ConfigParseContext<'_>) -> ConfigResult<Self> {
        convert_first::<String>(property, ctx)
    }
}

impl<T> FromConfig for Vec<T>
where
    T: DataConversionTarget,
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
    fn from_config(property: &Property, ctx: &ConfigParseContext<'_>) -> ConfigResult<Self> {
        if ctx.interpolates()
            && let ValueContainer::Collection(values) = property.value()
            && let MultiValuesRef::String(values) = values.view()
        {
            return convert_interpolated_string_collection(values, ctx);
        }

        substituted_values(property, ctx)?
            .to_list_with::<T>(ctx.options().conversion_policy(), ctx.options().conversion_limits())
            .map_err(|error| utils::map_value_error(ctx.key(), error))
    }
}
