// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Serde requests convert the original borrowed scalar exactly once.
//!
//! Typed requests use the requested target type. `deserialize_any` instead
//! preserves the JSON value's natural Serde category: signed and unsigned
//! integers use their native 64-bit visitor methods when representable, wider
//! 128-bit integers are projected as decimal strings, and fractional values
//! use `f64`. An integer outside the supported 128-bit range or a non-finite
//! float is reported as a conversion error.

use qubit_datatype::AdmittedScalarItem;
use qubit_datatype::ConversionSession;
use qubit_datatype::DataConversionError;
use qubit_datatype::DataConversionTarget;
use qubit_datatype::DataConverter;
use qubit_datatype::DataListConversionError;
use qubit_datatype::DataType;
use qubit_datatype::InvalidValueReason;
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

/// Deserializes one value while carrying optional original-source context and
/// the original outer collection index.
///
/// Borrowed inputs read payloads from the prepared tree and charge target
/// conversion to their session. Admitted inputs already carry a scalar item
/// and its charged session/index. Overlay text is selected through the typed
/// source location; the dotted diagnostic path is maintained separately.
pub(super) struct SourceDeserializer<'tree, 'policy, 'session> {
    /// Prepared-read context.
    pub(super) context: ReadContext<'tree>,
    /// Borrowed or already admitted conversion input.
    pub(super) input: ConversionInput<'tree, 'policy, 'session>,
    /// Optional original-tree location for interpolation overlays and missing
    /// values. Its property-table index and typed nested key/index segments
    /// are distinct from both the arena node ID and the diagnostic path.
    /// Ordinary reads omit it when neither needs source context.
    pub(super) location: Option<SourceLocation>,
    /// Root-relative diagnostic path.
    pub(super) path: String,
    /// Whether scalar text may be split into sequence items.
    pub(super) allow_split: bool,
    /// Original outer collection index, when known; nested traversal preserves
    /// this value for conversion diagnostics even as `path` gains nested
    /// segments.
    pub(super) original_index: Option<usize>,
}

impl<'tree, 'policy: 'tree, 'session> SourceDeserializer<'tree, 'policy, 'session> {
    /// Creates a deserializer for a prepared value.
    ///
    /// Source locations are retained when interpolation overlays exist or the
    /// value is missing, since those cases need the original property context.
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
                        .map_err(|reason| {
                            conversion_error(
                                &self.path,
                                index,
                                DataConversionError::invalid(DataType::Json, T::DATA_TYPE, reason),
                            )
                        })?
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
            if let Some(expected) = expected_len
                && expected != values.len()
            {
                return Err(ConfigDeserializeError::tuple_length(&self.path, values.len(), expected));
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
            if let Some(expected) = expected_len
                && access.has_remaining()?
            {
                return Err(ConfigDeserializeError::tuple_has_extra_items(&self.path, expected));
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
) -> Result<Option<DataConverter<'tree>>, InvalidValueReason> {
    match value {
        ReadView::Scalar(value) => Ok(Some(DataConverter::from(value))),
        ReadView::Text(value) => Ok(Some(DataConverter::from(value))),
        ReadView::Json(serde_json::Value::String(value)) => Ok(Some(DataConverter::from(value.as_str()))),
        ReadView::Json(serde_json::Value::Bool(value)) => Ok(Some(DataConverter::from(*value))),
        ReadView::Json(serde_json::Value::Number(value)) => json_number_converter(value).map(Some),
        ReadView::Json(serde_json::Value::Null) => Ok(Some(DataConverter::Unset(DataType::Json))),
        ReadView::Missing(_) => {
            let Some(origin) = location else {
                return Ok(None);
            };
            let Some(property) = context.prepared.sources.property(origin.property_index) else {
                return Ok(None);
            };
            let Some(value) = property.value().as_scalar() else {
                return Ok(None);
            };
            Ok(Some(DataConverter::from(value.view())))
        }
        _ => Ok(None),
    }
}

