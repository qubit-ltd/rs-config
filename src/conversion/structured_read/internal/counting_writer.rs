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

/// Formatter sink that charges each emitted UTF-8 chunk to a resource budget.
/// It records the first budget error and stops formatting when the charge
/// fails.
pub(crate) struct CountingWriter {
    /// Remaining and consumed allowance for formatted bytes.
    pub(crate) budget: ResourceBudget<ConversionResource, u64>,
    /// First budget failure encountered while formatting, if any.
    pub(crate) error: Option<MeasuredBudgetError<ConversionResource, u64>>,
}

impl fmt::Write for CountingWriter {
    /// Charges the UTF-8 bytes before accepting the formatted chunk.
    ///
    /// # Parameters
    ///
    /// * `text` - Formatted text whose UTF-8 byte length is charged.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after the text's byte length is charged to the budget.
    ///
    /// # Errors
    ///
    /// Returns `fmt::Error` when the budget cannot accept the text. The
    /// underlying budget error is retained in `self.error` for the caller.
    fn write_str(&mut self, text: &str) -> fmt::Result {
        self.budget.try_consume_usize(text.len()).map_err(|error| {
            self.error = Some(error);
            fmt::Error
        })
    }
}
