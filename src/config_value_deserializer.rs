/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Serde deserializer that applies configuration read conversion semantics.

use qubit_value::Value as QubitValue;
use serde::de::{self, IntoDeserializer, MapAccess, SeqAccess, Visitor, value::StringDeserializer};
use serde_json::{Map, Value};

use crate::ConfigError;
use crate::config_deserialize_error::ConfigDeserializeError;
use crate::options::ConfigReadOptions;

/// Deserializer over a single serde value.
pub(crate) struct ConfigValueDeserializer<'a> {
    value: Value,
    key: String,
    options: &'a ConfigReadOptions,
}

impl<'a> ConfigValueDeserializer<'a> {
    /// Creates a value deserializer.
    pub(crate) fn new(value: Value, key: String, options: &'a ConfigReadOptions) -> Self {
        Self {
            value,
            key,
            options,
        }
    }

    /// Converts any scalar value into a string using config read semantics.
    fn scalar_to_string(&self) -> Result<String, ConfigDeserializeError> {
        match &self.value {
            Value::String(value) => convert_string_value(&self.key, self.options, value),
            Value::Bool(value) => Ok(value.to_string()),
            Value::Number(value) => Ok(value.to_string()),
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
        }
    }
}

/// Converts a scalar string using the shared conversion layer.
fn convert_string_value(
    key: &str,
    options: &ConfigReadOptions,
    value: &str,
) -> Result<String, ConfigDeserializeError> {
    match QubitValue::String(value.to_string()).to_with::<String>(options.conversion_options()) {
        Ok(value) => Ok(value),
        Err(error) => Err(de::Error::custom(ConfigError::from((key, error)))),
    }
}

/// Converts a scalar string into a boolean using the shared conversion layer.
fn convert_bool_value(
    key: &str,
    options: &ConfigReadOptions,
    value: &str,
) -> Result<bool, ConfigDeserializeError> {
    match QubitValue::String(value.to_string()).to_with::<bool>(options.conversion_options()) {
        Ok(value) => Ok(value),
        Err(error) => Err(de::Error::custom(ConfigError::from((key, error)))),
    }
}

/// Converts a JSON number or string into the scalar text consumed by
/// `qubit-value` conversion.
fn number_scalar_text(
    value: Value,
    expected: &'static str,
) -> Result<String, ConfigDeserializeError> {
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
            let value = number_scalar_text(self.value, stringify!($ty))?;
            let value =
                crate::config::convert_deserialize_number::<$ty>(&self.key, self.options, value)
                    .map_err(de::Error::custom)?;
            visitor.$visit(value)
        }
    };
}