fn json_number_converter(value: &serde_json::Number) -> Result<DataConverter<'_>, InvalidValueReason> {
    if let Some(value) = value.as_i64() {
        return Ok(DataConverter::from(value));
    }
    if let Some(value) = value.as_u64() {
        return Ok(DataConverter::from(value));
    }

    let text = value.to_string();
    if !text.contains('.') && !text.contains('e') && !text.contains('E') {
        if text.starts_with('-') {
            return text
                .parse::<i128>()
                .map(DataConverter::from)
                .map_err(|_| InvalidValueReason::OutOfRange);
        }
        return text
            .parse::<u128>()
            .map(DataConverter::from)
            .map_err(|_| InvalidValueReason::OutOfRange);
    }

    value
        .as_f64()
        .filter(|number| number.is_finite())
        .map(DataConverter::from)
        .ok_or(InvalidValueReason::NonFinite)
}

/// Returns the first original array index, which is the source item's index
/// retained for error reporting even when the location is nested further.
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
    ConfigDeserializeError::shape_mismatch(path, expected)
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
                match json_number_converter(value).map_err(|reason| {
                    let text = value.to_string();
                    let is_integer = !text.contains('.') && !text.contains('e') && !text.contains('E');
                    let target = if !is_integer {
                        DataType::Float64
                    } else if text.starts_with('-') {
                        DataType::Int128
                    } else {
                        DataType::UInt128
                    };
                    conversion_error(
                        &self.path,
                        self.original_index,
                        DataConversionError::invalid(DataType::Json, target, reason),
                    )
                })? {
                    DataConverter::Int8(_)
                    | DataConverter::Int16(_)
                    | DataConverter::Int32(_)
                    | DataConverter::Int64(_) => visitor.visit_i64(self.convert::<i64>()?),
                    DataConverter::UInt8(_)
                    | DataConverter::UInt16(_)
                    | DataConverter::UInt32(_)
                    | DataConverter::UInt64(_) => visitor.visit_u64(self.convert::<u64>()?),
                    DataConverter::Int128(_) | DataConverter::UInt128(_) => {
                        visitor.visit_string(self.convert::<String>()?)
                    }
                    DataConverter::Float32(_) | DataConverter::Float64(_) => visitor.visit_f64(self.convert::<f64>()?),
                    _ => visitor.visit_string(self.convert::<String>()?),
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
        if let Some(paths) = self.context.ignored_paths {
            paths.borrow_mut().push(self.path.clone());
        }
        visitor.visit_unit()
    }
}

#[cfg(test)]
mod tests {
    use super::super::prepared_read::prepare_read;
    use super::ReadContext;
    use super::ReadView;
    use super::json_number_converter;
    use super::scalar_converter;
    use crate::Config;

    #[test]
    fn scalar_conversion_accepts_borrowed_text_and_json_primitives() {
        let config = Config::new();
        let prepared = prepare_read(&config, "", false).unwrap();
        let context = ReadContext {
            prepared: &prepared,
            ignored_paths: None,
        };
        let json_string = serde_json::Value::String("json".to_owned());
        let json_bool = serde_json::Value::Bool(true);
        let json_number = serde_json::json!(1.25);
        let json_null = serde_json::Value::Null;
        let json_array = serde_json::json!([]);
        let values = [
            ReadView::Text("text"),
            ReadView::Json(&json_string),
            ReadView::Json(&json_bool),
            ReadView::Json(&json_number),
            ReadView::Json(&json_null),
        ];
        for value in values {
            assert!(scalar_converter(value, context, None).unwrap().is_some());
        }
        assert!(
            scalar_converter(ReadView::Json(&json_array), context, None)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn json_number_conversion_preserves_signed_unsigned_and_fractional_values() {
        let signed = serde_json::Number::from(-17_i64);
        let unsigned = serde_json::Number::from(u64::MAX);
        let fractional = serde_json::Number::from_f64(1.25).unwrap();
        assert!(json_number_converter(&signed).is_ok());
        assert!(json_number_converter(&unsigned).is_ok());
        assert!(json_number_converter(&fractional).is_ok());
    }
}
