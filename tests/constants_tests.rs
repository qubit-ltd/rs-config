// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! [`qubit_config::constants`] behavior via substitution options.

use qubit_config::options::VariableSubstitutionOptions;

#[cfg(test)]
mod test_max_substitution_depth {
    #[allow(unused_imports)]
    use super::VariableSubstitutionOptions;

    #[test]
    fn test_max_substitution_depth_returns_default_value() {
        let options = VariableSubstitutionOptions::default();
        assert_eq!(options.max_depth(), 64);
    }

    #[test]
    fn test_set_max_substitution_depth_sets_value() {
        let options =
            VariableSubstitutionOptions::default().with_max_depth(100);
        assert_eq!(options.max_depth(), 100);
    }

    #[test]
    fn test_set_max_substitution_depth_sets_zero() {
        let options = VariableSubstitutionOptions::default().with_max_depth(0);
        assert_eq!(options.max_depth(), 0);
    }
}
