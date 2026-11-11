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
#[must_use = "the source deserializer must be passed to a Serde target"]
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
    ///
    /// # Parameters
    ///
    /// * `context` - Prepared read state, conversion policy, and overlay map.
    /// * `value` - Borrowed view of the value that Serde will request.
    /// * `location` - Optional typed location for source-aware diagnostics.
    /// * `path` - Root-relative diagnostic path for this value.
    /// * `allow_split` - Whether a scalar string may supply sequence items.
    /// * `session` - Shared conversion accounting state for this read.
    #[inline]
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

    /// Creates a deserializer from an item already admitted by the shared
    /// conversion session.
    ///
    /// The item's source index is retained for diagnostics, and its existing
    /// admission charge is not repeated during conversion.
    ///
    /// # Parameters
    ///
    /// * `context` - Prepared read state used for nested value resolution.
    /// * `path` - Root-relative diagnostic path for the admitted item.
    /// * `item` - Scalar item whose conversion admission is already charged.
    #[inline]
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

    /// Sets a known outer source index, preferring it over the index inferred
    /// from this value's source location.
    ///
    /// `None` preserves any index already inferred from the original source
    /// location. The index is retained for conversion diagnostics as nested
    /// paths are traversed.
    ///
    /// # Parameters
    ///
    /// * `index` - Known outer source collection position, if available.
    #[inline]
    pub(super) fn with_source_index(mut self, index: Option<usize>) -> Self {
        self.original_index = index.or(self.original_index);
        self
    }

    /// Converts this input to its original target type through the active
    /// conversion session, preserving source index and path in errors.
    ///
    /// Admitted items reuse their prior admission; borrowed values are
    /// converted from overlay text or their original scalar representation.
    /// Missing values retain their unset source type when the source records
    /// one.
    ///
    /// # Type Parameters
    ///
    /// * `T` - Requested target type supported by the conversion policy.
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

    /// Returns the borrowed prepared-tree view; admitted items have no tree
    /// view because their scalar and admission state are carried by the item.
    ///
    /// # Returns
    ///
    /// The prepared view for borrowed inputs, or `None` for admitted items.
    #[inline]
    fn view(&self) -> Option<ReadView<'tree>> {
        match &self.input {
            ConversionInput::Borrowed { value, .. } => Some(*value),
            ConversionInput::Admitted(_) => None,
        }
    }

    /// Returns text from the interpolation overlay when one exists for this
    /// source location, otherwise from the borrowed tree view.
    ///
    /// # Returns
    ///
    /// Overlay or source text when available, or `None` when the input has no
    /// string representation.
    #[inline]
    fn string(&self) -> Option<&'tree str> {
        self.location
            .as_ref()
            .and_then(|location| self.context.prepared.overlays.get(location))
            .map(String::as_str)
            .or_else(|| self.view()?.string())
    }

    /// Reports values represented as missing, JSON null, or an unset scalar.
    ///
    /// # Returns
    ///
    /// `true` when the input represents one of those absent-value states.
    #[must_use]
    #[inline]
    fn is_missing(&self) -> bool {
        matches!(
            self.view(),
            Some(ReadView::Missing(_) | ReadView::Json(serde_json::Value::Null) | ReadView::Scalar(ValueRef::Unset(_)))
        )
    }

    /// Deserializes either a stored collection or, when enabled, a scalar split
    /// into sequence items.
    ///
    /// For a stored collection, a requested tuple length is compared with the
    /// actual collection length before the visitor runs. A split scalar is
    /// consumed lazily: the visitor handles too few items, and after it returns
    /// an extra-item probe rejects any remainder. That probe may admit the next
    /// item through the shared conversion session. Nested item paths remain
    /// root-relative, while the original outer source index is retained
    /// separately for conversion errors.
    ///
    /// # Type Parameters
    ///
    /// * `V` - Serde visitor that constructs the requested sequence value.
    ///
    /// # Parameters
    ///
    /// * `visitor` - Visitor receiving source sequence elements.
    /// * `expected_len` - Required tuple length, or `None` for any length.
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

/// Builds a converter for scalar views and recovers an original scalar for a
/// missing prepared node when its source location identifies a scalar
/// property.
///
/// `Ok(None)` means the view is not convertible from a scalar, or that the
/// missing node has no usable source location/property/scalar. The caller then
/// reports the expected scalar shape; an invalid scalar representation is
/// returned as `Err` instead.
///
/// # Parameters
///
/// * `value` - Borrowed value view offered for scalar conversion.
/// * `context` - Prepared read context used to recover a missing scalar.
/// * `location` - Optional source location identifying the original property.
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

