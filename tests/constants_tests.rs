// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// [`qubit_config::constants`] behavior via read policies.

use qubit_config::options::ReadPolicy;

#[cfg(test)]
mod test_max_interpolation_depth {
    use super::ReadPolicy;

    #[test]
    fn test_max_interpolation_depth_returns_default_value() {
        let options = ReadPolicy::default();
        assert_eq!(options.max_interpolation_depth(), 64);
    }

    #[test]
    fn test_set_max_interpolation_depth_sets_value() {
        let options = ReadPolicy::builder().max_interpolation_depth(100).build();
        assert_eq!(options.max_interpolation_depth(), 100);
    }

    #[test]
    fn test_set_max_interpolation_depth_sets_zero() {
        let options = ReadPolicy::builder().max_interpolation_depth(0).build();
        assert_eq!(options.max_interpolation_depth(), 0);
    }
}
