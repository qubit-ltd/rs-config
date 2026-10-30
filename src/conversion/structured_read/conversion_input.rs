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
    /// A prepared value borrowed from the tree and its live conversion session.
    ///
    /// Converting this view charges the target conversion session as the
    /// requested target is produced.
    Borrowed {
        /// Prepared value being converted without copying its payload.
        value: ReadView<'tree>,
        /// Session that accounts resources used by target conversion.
        session: &'session mut ConversionSession<'policy>,
    },
    /// A scalar item whose admission was already charged by a collection
    /// conversion session.
    ///
    /// Its session and source index travel with the item; it must not be
    /// admitted again as fresh collection text or split into more items.
    Admitted(AdmittedScalarItem<'session, 'policy, 'tree>),
}