/// Builds a converter without losing the JSON number's signed or unsigned
/// integer domain. Integer values beyond the native 64-bit range are retained
/// as 128-bit integers when possible; fractional values use finite `f64`.
///
/// Returns `OutOfRange` for integral values outside the supported 128-bit
/// range and `NonFinite` when a fractional value cannot be represented as a
/// finite `f64`.
///
/// # Parameters
///
/// * `value` - JSON number whose integer domain or finite floating value is
///   retained in the converter.
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
///
/// # Parameters
///
/// * `location` - Optional typed source location to search for an array index.
pub(super) fn source_index(location: Option<&SourceLocation>) -> Option<usize> {
    location?.segments.iter().find_map(|segment| match segment {
        SourceSegment::Index(index) => Some(*index),
        SourceSegment::Key(_) => None,
    })
}

/// Wraps a conversion failure with its root-relative path and, when known, the
/// original outer source index.
///
/// The index is represented as a list-item conversion error without replacing
/// the nested diagnostic path. This preserves the distinction between where
/// traversal found the failure and which original collection item supplied
/// it.
///
/// # Parameters
///
/// * `path` - Root-relative diagnostic path where traversal found the failure.
/// * `index` - Original outer collection index, when known.
/// * `error` - Conversion failure to wrap with configuration context.
pub(super) fn conversion_error(path: &str, index: Option<usize>, error: DataConversionError) -> ConfigDeserializeError {
    let source = match index {
        Some(index) => ValueError::from(DataListConversionError::new(index, error)),
        None => ValueError::from(error),
    };
    ConfigDeserializeError::from_config(ConfigError::from((path, source)))
}

/// Creates a shape-mismatch error at a root-relative diagnostic path.
///
/// Unlike [`conversion_error`], this describes an unsupported input shape and
/// does not attach a source collection index.
///
/// # Parameters
///
/// * `path` - Root-relative path where the unsupported shape was found.
/// * `expected` - Human-readable description of the required input shape.
pub(super) fn type_error(path: &str, expected: &'static str) -> ConfigDeserializeError {
    ConfigDeserializeError::shape_mismatch(path, expected)
}

macro_rules! typed {
    ($method:ident, $visit:ident, $type:ty) => {
        /// Converts the source value to the requested primitive target type.
        ///
        /// # Type Parameters
        ///
        /// * `V` - Serde visitor that receives the converted value.
        ///
        /// # Parameters
        ///
        /// * `visitor` - Visitor for the requested primitive target.
        #[inline]
        fn $method<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
            visitor.$visit(self.convert::<$type>()?)
        }
    };
}

impl<'de, 'tree, 'policy: 'tree> de::Deserializer<'de> for SourceDeserializer<'tree, 'policy, '_> {
    /// Error that preserves configuration paths and source conversion context.
    type Error = ConfigDeserializeError;

    /// Deserializes using the natural Serde category of the source value.
    ///
    /// # Type Parameters
    ///
    /// * `V` - Visitor that constructs the requested output value.
    ///
    /// # Parameters
    ///
    /// * `visitor` - Visitor for the target type's natural Serde category.
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

