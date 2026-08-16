// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_config::ConfigResult;
use qubit_config::source::CompositeConfigSource;
use qubit_config::source::ConfigSource;
use qubit_config::source::SourceLimits;
use qubit_config::source::SourceLoadContext;

struct ChildAccountingSource {
    amount: usize,
}

impl ConfigSource for ChildAccountingSource {
    fn source_id(&self) -> String {
        "child".to_string()
    }

    fn limits(&self) -> SourceLimits {
        SourceLimits::builder().max_input_bytes(5).build()
    }

    fn load_into(
        &self,
        context: &mut SourceLoadContext<'_>,
    ) -> ConfigResult<()> {
        context.consume_input_bytes(self.amount)
    }
}

#[test]
fn source_load_session_charges_local_and_ancestor_budgets_atomically() {
    let mut aggregate = CompositeConfigSource::builder()
        .limits(SourceLimits::builder().max_input_bytes(3).build())
        .build();
    aggregate.add(ChildAccountingSource { amount: 2 });
    aggregate.add(ChildAccountingSource { amount: 2 });
    let error = aggregate
        .load()
        .expect_err("the aggregate budget should reject the second charge");

    assert_eq!(
        error.source_budget_id(),
        Some("composite configuration source")
    );
}
