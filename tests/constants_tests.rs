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

#[test]
fn read_policy_builder_round_trips_runtime_options() {
    use qubit_config::options::InterpolationSources;
    use qubit_datatype::ConversionLimits;
    use qubit_datatype::ConversionPolicy;

    let policy = ReadPolicy::builder()
        .conversion_policy(ConversionPolicy::env_friendly())
        .interpolation_sources(InterpolationSources::ConfigThenEnv)
        .max_interpolation_depth(3)
        .max_interpolation_expansions(5)
        .max_interpolation_output_bytes(7)
        .build();
    assert_eq!(policy.interpolation_sources(), InterpolationSources::ConfigThenEnv);
    assert_eq!(policy.max_interpolation_depth(), 3);
    assert_eq!(policy.max_interpolation_expansions(), 5);
    assert_eq!(policy.max_interpolation_output_bytes(), 7);
    assert_eq!(ReadPolicy::builder_from(&policy).build(), policy);
    assert_eq!(ReadPolicy::config_only(), ReadPolicy::default());
    assert_eq!(
        ReadPolicy::env_friendly().conversion_policy(),
        &ConversionPolicy::env_friendly()
    );
    assert_eq!(
        ReadPolicy::from(ConversionPolicy::default()).conversion_policy(),
        &ConversionPolicy::default()
    );
    let _: &ConversionPolicy = <ReadPolicy as AsRef<ConversionPolicy>>::as_ref(&policy);
    let _: &ConversionLimits = <ReadPolicy as AsRef<ConversionLimits>>::as_ref(&policy);
}