    /// Converts a string value to UTF-8 bytes for a byte-oriented target.
    ///
    /// # Type Parameters
    ///
    /// * `V` - Visitor that constructs the target byte sequence.
    ///
    /// # Parameters
    ///
    /// * `visitor` - Visitor receiving the owned UTF-8 byte buffer.
    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        // Serde bytes follow String conversion and UTF-8 encoding; numeric
        // sequences are handled separately by deserialize_seq.
        visitor.visit_byte_buf(self.convert::<String>()?.into_bytes())
    }
    /// Deserializes an owned byte buffer using the string conversion policy.
    ///
    /// # Type Parameters
    ///
    /// * `V` - Visitor that constructs the target byte buffer.
    ///
    /// # Parameters
    ///
    /// * `visitor` - Visitor receiving the converted byte buffer.
    #[inline]
    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_bytes(visitor)
    }
    /// Maps missing and null source values to `None`, otherwise deserializes
    /// `Some`.
    ///
    /// # Type Parameters
    ///
    /// * `V` - Visitor that constructs the optional target value.
    ///
    /// # Parameters
    ///
    /// * `visitor` - Visitor receiving the absent or present value.
    #[inline]
    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        if self.is_missing() {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }
    /// Accepts only a missing source value as unit.
    ///
    /// # Type Parameters
    ///
    /// * `V` - Visitor that constructs the unit target.
    ///
    /// # Parameters
    ///
    /// * `visitor` - Visitor receiving unit when the source is missing.
    #[inline]
    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        if self.is_missing() {
            visitor.visit_unit()
        } else {
            Err(type_error(&self.path, "unit"))
        }
    }
    /// Delegates a unit-struct payload to unit deserialization.
    ///
    /// # Type Parameters
    ///
    /// * `V` - Visitor that constructs the unit-struct target.
    ///
    /// # Parameters
    ///
    /// * `visitor` - Visitor receiving the unit payload.
    #[inline]
    fn deserialize_unit_struct<V: Visitor<'de>>(self, _: &'static str, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_unit(visitor)
    }
    /// Passes the source value through to a newtype visitor.
    ///
    /// # Type Parameters
    ///
    /// * `V` - Visitor that constructs the newtype target.
    ///
    /// # Parameters
    ///
    /// * `visitor` - Visitor that consumes the wrapped source value.
    #[inline]
    fn deserialize_newtype_struct<V: Visitor<'de>>(self, _: &'static str, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_newtype_struct(self)
    }
    /// Deserializes collection or permitted scalar input as a sequence.
    ///
    /// # Type Parameters
    ///
    /// * `V` - Visitor that constructs the sequence target.
    ///
    /// # Parameters
    ///
    /// * `visitor` - Visitor receiving the lazily accessed elements.
    #[inline]
    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.sequence(visitor, None)
    }
    /// Deserializes a sequence with an exact tuple element count.
    ///
    /// # Type Parameters
    ///
    /// * `V` - Visitor that constructs the tuple target.
    ///
    /// # Parameters
    ///
    /// * `len` - Required number of tuple elements.
    /// * `visitor` - Visitor receiving the tuple elements.
    #[inline]
    fn deserialize_tuple<V: Visitor<'de>>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error> {
        self.sequence(visitor, Some(len))
    }
    /// Delegates a tuple-struct payload to tuple deserialization.
    ///
    /// # Type Parameters
    ///
    /// * `V` - Visitor that constructs the tuple-struct target.
    ///
    /// # Parameters
    ///
    /// * `len` - Required number of tuple fields.
    /// * `visitor` - Visitor receiving the tuple fields.
    #[inline]
    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        _: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_tuple(len, visitor)
    }
    /// Deserializes borrowed map entries through the shared conversion session.
    ///
    /// # Type Parameters
    ///
    /// * `V` - Visitor that constructs the map target.
    ///
    /// # Parameters
    ///
    /// * `visitor` - Visitor receiving the lazily accessed map entries.
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
    /// Delegates a struct payload to map deserialization.
    ///
    /// # Type Parameters
    ///
    /// * `V` - Visitor that constructs the struct target.
    ///
    /// # Parameters
    ///
    /// * `visitor` - Visitor receiving the struct's named fields.
    #[inline]
    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _: &'static str,
        _: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_map(visitor)
    }
    /// Deserializes an external enum tag and its borrowed payload.
    ///
    /// # Type Parameters
    ///
    /// * `V` - Visitor that constructs the target enum value.
    ///
    /// # Parameters
    ///
    /// * `visitor` - Visitor receiving the variant name and payload.
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
    /// Deserializes an identifier using the same string conversion as a value.
    ///
    /// # Type Parameters
    ///
    /// * `V` - Visitor that constructs the identifier target.
    ///
    /// # Parameters
    ///
    /// * `visitor` - Visitor receiving the converted identifier.
    #[inline]
    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_str(visitor)
    }
    /// Skips the value and records its diagnostic path when requested.
    ///
    /// # Type Parameters
    ///
    /// * `V` - Visitor that accepts the ignored value.
    ///
    /// # Parameters
    ///
    /// * `visitor` - Visitor receiving the unit marker for the skipped value.
    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        if let Some(paths) = self.context.ignored_paths {
            paths.borrow_mut().push(self.path.clone());
        }
        visitor.visit_unit()
    }
}

