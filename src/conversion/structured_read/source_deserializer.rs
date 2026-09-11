// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Serde requests convert the original borrowed scalar exactly once.

use qubit_datatype::AdmittedScalarItem;
use qubit_datatype::ConversionSession;
use qubit_datatype::DataConversionError;
use qubit_datatype::DataConversionTarget;
use qubit_datatype::DataConverter;
use qubit_datatype::DataListConversionError;
use qubit_datatype::DataType;
use qubit_value::MultiValues;
use qubit_value::Value;
use qubit_value::ValueError;
use qubit_value::ValueMissingReason;
use qubit_value::ValueRef;
use serde::de;
use serde::de::IntoDeserializer;
use serde::de::Visitor;

use super::conversion_input::ConversionInput;
use super::node::SourceLocation;
use super::node::SourceSegment;
pub(super) use super::read_context::ReadContext;
use super::traversal::ReadView;
use crate::ConfigError;
use crate::config_deserialize_error::ConfigDeserializeError;

pub(super) struct SourceDeserializer<'tree, 'policy, 'session> {
    /// Prepared-read context.
    pub(super) context: ReadContext<'tree>,
    /// Borrowed or already admitted conversion input.
    pub(super) input: ConversionInput<'tree, 'policy, 'session>,
    /// Optional source location for diagnostics.
    pub(super) location: Option<SourceLocation>,
    /// Root-relative diagnostic path.
    pub(super) path: String,
    /// Whether scalar text may be split into sequence items.
    pub(super) allow_split: bool,
    /// Original source item index, when known.
    pub(super) original_index: Option<usize>,
}

impl<'tree, 'policy: 'tree, 'session> SourceDeserializer<'tree, 'policy, 'session> {
    /// Creates a deserializer for a prepared value.
    pub(super) fn new(
        context: ReadContext<'tree>,
        value: ReadView<'tree>,
        location: Option<SourceLocation>,
        path: String,
        allow_split: bool,
        session: &'session mut ConversionSession<'policy>,
    ) -> Self {
        let original_index = source_index(location.as_ref());
        let location = if context.needs_locations() || matches!(value, ReadView::Missing(_)) {
            location
        } else {
            None
        };
        Self {
            context,
            input: ConversionInput::Borrowed { value, session },
            location,
            path,
            allow_split,
            original_index,
        }
    }

    pub(super) fn with_source_index(mut self, index: Option<usize>) -> Self {
        self.original_index = index.or(self.original_index);
        self
    }

    pub(super) fn admitted(
        context: ReadContext<'tree>,
        path: String,
        item: AdmittedScalarItem<'session, 'policy, 'tree>,
    ) -> Self {
        let original_index = Some(item.source_index());
        Self {
            context,
            input: ConversionInput::Admitted(item),
            location: None,
            path,
            allow_split: false,
            original_index,
        }
    }

    fn view(&self) -> Option<ReadView<'tree>> {
        match &self.input {
            ConversionInput::Borrowed { value, .. } => Some(*value),
            ConversionInput::Admitted(_) => None,
        }
    }

    fn string(&self) -> Option<&'tree str> {
        self.location
            .as_ref()
            .and_then(|location| self.context.prepared.overlays.get(location))
            .map(String::as_str)
            .or_else(|| self.view()?.string())
    }

    fn is_missing(&self) -> bool {
        matches!(
            self.view(),
            Some(ReadView::Missing(_) | ReadView::Json(serde_json::Value::Null) | ReadView::Scalar(ValueRef::Unset(_)))
        )
    }

    pub(super) fn convert<T: DataConversionTarget>(self) -> Result<T, ConfigDeserializeError> {
        let index = self.original_index;
        let text = self.string();
        match self.input {
            ConversionInput::Admitted(item) => {
                let index = item.source_index();
                item.convert::<T>()
                    .map_err(|error| conversion_error(&self.path, Some(index), error))
            }
            ConversionInput::Borrowed { value, session } => {
                if let ReadView::Missing(missing) = value {
                    let source_type = missing.source_type().unwrap_or(DataType::Json);
                    let result = match missing.reason() {
                        ValueMissingReason::UnsetCollection => {
                            Some(MultiValues::Unset(source_type).to_first_in::<T>(session))
                        }
                        ValueMissingReason::UnsetScalar => Some(Value::Unset(source_type).to_in::<T>(session)),
                        _ => None,
                    };
                    if let Some(result) = result {
                        return result.map_err(|error| {
                            ConfigDeserializeError::from_config(ConfigError::from((self.path.as_str(), error)))
                        });
                    }
                }
                let converter = if let Some(text) = text {
                    DataConverter::from(text)
                } else {
                    scalar_converter(value, self.context, self.location.as_ref())
                        .ok_or_else(|| type_error(&self.path, "a scalar"))?
                };
                converter
                    .to_in::<T>(session)
                    .map_err(|error| conversion_error(&self.path, index, error))
            }
        }
    }

    fn sequence<'de, V: Visitor<'de>>(
        self,
        visitor: V,
        expected_len: Option<usize>,
    ) -> Result<V::Value, ConfigDeserializeError> {
        let text = self.string();
        let ConversionInput::Borrowed { value, session } = self.input else {
            return Err(type_error(&self.path, "a sequence"));
        };
        if let Some(values) = super::seq_access::SequenceValues::from_view(value) {
            if expected_len.is_some_and(|length| length != values.len()) {
                return Err(type_error(&self.path, "a tuple with the requested length"));
            }
            return visitor.visit_seq(super::seq_access::SourceSeqAccess::new(
                self.context,
                values,
                self.location,
                self.path,
                self.original_index,
                session,
            ));
        }
        if self.allow_split
            && let Some(text) = text
        {
            let mut access =
                super::scalar_seq_access::ScalarSeqAccess::new(self.context, text, self.path.clone(), session)?;
            let result = visitor.visit_seq(&mut access)?;
            if expected_len.is_some() && access.has_remaining()? {
                return Err(type_error(&self.path, "a tuple with the requested length"));
            }
            return Ok(result);
        }
        Err(type_error(&self.path, "a sequence"))
    }
}

