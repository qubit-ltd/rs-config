// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Borrowed conversion input or an already admitted scalar item.

use qubit_datatype::AdmittedScalarItem;
use qubit_datatype::ConversionSession;

use super::traversal::ReadView;

pub(super) enum ConversionInput<'tree, 'policy, 'session> {
    Borrowed {
        value: ReadView<'tree>,
        session: &'session mut ConversionSession<'policy>,
    },
    Admitted(AdmittedScalarItem<'session, 'policy, 'tree>),
}
