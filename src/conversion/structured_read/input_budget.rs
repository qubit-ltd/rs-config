// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Complete logical-input admission, independent of target conversion usage.

use std::fmt;
use std::fmt::Display;
use std::fmt::Write;

use qubit_budget::MeasuredBudgetError;
use qubit_budget::ResourceBudget;
use qubit_budget::ResourceQuantity;
use qubit_budget::json::JsonMeasurement;
use qubit_budget::json::JsonValueBudget;
use qubit_datatype::ConversionLimits;
use qubit_datatype::ConversionResource;
use qubit_datatype::DataConversionError;
use qubit_datatype::DataConverter;
use qubit_datatype::DataListConversionError;
use qubit_datatype::DataType;
use qubit_datatype::InvalidValueReason;
use qubit_value::ValueError;
use qubit_value::ValueRef;

use super::prepared_read::PreparedConfigRead;
use super::traversal;
use super::traversal::ReadView;
use super::traversal::Visit;
use crate::ConfigError;
use crate::ConfigResult;
use crate::ReadPolicy;

struct PreparedInputBudget<'a> {
    options: &'a ReadPolicy,
    structure: JsonValueBudget<ConversionResource, u64>,
    input: ResourceBudget<ConversionResource, u64>,
}

impl<'a> PreparedInputBudget<'a> {
    fn new(options: &'a ReadPolicy) -> Self {
        let limits = options.conversion_limits();
        let value_limits = (*limits.structured().value())
            .into_builder()
            .structure_limits(
                limits
                    .structured()
                    .value()
                    .structure_limits()
                    .to_builder()
                    .nodes_limit(*limits.operation().structured_nodes_limit())
                    .build(),
            )
            .payload_bytes_limit(*limits.operation().structured_payload_bytes_limit())
            .build();
        Self {
            options,
            structure: JsonValueBudget::new(value_limits),
            input: ResourceBudget::from_limit(*limits.operation().input_bytes_limit()),
        }
    }

    fn admit(&mut self, measurement: JsonMeasurement, visit: &Visit<'_>) -> ConfigResult<()> {
        let mut transaction = self.structure.transaction();
        transaction
            .try_admit(measurement)
            .and_then(|()| transaction.commit())
            .map_err(|error| budget_error(visit, error))
    }

    fn input(&mut self, text: &str, visit: &Visit<'_>) -> ConfigResult<()> {
        self.input
            .try_consume_usize(text.len())
            .map_err(|error| budget_error(visit, error))
    }

    fn text(&mut self, text: &str, visit: &Visit<'_>) -> ConfigResult<()> {
        self.input(text, visit)?;
        self.admit(
            JsonMeasurement::String {
                depth: visit.depth,
                bytes: text.len(),
            },
            visit,
        )
    }

    /// Counts formatted text without producing a complete intermediate copy.
    fn display(&mut self, value: &impl Display, number: bool, visit: &Visit<'_>) -> ConfigResult<()> {
        let probe = if number {
            JsonMeasurement::Number {
                depth: visit.depth,
                bytes: 0,
            }
        } else {
            JsonMeasurement::String {
                depth: visit.depth,
                bytes: 0,
            }
        };
        self.structure
            .transaction()
            .try_admit(probe)
            .map_err(|error| budget_error(visit, error))?;
        let limits = self.options.conversion_limits();
        let mut bound = ResourceBudget::from_limit(*limits.operation().structured_payload_bytes_limit());
        if let Err(error) = bound.try_consume(self.structure.used_payload_bytes().unwrap_or(0)) {
            return Err(budget_error(visit, error.into()));
        }
        let text = ResourceBudget::from_limit(*limits.structured().max_text_bytes_limit());
        if text.remaining() < bound.remaining() {
            bound = text;
        }
        let initial = bound.used();
        let mut writer = CountingWriter {
            budget: bound,
            error: None,
        };
        let formatted = write!(&mut writer, "{value}");
        if let Some(error) = writer.error {
            return Err(budget_error(visit, error));
        }
        formatted.map_err(|_| conversion_error(visit, InvalidValueReason::OutOfRange))?;
        let bytes = match usize::try_from_u64(writer.budget.used() - initial) {
            Ok(bytes) => bytes,
            Err(error) => {
                return Err(budget_error(
                    visit,
                    MeasuredBudgetError::quantity(ConversionResource::StructuredPayloadBytes, error),
                ));
            }
        };
        self.admit(
            if number {
                JsonMeasurement::Number {
                    depth: visit.depth,
                    bytes,
                }
            } else {
                JsonMeasurement::String {
                    depth: visit.depth,
                    bytes,
                }
            },
            visit,
        )
    }

