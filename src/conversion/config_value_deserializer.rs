// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Serde deserializer that applies configuration read conversion semantics.

use qubit_datatype::AdmittedScalarItem;
use qubit_datatype::ConversionSession;
use qubit_datatype::DataConversionError;
use qubit_datatype::DataConversionTarget;
use qubit_datatype::DataConverter;
use qubit_value::Value as QubitValue;
use qubit_value::ValueError;
use serde::de;
use serde::de::Visitor;
use serde_json::Value;

use crate::ConfigError;
use crate::config_deserialize_error::ConfigDeserializeError;
use crate::config_value_deserializer::internal::ConfigEnumAccess;
use crate::config_value_deserializer::internal::ConfigMapAccess;
use crate::config_value_deserializer::internal::ConfigScalarSeqAccess;
use crate::config_value_deserializer::internal::ConfigSeqAccess;
use crate::config_value_deserializer::internal::admit_scalar_items;
use crate::options::ReadPolicy;

mod internal;

/// Deserializer over a single serde value.
pub(crate) struct ConfigValueDeserializer<'policy, 'session, 'source> {
    input: ConfigConversionInput<'session, 'policy, 'source>,
    key: String,
    options: &'policy ReadPolicy,
}

/// Owns exactly one conversion source and its associated session capability.
enum ConfigConversionInput<'session, 'policy, 'source> {
    /// A normal Serde value that has not been admitted as a scalar item.
    TopLevel {
        value: Value,
        session: &'session mut ConversionSession<'policy>,
    },
    /// A scalar value already charged to the bound conversion session.
    AdmittedScalar(AdmittedScalarItem<'session, 'policy, 'source>),
}

impl<'policy, 'session> ConfigValueDeserializer<'policy, 'session, 'static> {
    /// Creates a value deserializer.
    pub(crate) fn new(
        value: Value,
        key: String,
        options: &'policy ReadPolicy,
        session: &'session mut ConversionSession<'policy>,
    ) -> Self {
        Self {
            input: ConfigConversionInput::TopLevel { value, session },
            key,
            options,
        }
    }
}

impl<'policy, 'session, 'source> ConfigValueDeserializer<'policy, 'session, 'source> {
    /// Creates a deserializer for an item already admitted by a scalar list.
    pub(crate) fn new_admitted(
        key: String,
        options: &'policy ReadPolicy,
        admission: AdmittedScalarItem<'session, 'policy, 'source>,
    ) -> Self {
        Self {
            input: ConfigConversionInput::AdmittedScalar(admission),
            key,
            options,
        }
    }

    /// Converts any scalar value into a string using config read semantics.
    fn scalar_to_string(self) -> Result<String, ConfigDeserializeError> {
        match self.input {
            ConfigConversionInput::AdmittedScalar(admission) => admission
                .convert::<String>()
                .map_err(|error| map_admitted_error(&self.key, error)),
            ConfigConversionInput::TopLevel { value, session } => match value {
                Value::String(value) => convert_string_value(&self.key, session, &value),
                Value::Bool(value) => convert_scalar_value(&self.key, session, QubitValue::Bool(value)),
                Value::Number(value) => {
                    let source = if let Some(value) = value.as_i64() {
                        QubitValue::Int64(value)
                    } else if let Some(value) = value.as_u64() {
                        QubitValue::UInt64(value)
                    } else {
                        QubitValue::Float64(value.as_f64().expect("JSON numbers are finite"))
                    };
                    convert_scalar_value(&self.key, session, source)
                }
                Value::Null => Err(de::Error::invalid_type(
                    de::Unexpected::Unit,
                    &"a string-compatible scalar",
                )),
                Value::Array(_) => Err(de::Error::invalid_type(
                    de::Unexpected::Seq,
                    &"a string-compatible scalar",
                )),
                Value::Object(_) => Err(de::Error::invalid_type(
                    de::Unexpected::Map,
                    &"a string-compatible scalar",
                )),
            },
        }
    }
}

/// Maps an error from an already admitted scalar without adding another
/// top-level accounting layer.
fn map_admitted_error(key: &str, error: DataConversionError) -> ConfigDeserializeError {
    ConfigDeserializeError::from_config(ConfigError::from((key, ValueError::from(error))))
}

/// Converts a scalar string using the shared conversion layer.
fn convert_string_value(
    key: &str,
    session: &mut ConversionSession<'_>,
    value: &str,
) -> Result<String, ConfigDeserializeError> {
    let source = DataConverter::from(value);
    source.to_in::<String>(session).map_err(|error| {
        ConfigDeserializeError::from_config(crate::utils::map_value_error(key, ValueError::from(error)))
    })
}

/// Converts a scalar through either top-level or nested session accounting.
fn convert_scalar_value<T>(
    key: &str,
    session: &mut ConversionSession<'_>,
    value: QubitValue,
) -> Result<T, ConfigDeserializeError>
where
    T: DataConversionTarget,
{
    value
        .to_in::<T>(session)
        .map_err(|error| ConfigDeserializeError::from_config(crate::utils::map_value_error(key, error)))
}

