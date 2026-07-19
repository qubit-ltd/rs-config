// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! [`qubit_config::constants`] behavior via read options.

use qubit_config::options::ReadOptions;

#[cfg(test)]
mod test_max_interpolation_depth {
    #[allow(unused_imports)]
    use super::ReadOptions;

    #[test]
    fn test_max_interpolation_depth_returns_default_value() {
        let options = ReadOptions::default();
        assert_eq!(options.max_interpolation_depth(), 64);
    }

    #[test]
    fn test_set_max_interpolation_depth_sets_value() {
        let options = ReadOptions::default().with_max_interpolation_depth(100);
        assert_eq!(options.max_interpolation_depth(), 100);
    }

    #[test]
    fn test_set_max_interpolation_depth_sets_zero() {
        let options = ReadOptions::default().with_max_interpolation_depth(0);
        assert_eq!(options.max_interpolation_depth(), 0);
    }
}
