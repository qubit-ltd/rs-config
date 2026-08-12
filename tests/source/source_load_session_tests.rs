// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================

use qubit_config::source::SourceLimits;
use qubit_config::source::SourceLoadSession;

#[test]
fn source_load_session_charges_local_and_ancestor_budgets_atomically() {
    let mut aggregate =
        SourceLoadSession::new("composite", SourceLimits::default().with_max_input_bytes(3));
    let mut child = aggregate.child(
        "properties:<memory>",
        SourceLimits::default().with_max_input_bytes(5),
    );

    child
        .consume_input_bytes(2)
        .expect("two bytes should fit both budgets");
    let error = child
        .consume_input_bytes(2)
        .expect_err("the aggregate budget should reject the second charge");

    assert_eq!(error.source_id(), Some("properties:<memory>"));
    assert_eq!(error.source_budget_id(), Some("composite"));
    child
        .consume_input_bytes(1)
        .expect("a rejected grouped charge must leave all budgets unchanged");
}