/// Converts a scalar string into a boolean using the shared conversion layer.
fn convert_bool_value(
    key: &str,
    session: &mut ConversionSession<'_>,
    value: &str,
) -> Result<bool, ConfigDeserializeError> {
    let source = DataConverter::from(value);
    source.to_in::<bool>(session).map_err(|error| {
        ConfigDeserializeError::from_config(crate::utils::map_value_error(key, ValueError::from(error)))
    })
}

/// Converts a scalar string into a character using the shared conversion
/// layer.
fn convert_char_value(
    key: &str,
    session: &mut ConversionSession<'_>,
    value: &str,
) -> Result<char, ConfigDeserializeError> {
    let source = DataConverter::from(value);
    source.to_in::<char>(session).map_err(|error| {
        ConfigDeserializeError::from_config(crate::utils::map_value_error(key, ValueError::from(error)))
    })
}

/// Converts a JSON number or string into the scalar text consumed by
/// `qubit-value` conversion.
fn number_scalar_text(value: Value, expected: &'static str) -> Result<String, ConfigDeserializeError> {
    match value {
        Value::Number(value) => Ok(value.to_string()),
        Value::String(value) => Ok(value),
        other => Err(de::Error::invalid_type(unexpected_value(&other), &expected)),
    }
}

macro_rules! deserialize_number {
    ($method:ident, $visit:ident, $ty:ty) => {
        fn $method<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            let value = match self.input {
                ConfigConversionInput::AdmittedScalar(admission) => admission
                    .convert::<$ty>()
                    .map_err(|error| map_admitted_error(&self.key, error))?,
                ConfigConversionInput::TopLevel { value, session } => {
                    let value = number_scalar_text(value, stringify!($ty))?;
                    convert_scalar_value::<$ty>(&self.key, session, QubitValue::String(value))?
                }
            };
            visitor.$visit(value)
        }
    };
}