fn scalar_converter<'tree>(
    value: ReadView<'tree>,
    context: ReadContext<'tree>,
    location: Option<&SourceLocation>,
) -> Option<DataConverter<'tree>> {
    match value {
        ReadView::Scalar(value) => Some(DataConverter::from(value)),
        ReadView::Text(value) => Some(DataConverter::from(value)),
        ReadView::Json(serde_json::Value::String(value)) => Some(DataConverter::from(value.as_str())),
        ReadView::Json(serde_json::Value::Bool(value)) => Some(DataConverter::from(*value)),
        ReadView::Json(serde_json::Value::Number(value)) => Some(json_number_converter(value)),
        ReadView::Json(serde_json::Value::Null) => Some(DataConverter::Unset(DataType::Json)),
        ReadView::Missing(_) => {
            let origin = location?;
            let property = context.prepared.sources.property(origin.property_index)?;
            Some(DataConverter::from(property.value().as_scalar()?.view()))
        }
        _ => None,
    }
}

fn json_number_converter(value: &serde_json::Number) -> DataConverter<'_> {
    if let Some(value) = value.as_i64() {
        DataConverter::from(value)
    } else if let Some(value) = value.as_u64() {
        DataConverter::from(value)
    } else {
        DataConverter::from(value.as_f64().expect("admitted finite JSON number"))
    }
}

pub(super) fn source_index(location: Option<&SourceLocation>) -> Option<usize> {
    location?.segments.iter().find_map(|segment| match segment {
        SourceSegment::Index(index) => Some(*index),
        SourceSegment::Key(_) => None,
    })
}

pub(super) fn conversion_error(path: &str, index: Option<usize>, error: DataConversionError) -> ConfigDeserializeError {
    let source = match index {
        Some(index) => ValueError::from(DataListConversionError::new(index, error)),
        None => ValueError::from(error),
    };
    ConfigDeserializeError::from_config(ConfigError::from((path, source)))
}

pub(super) fn type_error(path: &str, expected: &'static str) -> ConfigDeserializeError {
    <ConfigDeserializeError as de::Error>::invalid_type(de::Unexpected::Other("configuration source"), &expected)
        .with_path(path.to_owned())
}

macro_rules! typed {
    ($method:ident, $visit:ident, $type:ty) => {
        fn $method<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
            visitor.$visit(self.convert::<$type>()?)
        }
    };
}

