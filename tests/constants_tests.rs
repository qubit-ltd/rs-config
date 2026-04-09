/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! [`qubit_config::constants`] behavior via [`qubit_config::Config`] (`DEFAULT_MAX_SUBSTITUTION_DEPTH`).

use qubit_config::Config;

#[cfg(test)]
mod test_max_substitution_depth {
    use super::*;

    #[test]
    fn test_max_substitution_depth_returns_default_value() {
        let config = Config::new();
        assert_eq!(config.max_substitution_depth(), 64);
    }

    #[test]
    fn test_set_max_substitution_depth_sets_value() {
        let mut config = Config::new();
        config.set_max_substitution_depth(100);
        assert_eq!(config.max_substitution_depth(), 100);
    }

    #[test]
    fn test_set_max_substitution_depth_sets_zero() {
        let mut config = Config::new();
        config.set_max_substitution_depth(0);
        assert_eq!(config.max_substitution_depth(), 0);
    }
}
