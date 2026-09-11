// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Borrowed conversion input or an already admitted scalar item.

use qubit_datatype::AdmittedScalarItem;
use qubit_datatype::ConversionSession;

use super::traversal::ReadView;

pub(super) enum ConversionInput<'tree, 'policy, 'session> {
    /// A borrowed prepared view and its live conversion session.
    Borrowed {
        /// Prepared value being converted.
        value: ReadView<'tree>,
        /// Session that accounts conversion resources.
        session: &'session mut ConversionSession<'policy>,
    },
    /// A scalar item whose admission was already charged.
    Admitted(AdmittedScalarItem<'session, 'policy, 'tree>),
}
