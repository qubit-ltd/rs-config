// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Budget-counting formatter used by structured input admission.

use std::fmt;

use qubit_budget::MeasuredBudgetError;
use qubit_budget::ResourceBudget;
use qubit_datatype::ConversionResource;

pub(crate) struct CountingWriter {
    pub(crate) budget: ResourceBudget<ConversionResource, u64>,
    pub(crate) error: Option<MeasuredBudgetError<ConversionResource, u64>>,
}

impl fmt::Write for CountingWriter {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        self.budget.try_consume_usize(text.len()).map_err(|error| {
            self.error = Some(error);
            fmt::Error
        })
    }
}
