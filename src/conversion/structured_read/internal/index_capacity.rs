// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Resource bounds used while constructing the prepared path index.

use qubit_budget::MeasuredBudgetError;
use qubit_budget::ResourceBudget;
use qubit_budget::json::JsonMeasurement;
use qubit_budget::json::JsonValueBudget;
use qubit_budget::json::JsonValueLimits;
use qubit_datatype::ConversionLimits;
use qubit_datatype::ConversionResource;
use qubit_datatype::DataType;
use qubit_value::ValueError;

use crate::ConfigError;
use crate::ConfigResult;

pub(crate) struct IndexCapacity {
    budget: JsonValueBudget<ConversionResource, u64>,
    input_keys: ResourceBudget<ConversionResource, u64>,
    payload_keys: ResourceBudget<ConversionResource, u64>,
    limits: ConversionLimits,
}

impl IndexCapacity {
    pub(crate) fn new(limits: &ConversionLimits) -> Self {
        Self {
            budget: JsonValueBudget::new(
                JsonValueLimits::builder()
                    .structure_limits(
                        limits
                            .structured()
                            .value()
                            .structure_limits()
                            .to_builder()
                            .nodes_limit(*limits.operation().structured_nodes_limit())
                            .build(),
                    )
                    .build(),
            ),
            input_keys: ResourceBudget::from_limit(*limits.operation().input_bytes_limit()),
            payload_keys: ResourceBudget::from_limit(*limits.operation().structured_payload_bytes_limit()),
            limits: limits.clone(),
        }
    }

    pub(crate) fn admit(&mut self, measurement: JsonMeasurement, path: &str) -> ConfigResult<()> {
        let mut transaction = self.budget.transaction();
        transaction
            .try_admit(measurement)
            .and_then(|()| transaction.commit())
            .map_err(|source| {
                ConfigError::from((
                    path,
                    ValueError::JsonProjectionLimit {
                        data_type: DataType::Json,
                        source_index: None,
                        source,
                    },
                ))
            })
    }

    pub(crate) fn key(&mut self, bytes: usize, path: &str) -> ConfigResult<()> {
        self.input_keys
            .try_consume_usize(bytes)
            .map_err(|error| index_error(path, error))?;
        self.payload_keys
            .try_consume_usize(bytes)
            .map_err(|error| index_error(path, error))?;
        self.admit(JsonMeasurement::Key { bytes }, path)
    }

    pub(crate) fn entries(&self, count: usize, path: &str) -> ConfigResult<()> {
        // Rust supports pointer widths up to 64 bits, so every `usize` fits in `u64`.
        let count = count as u64;
        self.limits
            .structured()
            .max_map_entries_limit()
            .check(count)
            .map_err(|error| index_error(path, error.into()))
    }
}

fn index_error(path: &str, source: MeasuredBudgetError<ConversionResource, u64>) -> ConfigError {
    ConfigError::from((
        path,
        ValueError::JsonProjectionLimit {
            data_type: DataType::Json,
            source_index: None,
            source,
        },
    ))
}