impl<'de, 'tree, 'policy: 'tree> de::Deserializer<'de> for SourceDeserializer<'tree, 'policy, '_> {
    type Error = ConfigDeserializeError;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.view() {
            Some(
                ReadView::Missing(_) | ReadView::Json(serde_json::Value::Null) | ReadView::Scalar(ValueRef::Unset(_)),
            ) => visitor.visit_unit(),
            Some(ReadView::Scalar(ValueRef::Bool(_)) | ReadView::Json(serde_json::Value::Bool(_))) => {
                visitor.visit_bool(self.convert::<bool>()?)
            }
            Some(ReadView::Scalar(ValueRef::Int8(_))) => visitor.visit_i64(i64::from(self.convert::<i8>()?)),
            Some(ReadView::Scalar(ValueRef::Int16(_))) => visitor.visit_i64(i64::from(self.convert::<i16>()?)),
            Some(ReadView::Scalar(ValueRef::Int32(_))) => visitor.visit_i64(i64::from(self.convert::<i32>()?)),
            Some(ReadView::Scalar(ValueRef::Int64(_))) => visitor.visit_i64(self.convert::<i64>()?),
            Some(ReadView::Scalar(ValueRef::UInt8(_))) => visitor.visit_u64(u64::from(self.convert::<u8>()?)),
            Some(ReadView::Scalar(ValueRef::UInt16(_))) => visitor.visit_u64(u64::from(self.convert::<u16>()?)),
            Some(ReadView::Scalar(ValueRef::UInt32(_))) => visitor.visit_u64(u64::from(self.convert::<u32>()?)),
            Some(ReadView::Scalar(ValueRef::UInt64(_))) => visitor.visit_u64(self.convert::<u64>()?),
            Some(ReadView::Scalar(ValueRef::Float32(_))) => {
                let value = self.convert::<f32>()?;
                visitor.visit_f64(value.to_string().parse::<f64>().expect("admitted finite f32"))
            }
            Some(ReadView::Scalar(ValueRef::Float64(_))) => visitor.visit_f64(self.convert::<f64>()?),
            Some(ReadView::Json(serde_json::Value::Number(value))) => {
                if value.as_i64().is_some() {
                    visitor.visit_i64(self.convert::<i64>()?)
                } else if value.as_u64().is_some() {
                    visitor.visit_u64(self.convert::<u64>()?)
                } else {
                    visitor.visit_f64(self.convert::<f64>()?)
                }
            }
            Some(ReadView::Collection(_) | ReadView::Json(serde_json::Value::Array(_))) => {
                self.deserialize_seq(visitor)
            }
            Some(ReadView::Object(_) | ReadView::StringMap(_) | ReadView::Json(serde_json::Value::Object(_))) => {
                self.deserialize_map(visitor)
            }
            _ => visitor.visit_string(self.convert::<String>()?),
        }
    }

    typed!(deserialize_bool, visit_bool, bool);
    typed!(deserialize_i8, visit_i8, i8);
    typed!(deserialize_i16, visit_i16, i16);
    typed!(deserialize_i32, visit_i32, i32);
    typed!(deserialize_i64, visit_i64, i64);
    typed!(deserialize_i128, visit_i128, i128);
    typed!(deserialize_u8, visit_u8, u8);
    typed!(deserialize_u16, visit_u16, u16);
    typed!(deserialize_u32, visit_u32, u32);
    typed!(deserialize_u64, visit_u64, u64);
    typed!(deserialize_u128, visit_u128, u128);
    typed!(deserialize_f32, visit_f32, f32);
    typed!(deserialize_f64, visit_f64, f64);
    typed!(deserialize_char, visit_char, char);
    typed!(deserialize_str, visit_string, String);
    typed!(deserialize_string, visit_string, String);

    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_byte_buf(self.convert::<String>()?.into_bytes())
    }
    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_bytes(visitor)
    }
    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        if self.is_missing() {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }
    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        if self.is_missing() {
            visitor.visit_unit()
        } else {
            Err(type_error(&self.path, "unit"))
        }
    }
    fn deserialize_unit_struct<V: Visitor<'de>>(self, _: &'static str, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_unit(visitor)
    }
    fn deserialize_newtype_struct<V: Visitor<'de>>(self, _: &'static str, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_newtype_struct(self)
    }
    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.sequence(visitor, None)
    }
    fn deserialize_tuple<V: Visitor<'de>>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error> {
        self.sequence(visitor, Some(len))
    }
    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        _: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_tuple(len, visitor)
    }
    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let ConversionInput::Borrowed { value, session } = self.input else {
            return Err(type_error(&self.path, "a map"));
        };
        let entries = super::map_access::MapEntries::new(value).ok_or_else(|| type_error(&self.path, "a map"))?;
        visitor.visit_map(super::map_access::SourceMapAccess::new(
            self.context,
            entries,
            self.location,
            self.path,
            self.original_index,
            session,
        ))
    }
    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _: &'static str,
        _: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_map(visitor)
    }
    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _: &'static str,
        _: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        if self.string().is_some() || matches!(self.input, ConversionInput::Admitted(_)) {
            let value = self.convert::<String>()?;
            return visitor.visit_enum(value.into_deserializer());
        }
        let ConversionInput::Borrowed { value, session } = self.input else {
            return Err(type_error(&self.path, "an enum"));
        };
        let mut entries = super::map_access::MapEntries::new(value).ok_or_else(|| type_error(&self.path, "an enum"))?;
        let (key, value, origin) = entries
            .next(self.context)
            .ok_or_else(|| type_error(&self.path, "a single enum variant"))?;
        if entries.next(self.context).is_some() {
            return Err(type_error(&self.path, "a single enum variant"));
        }
        let location = origin.or_else(|| {
            self.location
                .as_ref()
                .map(|origin| origin.child(SourceSegment::Key(key.to_owned())))
        });
        let path = super::path::child_key(&self.path, key);
        let payload =
            Self::new(self.context, value, location, path, true, session).with_source_index(self.original_index);
        visitor.visit_enum(super::enum_access::SourceEnumAccess { variant: key, payload })
    }
    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_str(visitor)
    }
    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_unit()
    }
}