impl<'de> de::Deserializer<'de> for ConfigValueDeserializer<'_, '_, '_> {
    type Error = ConfigDeserializeError;

    /// Deserializes using the natural serde type for the stored value.
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.input {
            ConfigConversionInput::AdmittedScalar(admission) => {
                let value = admission
                    .convert::<String>()
                    .map_err(|error| map_admitted_error(&self.key, error))?;
                visitor.visit_string(value)
            }
            ConfigConversionInput::TopLevel { value, session } => match value {
                Value::Null => visitor.visit_unit(),
                Value::Bool(value) => visitor.visit_bool(value),
                Value::Number(value) => {
                    if let Some(value) = value.as_i64() {
                        visitor.visit_i64(value)
                    } else if let Some(value) = value.as_u64() {
                        visitor.visit_u64(value)
                    } else {
                        visitor.visit_f64(value.as_f64().expect("JSON numbers are finite"))
                    }
                }
                Value::String(value) => visitor.visit_string(convert_string_value(&self.key, session, &value)?),
                Value::Array(values) => {
                    visitor.visit_seq(ConfigSeqAccess::new(values, self.key, self.options, session))
                }
                Value::Object(values) => {
                    visitor.visit_map(ConfigMapAccess::new(values, self.key, self.options, session))
                }
            },
        }
    }

    /// Deserializes a boolean, accepting configured string literals.
    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.input {
            ConfigConversionInput::AdmittedScalar(admission) => visitor.visit_bool(
                admission
                    .convert::<bool>()
                    .map_err(|error| map_admitted_error(&self.key, error))?,
            ),
            ConfigConversionInput::TopLevel { value, session } => match value {
                Value::Bool(value) => visitor.visit_bool(value),
                Value::String(value) => visitor.visit_bool(convert_bool_value(&self.key, session, &value)?),
                other => Err(de::Error::invalid_type(
                    unexpected_value(&other),
                    &"a boolean-compatible scalar",
                )),
            },
        }
    }

    deserialize_number!(deserialize_i8, visit_i8, i8);
    deserialize_number!(deserialize_i16, visit_i16, i16);
    deserialize_number!(deserialize_i32, visit_i32, i32);
    deserialize_number!(deserialize_i64, visit_i64, i64);
    deserialize_number!(deserialize_i128, visit_i128, i128);
    deserialize_number!(deserialize_u8, visit_u8, u8);
    deserialize_number!(deserialize_u16, visit_u16, u16);
    deserialize_number!(deserialize_u32, visit_u32, u32);
    deserialize_number!(deserialize_u64, visit_u64, u64);
    deserialize_number!(deserialize_u128, visit_u128, u128);
    deserialize_number!(deserialize_f32, visit_f32, f32);
    deserialize_number!(deserialize_f64, visit_f64, f64);

    /// Deserializes a character using configured string normalization.
    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.input {
            ConfigConversionInput::AdmittedScalar(admission) => visitor.visit_char(
                admission
                    .convert::<char>()
                    .map_err(|error| map_admitted_error(&self.key, error))?,
            ),
            ConfigConversionInput::TopLevel { value, session } => match value {
                Value::String(value) => visitor.visit_char(convert_char_value(&self.key, session, &value)?),
                other => Err(de::Error::invalid_type(
                    unexpected_value(&other),
                    &"a single character string",
                )),
            },
        }
    }

    /// Deserializes a string-compatible scalar.
    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_string(self.scalar_to_string()?)
    }

    /// Deserializes a string-compatible scalar.
    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_string(self.scalar_to_string()?)
    }

    /// Deserializes bytes from a string.
    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_byte_buf(self.scalar_to_string()?.into_bytes())
    }

    /// Deserializes bytes from a string.
    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_byte_buf(self.scalar_to_string()?.into_bytes())
    }

    /// Deserializes an option.
    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.input {
            ConfigConversionInput::TopLevel { value: Value::Null, .. } => visitor.visit_none(),
            input => visitor.visit_some(Self {
                input,
                key: self.key,
                options: self.options,
            }),
        }
    }

    /// Deserializes unit from null.
    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.input {
            ConfigConversionInput::TopLevel { value: Value::Null, .. } => visitor.visit_unit(),
            ConfigConversionInput::TopLevel { value, .. } => {
                Err(de::Error::invalid_type(unexpected_value(&value), &"unit"))
            }
            ConfigConversionInput::AdmittedScalar(_) => Err(de::Error::invalid_type(
                de::Unexpected::Other("admitted scalar"),
                &"unit",
            )),
        }
    }

    /// Deserializes unit struct.
    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    /// Deserializes a newtype struct.
    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    /// Deserializes a sequence; scalar strings become one or more items based
    /// on the active collection conversion policy.
    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.input {
            ConfigConversionInput::TopLevel {
                value: Value::Array(values),
                session,
            } => visitor.visit_seq(ConfigSeqAccess::new(values, self.key, self.options, session)),
            ConfigConversionInput::TopLevel {
                value: Value::String(value),
                session,
            } => {
                let values = admit_scalar_items(&self.key, &value, self.options, session)?;
                visitor.visit_seq(ConfigScalarSeqAccess::new(values, self.key, self.options, session))
            }
            ConfigConversionInput::TopLevel { value, .. } => {
                Err(de::Error::invalid_type(unexpected_value(&value), &"a sequence"))
            }
            ConfigConversionInput::AdmittedScalar(_) => Err(de::Error::invalid_type(
                de::Unexpected::Other("admitted scalar"),
                &"a sequence",
            )),
        }
    }

    /// Deserializes a tuple.
    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    /// Deserializes a tuple struct.
    fn deserialize_tuple_struct<V>(self, _name: &'static str, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_tuple(len, visitor)
    }

    /// Deserializes a map.
    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.input {
            ConfigConversionInput::TopLevel {
                value: Value::Object(values),
                session,
            } => visitor.visit_map(ConfigMapAccess::new(values, self.key, self.options, session)),
            ConfigConversionInput::TopLevel { value, .. } => {
                Err(de::Error::invalid_type(unexpected_value(&value), &"a map"))
            }
            ConfigConversionInput::AdmittedScalar(_) => Err(de::Error::invalid_type(
                de::Unexpected::Other("admitted scalar"),
                &"a map",
            )),
        }
    }

    /// Deserializes a struct.
    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    /// Deserializes an enum using config string conversion semantics.
    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.input {
            ConfigConversionInput::AdmittedScalar(admission) => {
                let (variant, session) = admission
                    .convert_with_session::<String>()
                    .map_err(|error| map_admitted_error(&self.key, error))?;
                visitor.visit_enum(ConfigEnumAccess::new(variant, None, self.key, self.options, session))
            }
            ConfigConversionInput::TopLevel {
                value: Value::String(value),
                session,
            } => {
                let variant = convert_string_value(&self.key, session, &value)?;
                visitor.visit_enum(ConfigEnumAccess::new(variant, None, self.key, self.options, session))
            }
            ConfigConversionInput::TopLevel {
                value: Value::Object(values),
                session,
            } => {
                let mut entries = values.into_iter();
                let Some((variant, value)) = entries.next() else {
                    return Err(de::Error::invalid_value(
                        de::Unexpected::Map,
                        &"an enum object with exactly one variant",
                    ));
                };
                if entries.next().is_some() {
                    return Err(de::Error::invalid_value(
                        de::Unexpected::Map,
                        &"an enum object with exactly one variant",
                    ));
                }
                visitor.visit_enum(ConfigEnumAccess::new(
                    variant,
                    Some(value),
                    self.key,
                    self.options,
                    session,
                ))
            }
            ConfigConversionInput::TopLevel { value, .. } => Err(de::Error::invalid_type(
                unexpected_value(&value),
                &"a string enum variant or externally tagged enum object",
            )),
        }
    }

    /// Deserializes an identifier.
    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    /// Deserializes ignored values.
    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }
}

/// Classifies a serde value for type error diagnostics.
fn unexpected_value(value: &Value) -> de::Unexpected<'static> {
    match value {
        Value::Null => de::Unexpected::Unit,
        Value::Bool(value) => de::Unexpected::Bool(*value),
        Value::Number(_) => de::Unexpected::Other("number"),
        Value::String(_) => de::Unexpected::Other("string"),
        Value::Array(_) => de::Unexpected::Seq,
        Value::Object(_) => de::Unexpected::Map,
    }
}