    fn scalar(&mut self, value: ValueRef<'_>, visit: &Visit<'_>) -> ConfigResult<()> {
        match value {
            ValueRef::Unset(_) => self.admit(JsonMeasurement::Null { depth: visit.depth }, visit),
            ValueRef::Bool(_) => self.admit(JsonMeasurement::Boolean { depth: visit.depth }, visit),
            ValueRef::String(value) => self.text(value, visit),
            ValueRef::Char(value) => self.display(&value, false, visit),
            ValueRef::Int8(value) => self.display(&value, true, visit),
            ValueRef::Int16(value) => self.display(&value, true, visit),
            ValueRef::Int32(value) => self.display(&value, true, visit),
            ValueRef::Int64(value) => self.display(&value, true, visit),
            ValueRef::UInt8(value) => self.display(&value, true, visit),
            ValueRef::UInt16(value) => self.display(&value, true, visit),
            ValueRef::UInt32(value) => self.display(&value, true, visit),
            ValueRef::UInt64(value) => self.display(&value, true, visit),
            ValueRef::Int128(value) => self.display(&value, false, visit),
            ValueRef::UInt128(value) => self.display(&value, false, visit),
            ValueRef::Float32(value) => {
                let number: serde_json::Number = value
                    .to_string()
                    .parse()
                    .map_err(|_| conversion_error(visit, InvalidValueReason::NonFinite))?;
                self.display(&number, true, visit)
            }
            ValueRef::Float64(value) => {
                let number = serde_json::Number::from_f64(value)
                    .ok_or_else(|| conversion_error(visit, InvalidValueReason::NonFinite))?;
                self.display(&number, true, visit)
            }
            #[cfg(feature = "num-bigint")]
            ValueRef::BigInteger(value) => {
                self.options
                    .conversion_limits()
                    .numeric()
                    .big_integer()
                    .check(value)
                    .map_err(|error| budget_error(visit, error))?;
                self.display(&value, false, visit)
            }
            #[cfg(feature = "bigdecimal")]
            ValueRef::BigDecimal(value) => {
                self.options
                    .conversion_limits()
                    .numeric()
                    .big_decimal()
                    .check(value)
                    .map_err(|error| budget_error(visit, error))?;
                self.display(&value, false, visit)
            }
            #[cfg(feature = "chrono")]
            ValueRef::Date(value) => self.display(&value, false, visit),
            #[cfg(feature = "chrono")]
            ValueRef::Time(value) => self.display(&value, false, visit),
            #[cfg(feature = "chrono")]
            ValueRef::DateTime(value) => self.display(&value, false, visit),
            #[cfg(feature = "chrono")]
            ValueRef::Instant(value) => self.display(&value, false, visit),
            #[cfg(feature = "url")]
            ValueRef::Url(value) => self.display(&value, false, visit),
            ValueRef::Duration(value) => {
                let limits = formatting_limits(self.options.conversion_limits());
                let text = DataConverter::from(value)
                    .to_with::<String>(self.options.conversion_policy(), &limits)
                    .map_err(|error| source_error(visit, error))?;
                self.admit(
                    JsonMeasurement::String {
                        depth: visit.depth,
                        bytes: text.len(),
                    },
                    visit,
                )
            }
            _ => Err(source_error(
                visit,
                DataConversionError::unsupported(DataConverter::from(value).data_type(), DataType::Json),
            )),
        }
    }
}

struct CountingWriter {
    budget: ResourceBudget<ConversionResource, u64>,
    error: Option<MeasuredBudgetError<ConversionResource, u64>>,
}