#[cfg(test)]
mod tests {
    use qubit_datatype::ConversionSession;
    use qubit_value::ValueMissing;
    use serde::Deserialize;

    use super::super::prepared_read::prepare_read;
    use super::ReadContext;
    use super::ReadView;
    use super::SourceDeserializer;
    use super::json_number_converter;
    use super::scalar_converter;
    use crate::Config;
    use crate::ReadPolicy;

    #[test]
    fn test_scalar_conversion_accepts_borrowed_text_and_json_primitives() {
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

        let missing = ValueMissing::unset_scalar(qubit_datatype::DataType::String, qubit_datatype::DataType::String);
        assert!(
            scalar_converter(ReadView::Missing(&missing), context, None)
                .unwrap()
                .is_none()
        );

        let absent_property_location = super::super::node::SourceLocation {
            property_index: 0,
            segments: Vec::new(),
        };
        assert!(
            scalar_converter(ReadView::Missing(&missing), context, Some(&absent_property_location),)
                .unwrap()
                .is_none()
        );

        let mut collection_config = Config::new();
        collection_config.set("items", vec!["one"]).unwrap();
        let collection_prepared = prepare_read(&collection_config, "", false).unwrap();
        let collection_context = ReadContext {
            prepared: &collection_prepared,
            ignored_paths: None,
        };
        let collection_location = super::super::node::SourceLocation {
            property_index: 0,
            segments: Vec::new(),
        };
        assert!(
            scalar_converter(
                ReadView::Missing(&missing),
                collection_context,
                Some(&collection_location),
            )
            .unwrap()
            .is_none()
        );
    }

    #[test]
    fn test_json_number_conversion_preserves_signed_unsigned_and_fractional_values() {
        let signed = serde_json::Number::from(-17_i64);
        let unsigned = serde_json::Number::from(u64::MAX);
        let fractional = serde_json::Number::from_f64(1.25).unwrap();
        assert!(json_number_converter(&signed).is_ok());
        assert!(json_number_converter(&unsigned).is_ok());
        assert!(json_number_converter(&fractional).is_ok());
        assert!(json_number_converter(&serde_json::Number::from_i128(i128::MIN).unwrap()).is_ok());
        assert!(json_number_converter(&serde_json::Number::from_u128(u128::MAX).unwrap()).is_ok());
        let outside_u128: serde_json::Number = serde_json::from_str("340282366920938463463374607431768211456").unwrap();
        assert_eq!(
            json_number_converter(&outside_u128),
            Err(qubit_datatype::InvalidValueReason::OutOfRange)
        );
        let outside_i128: serde_json::Number =
            serde_json::from_str("-170141183460469231731687303715884105729").unwrap();
        assert_eq!(
            json_number_converter(&outside_i128),
            Err(qubit_datatype::InvalidValueReason::OutOfRange)
        );
    }

    #[test]
    fn test_source_deserializer_reports_unrepresentable_json_numbers_safely() {
        let config = Config::new();
        let prepared = prepare_read(&config, "", false).unwrap();
        let context = ReadContext {
            prepared: &prepared,
            ignored_paths: None,
        };
        let policy = ReadPolicy::default();
        let mut session = ConversionSession::new(policy.conversion_policy(), policy.conversion_limits());
        let number: serde_json::Value = serde_json::from_str("340282366920938463463374607431768211456").unwrap();

        let error = SourceDeserializer::new(
            context,
            ReadView::Json(&number),
            None,
            String::from("number"),
            false,
            &mut session,
        )
        .convert::<u64>()
        .unwrap_err();
        assert!(error.to_string().contains("number"));

        let deserializer = SourceDeserializer::new(
            context,
            ReadView::Json(&number),
            None,
            String::from("number"),
            false,
            &mut session,
        );
        let error = serde_json::Value::deserialize(deserializer).unwrap_err();
        assert!(error.to_string().contains("number"));

        let wide = serde_json::Value::Number(serde_json::Number::from_u128(u128::MAX).unwrap());
        let deserializer = SourceDeserializer::new(
            context,
            ReadView::Json(&wide),
            None,
            String::from("wide"),
            false,
            &mut session,
        );
        assert_eq!(
            serde_json::Value::deserialize(deserializer).unwrap(),
            serde_json::Value::String(u128::MAX.to_string())
        );
    }
}