impl<'de> de::Deserializer<'de> for ConfigValueDeserializer<'_> {
    type Error = ConfigDeserializeError;

    /// Deserializes using the natural serde type for the stored value.
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
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
            Value::String(value) => {
                visitor.visit_string(convert_string_value(&self.key, self.options, &value)?)
            }
            Value::Array(values) => {
                visitor.visit_seq(ConfigSeqAccess::new(values, self.key, self.options))
            }
            Value::Object(values) => {
                visitor.visit_map(ConfigMapAccess::new(values, self.key, self.options))
            }
        }
    }

    /// Deserializes a boolean, accepting configured string literals.
    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Bool(value) => visitor.visit_bool(value),
            Value::String(value) => {
                visitor.visit_bool(convert_bool_value(&self.key, self.options, &value)?)
            }
            other => Err(de::Error::invalid_type(
                unexpected_value(&other),
                &"a boolean-compatible scalar",
            )),
        }
    }

    deserialize_number!(deserialize_i8, visit_i8, i8);
    deserialize_number!(deserialize_i16, visit_i16, i16);
    deserialize_number!(deserialize_i32, visit_i32, i32);
    deserialize_number!(deserialize_i64, visit_i64, i64);
    deserialize_number!(deserialize_u8, visit_u8, u8);
    deserialize_number!(deserialize_u16, visit_u16, u16);
    deserialize_number!(deserialize_u32, visit_u32, u32);
    deserialize_number!(deserialize_u64, visit_u64, u64);
    deserialize_number!(deserialize_f32, visit_f32, f32);
    deserialize_number!(deserialize_f64, visit_f64, f64);

    /// Deserializes a character using configured string normalization.
    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::String(value) => {
                let value = convert_string_value(&self.key, self.options, &value)?;
                let mut chars = value.chars();
                let Some(ch) = chars.next() else {
                    return Err(de::Error::invalid_value(
                        de::Unexpected::Str(&value),
                        &"a single character",
                    ));
                };
                if chars.next().is_some() {
                    return Err(de::Error::invalid_value(
                        de::Unexpected::Str(&value),
                        &"a single character",
                    ));
                }
                visitor.visit_char(ch)
            }
            other => Err(de::Error::invalid_type(
                unexpected_value(&other),
                &"a single character string",
            )),
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
        match self.value {
            Value::Null => visitor.visit_none(),
            value => {
                visitor.visit_some(ConfigValueDeserializer::new(value, self.key, self.options))
            }
        }
    }

    /// Deserializes unit from null.
    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Null => visitor.visit_unit(),
            other => Err(de::Error::invalid_type(unexpected_value(&other), &"unit")),
        }
    }

    /// Deserializes unit struct.
    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    /// Deserializes a newtype struct.
    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    /// Deserializes a sequence; scalar strings become one or more items based
    /// on collection read options.
    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Array(values) => {
                visitor.visit_seq(ConfigSeqAccess::new(values, self.key, self.options))
            }
            Value::String(value) => {
                let normalized = match self.options.string.normalize(&value) {
                    Ok(value) => value,
                    Err(error) => {
                        return Err(de::Error::custom(ConfigError::from_data_conversion_error(
                            &self.key, error,
                        )));
                    }
                };
                let values = match self.options.collection.scalar_items(&normalized) {
                    Ok(values) => values,
                    Err(error) => return Err(de::Error::custom(error.to_string())),
                }
                .into_iter()
                .map(Value::String)
                .collect();
                visitor.visit_seq(ConfigSeqAccess::new(values, self.key, self.options))
            }
            other => Err(de::Error::invalid_type(
                unexpected_value(&other),
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
    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
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
        match self.value {
            Value::Object(values) => {
                visitor.visit_map(ConfigMapAccess::new(values, self.key, self.options))
            }
            other => Err(de::Error::invalid_type(unexpected_value(&other), &"a map")),
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

    /// Deserializes an enum using serde_json-compatible semantics.
    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value.deserialize_enum(name, variants, visitor) {
            Ok(value) => Ok(value),
            Err(error) => Err(de::Error::custom(error.to_string())),
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

/// Sequence access over configuration values.
struct ConfigSeqAccess<'a> {
    values: std::vec::IntoIter<Value>,
    key: String,
    index: usize,
    options: &'a ConfigReadOptions,
}

impl<'a> ConfigSeqAccess<'a> {
    /// Creates sequence access.
    fn new(values: Vec<Value>, key: String, options: &'a ConfigReadOptions) -> Self {
        Self {
            values: values.into_iter(),
            key,
            index: 0,
            options,
        }
    }
}

impl<'de> SeqAccess<'de> for ConfigSeqAccess<'_> {
    type Error = ConfigDeserializeError;

    /// Deserializes the next element.
    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        let Some(value) = self.values.next() else {
            return Ok(None);
        };
        let key = format!("{}[{}]", self.key, self.index);
        self.index += 1;
        seed.deserialize(ConfigValueDeserializer::new(value, key, self.options))
            .map(Some)
    }
}

/// Map access over configuration objects.
struct ConfigMapAccess<'a> {
    entries: std::vec::IntoIter<(String, Value)>,
    next_value: Option<(String, Value)>,
    key: String,
    options: &'a ConfigReadOptions,
}

impl<'a> ConfigMapAccess<'a> {
    /// Creates map access.
    fn new(values: Map<String, Value>, key: String, options: &'a ConfigReadOptions) -> Self {
        Self {
            entries: values.into_iter().collect::<Vec<_>>().into_iter(),
            next_value: None,
            key,
            options,
        }
    }
}

impl<'de> MapAccess<'de> for ConfigMapAccess<'_> {
    type Error = ConfigDeserializeError;

    /// Deserializes the next key.
    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        let Some((key, value)) = self.entries.next() else {
            return Ok(None);
        };
        let key_deserializer: StringDeserializer<Self::Error> = key.clone().into_deserializer();
        self.next_value = Some((key, value));
        seed.deserialize(key_deserializer).map(Some)
    }

    /// Deserializes the value for the last key.
    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        let (key, value) = self
            .next_value
            .take()
            .expect("map value requested before key");
        let child_key = if self.key.is_empty() {
            key
        } else {
            format!("{}.{}", self.key, key)
        };
        seed.deserialize(ConfigValueDeserializer::new(value, child_key, self.options))
    }
}