impl fmt::Write for CountingWriter {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        self.budget.try_consume_usize(text.len()).map_err(|error| {
            self.error = Some(error);
            fmt::Error
        })
    }
}

fn data_type(visit: &Visit<'_>) -> DataType {
    match visit.value {
        ReadView::Scalar(value) => DataConverter::from(value).data_type(),
        ReadView::Collection(value) => value.data_type(),
        ReadView::StringMap(_) => DataType::StringMap,
        ReadView::Text(_) => DataType::String,
        ReadView::Missing(value) => value.source_type().unwrap_or(DataType::Json),
        _ => DataType::Json,
    }
}

fn budget_error(visit: &Visit<'_>, source: MeasuredBudgetError<ConversionResource, u64>) -> ConfigError {
    ConfigError::from((
        visit.path.as_ref(),
        ValueError::JsonProjectionLimit {
            data_type: data_type(visit),
            source_index: visit.source_index(),
            source,
        },
    ))
}

fn conversion_error(visit: &Visit<'_>, reason: InvalidValueReason) -> ConfigError {
    source_error(
        visit,
        DataConversionError::invalid(data_type(visit), DataType::Json, reason),
    )
}

fn source_error(visit: &Visit<'_>, source: DataConversionError) -> ConfigError {
    match visit.source_index() {
        Some(source_index) => ConfigError::from((
            visit.path.as_ref(),
            ValueError::from(DataListConversionError::new(source_index, source)),
        )),
        None => ConfigError::from_data_conversion_error(&visit.path, source),
    }
}

/// Rich natural-input rendering consumes the input allowance, not the final
/// target's output allowance. Both remain bounded by their real resource
/// domain.
fn formatting_limits(limits: &ConversionLimits) -> ConversionLimits {
    limits
        .clone()
        .into_builder()
        .operation_limits(
            (*limits.operation())
                .into_builder()
                .max_output_bytes(limits.operation().max_structured_payload_bytes())
                .build(),
        )
        .build()
}

pub(super) fn admit_input(prepared: &PreparedConfigRead<'_>, options: &ReadPolicy) -> ConfigResult<()> {
    let mut budget = PreparedInputBudget::new(options);
    traversal::walk(prepared, !prepared.overlays.is_empty(), |visit| {
        if let Some(key) = visit.key {
            budget.input(key, visit)?;
            budget.admit(JsonMeasurement::Key { bytes: key.len() }, visit)?;
        }
        if let Some(text) = visit
            .location
            .as_ref()
            .and_then(|location| prepared.overlays.get(location))
        {
            return budget.text(text, visit);
        }
        match visit.value {
            ReadView::Scalar(value) => budget.scalar(value, visit),
            ReadView::Text(value) => budget.text(value, visit),
            ReadView::Missing(_) | ReadView::Json(serde_json::Value::Null) => {
                budget.admit(JsonMeasurement::Null { depth: visit.depth }, visit)
            }
            ReadView::Json(serde_json::Value::Bool(_)) => {
                budget.admit(JsonMeasurement::Boolean { depth: visit.depth }, visit)
            }
            ReadView::Json(serde_json::Value::String(value)) => budget.text(value, visit),
            ReadView::Json(serde_json::Value::Number(value)) => budget.display(value, true, visit),
            ReadView::Collection(values) => budget.admit(
                JsonMeasurement::Array {
                    depth: visit.depth,
                    items: values.len(),
                },
                visit,
            ),
            ReadView::Json(serde_json::Value::Array(values)) => budget.admit(
                JsonMeasurement::Array {
                    depth: visit.depth,
                    items: values.len(),
                },
                visit,
            ),
            ReadView::Json(serde_json::Value::Object(values)) => budget.admit(
                JsonMeasurement::Object {
                    depth: visit.depth,
                    entries: values.len(),
                },
                visit,
            ),
            ReadView::Object(values) => budget.admit(
                JsonMeasurement::Object {
                    depth: visit.depth,
                    entries: values.len(),
                },
                visit,
            ),
            ReadView::StringMap(values) => budget.admit(
                JsonMeasurement::Object {
                    depth: visit.depth,
                    entries: values.len(),
                },
                visit,
            ),
        }
    })
}
