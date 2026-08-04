// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// # [`qubit_config::Config`] unit tests
//
// Covers the public `Config` API (including APIs introduced in v0.4.0).

#![allow(dead_code, unused_imports)]

#[cfg(feature = "chrono")]
pub(crate) use chrono::{
    DateTime,
    NaiveDate,
    NaiveDateTime,
    NaiveTime,
    Utc,
};
pub(crate) use qubit_config::{
    Config,
    ConfigError,
    ConfigReader,
    Property,
    options::{
        InterpolationSources,
        ReadPolicy,
    },
};
pub(crate) use qubit_datatype::{
    BlankStringPolicy,
    DataConversionErrorKind,
    DataType,
};
pub(crate) use qubit_value::MultiValues;
pub(crate) use serde::Deserialize;

/// Creates a test configuration object
pub(crate) fn create_test_config() -> Config {
    let mut config = Config::new();
    config.set("string_value", "test").unwrap();
    config.set("int_value", 42).unwrap();
    config.set("bool_value", true).unwrap();
    config.set("float_value", 3.5).unwrap();
    config
}

/// Creates a test configuration with description
#[allow(dead_code)]
pub(crate) fn create_test_config_with_description() -> Config {
    Config::with_description("Test Configuration")
}

/// Changes the interpolation recursion limit while preserving other options.
pub(crate) fn set_max_interpolation_depth(
    config: &mut Config,
    max_depth: usize,
) {
    let options = config
        .default_read_policy()
        .clone()
        .with_max_interpolation_depth(max_depth);
    config.set_default_read_policy(options);
}
// IntoConfigDefault Tests
// ============================================================================

#[cfg(test)]
mod test_into_config_default {
    use qubit_config::conversion::IntoConfigDefault;

    #[test]
    fn test_identity_default_conversion() {
        let value: i64 = 42_i64.into_config_default();

        assert_eq!(value, 42);
    }

    #[test]
    fn test_string_default_conversions() {
        let borrowed: String = "default".into_config_default();
        let owned = "owned".to_string();
        let cloned: String = (&owned).into_config_default();

        assert_eq!(borrowed, "default");
        assert_eq!(cloned, "owned");
    }

    #[test]
    fn test_vec_default_conversions() {
        let slice_source = [1, 2, 3];
        let vec_source = vec![4, 5, 6];
        let array_ref_source = [7, 8, 9];

        let from_slice: Vec<i32> =
            slice_source.as_slice().into_config_default();
        let from_vec_ref: Vec<i32> = (&vec_source).into_config_default();
        let from_array: Vec<i32> = [10, 11, 12].into_config_default();
        let from_array_ref: Vec<i32> =
            (&array_ref_source).into_config_default();

        assert_eq!(from_slice, vec![1, 2, 3]);
        assert_eq!(from_vec_ref, vec![4, 5, 6]);
        assert_eq!(from_array, vec![10, 11, 12]);
        assert_eq!(from_array_ref, vec![7, 8, 9]);
    }

    #[test]
    fn test_string_vec_default_conversions() {
        let slice_source: &[&str] = &["a", "b"];
        let vec_ref_source = vec!["c", "d"];
        let array_ref_source = ["g", "h"];

        let from_slice: Vec<String> = slice_source.into_config_default();
        let from_vec_ref: Vec<String> = (&vec_ref_source).into_config_default();
        let from_vec: Vec<String> = vec!["e", "f"].into_config_default();
        let from_array: Vec<String> = ["i", "j"].into_config_default();
        let from_array_ref: Vec<String> =
            (&array_ref_source).into_config_default();

        assert_eq!(from_slice, vec!["a".to_string(), "b".to_string()]);
        assert_eq!(from_vec_ref, vec!["c".to_string(), "d".to_string()]);
        assert_eq!(from_vec, vec!["e".to_string(), "f".to_string()]);
        assert_eq!(from_array, vec!["i".to_string(), "j".to_string()]);
        assert_eq!(from_array_ref, vec!["g".to_string(), "h".to_string()]);
    }
}

// ============================================================================
// Core Generic Method Tests - get_list<T>
// ============================================================================

#[cfg(test)]
mod test_get_list {
    #[allow(unused_imports)]
    use super::{
        Config,
        ConfigError,
        DataType,
        Deserialize,
        MultiValues,
        Property,
        create_test_config,
        create_test_config_with_description,
    };

    #[test]
    fn test_get_list_string() {
        let mut config = Config::new();
        config
            .set(
                "test",
                vec![
                    "value1".to_string(),
                    "value2".to_string(),
                    "value3".to_string(),
                ],
            )
            .unwrap();
        let values: Vec<String> = config.get_list("test").unwrap();
        assert_eq!(values, vec!["value1", "value2", "value3"]);
    }

    #[test]
    fn test_get_list_integer() {
        let mut config = Config::new();
        config.set("test", vec![1, 2, 3, 4, 5]).unwrap();
        let values: Vec<i32> = config.get_list("test").unwrap();
        assert_eq!(values, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_get_list_bool() {
        let mut config = Config::new();
        config.set("test", vec![true, false, true]).unwrap();
        let values: Vec<bool> = config.get_list("test").unwrap();
        assert_eq!(values, vec![true, false, true]);
    }

    #[test]
    fn test_get_list_bool_from_string_values() {
        let mut config = Config::new();
        config.set("test", vec!["1", "0", "true", "FALSE"]).unwrap();

        let values: Vec<bool> = config.get_list("test").unwrap();
        assert_eq!(values, vec![true, false, true, false]);
    }

    #[test]
    fn test_get_list_strict_preserves_exact_type_checking() {
        let mut config = Config::new();
        config.set("test", vec!["1", "0"]).unwrap();

        let err = config.get_list_strict::<bool>("test").unwrap_err();
        assert!(matches!(err, ConfigError::TypeMismatch { .. }));
    }

    #[test]
    fn test_get_list_not_found() {
        let config = Config::new();
        let result: Result<Vec<String>, _> = config.get_list("nonexistent");
        assert!(result.is_err());
        assert!(matches!(result, Err(ConfigError::PropertyNotFound(_))));
    }

    #[test]
    fn test_get_list_type_mismatch() {
        let mut config = Config::new();
        config.set("test", "string").unwrap();
        let result: Result<Vec<i32>, _> = config.get_list("test");
        assert!(result.is_err());
        assert!(matches!(result, Err(ConfigError::ConversionError { .. })));
    }
}

// ============================================================================
// Core Generic Method Tests - set<T>
// ============================================================================

#[cfg(test)]
mod test_set {
    #[allow(unused_imports)]
    use super::{
        Config,
        ConfigError,
        DataType,
        Deserialize,
        MultiValues,
        Property,
        create_test_config,
        create_test_config_with_description,
    };
    #[cfg(feature = "chrono")]
    use super::{
        DateTime,
        NaiveDate,
        NaiveDateTime,
        NaiveTime,
        Utc,
    };

    #[test]
    fn test_set_string() {
        let mut config = Config::new();
        config.set("test", "value").unwrap();
        let value: String = config.get("test").unwrap();
        assert_eq!(value, "value");
    }

    #[test]
    fn test_set_integer() {
        let mut config = Config::new();
        config.set("test", 42).unwrap();
        let value: i32 = config.get("test").unwrap();
        assert_eq!(value, 42);
    }

    #[test]
    fn test_set_bool() {
        let mut config = Config::new();
        config.set("test", true).unwrap();
        let value: bool = config.get("test").unwrap();
        assert!(value);
    }

    #[test]
    fn test_set_float() {
        let mut config = Config::new();
        config.set("test", 3.5).unwrap();
        let value: f64 = config.get("test").unwrap();
        assert_eq!(value, 3.5);
    }

    #[test]
    fn test_set_vector() {
        let mut config = Config::new();
        config.set("test", vec![1, 2, 3]).unwrap();
        let value: Vec<i32> = config.get_list("test").unwrap();
        assert_eq!(value, vec![1, 2, 3]);
    }

    #[test]
    fn test_set_overwrites_existing() {
        let mut config = Config::new();
        config.set("test", "value1").unwrap();
        config.set("test", "value2").unwrap();
        let value: String = config.get("test").unwrap();
        assert_eq!(value, "value2");
    }

    // Test all supported data types
    #[test]
    fn test_set_all_integer_types() {
        let mut config = Config::new();

        config.set("i8", 42i8).unwrap();
        config.set("i16", 42i16).unwrap();
        config.set("i32", 42i32).unwrap();
        config.set("i64", 42i64).unwrap();
        config.set("i128", 42i128).unwrap();
        config.set("u8", 42u8).unwrap();
        config.set("u16", 42u16).unwrap();
        config.set("u32", 42u32).unwrap();
        config.set("u64", 42u64).unwrap();
        config.set("u128", 42u128).unwrap();
        assert_eq!(config.get::<i8>("i8").unwrap(), 42);
        assert_eq!(config.get::<i16>("i16").unwrap(), 42);
        assert_eq!(config.get::<i32>("i32").unwrap(), 42);
        assert_eq!(config.get::<i64>("i64").unwrap(), 42);
        assert_eq!(config.get::<i128>("i128").unwrap(), 42);
        assert_eq!(config.get::<u8>("u8").unwrap(), 42);
        assert_eq!(config.get::<u16>("u16").unwrap(), 42);
        assert_eq!(config.get::<u32>("u32").unwrap(), 42);
        assert_eq!(config.get::<u64>("u64").unwrap(), 42);
        assert_eq!(config.get::<u128>("u128").unwrap(), 42);
    }

    #[test]
    fn test_set_all_float_types() {
        let mut config = Config::new();

        config.set("f32", 3.5f32).unwrap();
        config.set("f64", 3.5f64).unwrap();

        assert_eq!(config.get::<f32>("f32").unwrap(), 3.5);
        assert_eq!(config.get::<f64>("f64").unwrap(), 3.5);
    }

    #[cfg(feature = "chrono")]
    #[test]
    fn test_set_all_other_types() {
        let mut config = Config::new();

        config.set("bool", true).unwrap();
        config.set("char", 'A').unwrap();
        config.set("string", "test").unwrap();
        config.set("str", "test".to_string()).unwrap();

        let date = NaiveDate::from_ymd_opt(2023, 12, 25).unwrap();
        let time = NaiveTime::from_hms_opt(12, 30, 45).unwrap();
        let datetime = DateTime::<Utc>::from_timestamp(1703505600, 0)
            .unwrap()
            .naive_utc();
        let utc_datetime =
            DateTime::<Utc>::from_timestamp(1703505600, 0).unwrap();

        config.set("date", date).unwrap();
        config.set("time", time).unwrap();
        config.set("datetime", datetime).unwrap();
        config.set("utc_datetime", utc_datetime).unwrap();

        assert!(config.get::<bool>("bool").unwrap());
        assert_eq!(config.get::<char>("char").unwrap(), 'A');
        assert_eq!(config.get::<String>("string").unwrap(), "test");
        assert_eq!(config.get::<NaiveDate>("date").unwrap(), date);
        assert_eq!(config.get::<NaiveTime>("time").unwrap(), time);
        assert_eq!(config.get::<NaiveDateTime>("datetime").unwrap(), datetime);
        assert_eq!(
            config.get::<DateTime<Utc>>("utc_datetime").unwrap(),
            utc_datetime
        );
    }
}

// ============================================================================
// Core Generic Method Tests - add<T>
// ============================================================================

#[cfg(test)]
mod test_add {
    #[allow(unused_imports)]
    use super::{
        Config,
        ConfigError,
        DataType,
        Deserialize,
        MultiValues,
        Property,
        create_test_config,
        create_test_config_with_description,
    };

    #[test]
    fn test_add_creates_new_property() {
        let mut config = Config::new();
        config.add("test", 42).unwrap();
        let value: i32 = config.get("test").unwrap();
        assert_eq!(value, 42);
    }

    #[test]
    fn test_add_appends_to_existing_property() {
        let mut config = Config::new();
        config.set("test", vec![1, 2]).unwrap();
        config.add("test", 3).unwrap();
        let values: Vec<i32> = config.get_list("test").unwrap();
        assert_eq!(values, vec![1, 2, 3]);
    }

    #[test]
    fn test_add_multiple_values() {
        let mut config = Config::new();
        config.add("test", 1).unwrap();
        config.add("test", 2).unwrap();
        config.add("test", 3).unwrap();
        let values: Vec<i32> = config.get_list("test").unwrap();
        assert_eq!(values, vec![1, 2, 3]);
    }

    #[test]
    fn test_add_string_values() {
        let mut config = Config::new();
        config.add("test", "value1").unwrap();
        config.add("test", "value2").unwrap();
        let values: Vec<String> = config.get_list("test").unwrap();
        assert_eq!(values, vec!["value1", "value2"]);
    }

    #[test]
    fn test_add_type_mismatch() {
        let mut config = Config::new();
        config.set("test", "string").unwrap();
        let result = config.add("test", 42);
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(ConfigError::TypeMismatch { key, .. }) if key == "test"
        ));
    }
}

// ============================================================================
// String Special Handling Tests
// ============================================================================

#[cfg(test)]
mod test_get_string {
    #[allow(unused_imports)]
    use super::{
        Config,
        ConfigError,
        DataType,
        Deserialize,
        MultiValues,
        Property,
        create_test_config,
        create_test_config_with_description,
    };

    #[test]
    fn test_get_string_returns_string_value() {
        let mut config = Config::new();
        config.set("test", "value").unwrap();
        let value = config.get::<String>("test").unwrap();
        assert_eq!(value, "value");
    }

    #[test]
    fn test_get_string_not_found() {
        let config = Config::new();
        let result = config.get::<String>("nonexistent");
        assert!(result.is_err());
        assert!(matches!(result, Err(ConfigError::PropertyNotFound(_))));
    }

    #[test]
    fn test_get_string_type_mismatch() {
        let mut config = Config::new();
        config.set("test", 42).unwrap();
        let value = config.get::<String>("test").unwrap();
        assert_eq!(value, "42");
    }

    #[test]
    fn test_get_string_preserves_placeholders() {
        let mut config = Config::new();
        config.set("test", "${other}").unwrap();
        let value = config.get::<String>("test").unwrap();
        assert_eq!(value, "${other}");
    }
}

#[cfg(test)]
mod test_get_string_or {
    #[allow(unused_imports)]
    use super::{
        Config,
        ConfigError,
        DataType,
        Deserialize,
        MultiValues,
        Property,
        create_test_config,
        create_test_config_with_description,
    };

    #[test]
    fn test_get_string_or_returns_value_when_property_exists() {
        let mut config = Config::new();
        config.set("test", "value").unwrap();
        let value = config.get_or::<String>("test", "default").unwrap();
        assert_eq!(value, "value");
    }

    #[test]
    fn test_get_string_or_returns_default_when_property_not_exists() {
        let config = Config::new();
        let value = config.get_or::<String>("nonexistent", "default").unwrap();
        assert_eq!(value, "default");
    }

    #[test]
    fn test_get_string_or_converts_non_string_value() {
        let mut config = Config::new();
        config.set("test", 42).unwrap();
        let value = config.get_or::<String>("test", "default").unwrap();
        assert_eq!(value, "42");
    }
}

// ============================================================================
// get_string_list Tests
// ============================================================================

#[cfg(test)]
mod test_get_string_list {
    #[allow(unused_imports)]
    use super::{
        Config,
        ConfigError,
        DataType,
        Deserialize,
        MultiValues,
        Property,
        create_test_config,
        create_test_config_with_description,
    };

    #[test]
    fn test_get_string_list_returns_string_list() {
        let mut config = Config::new();
        config
            .set("test", vec!["value1", "value2", "value3"])
            .unwrap();
        let values = config.get::<Vec<String>>("test").unwrap();
        assert_eq!(values, vec!["value1", "value2", "value3"]);
    }

    #[test]
    fn test_get_string_list_with_variable_substitution() {
        let mut config = Config::new();
        config.set("base", "http://localhost").unwrap();
        config
            .set("urls", vec!["${base}/api", "${base}/admin"])
            .unwrap();
        let urls = config.get_interpolated::<Vec<String>>("urls").unwrap();
        assert_eq!(
            urls,
            vec!["http://localhost/api", "http://localhost/admin"]
        );
    }

    #[test]
    fn test_get_string_list_with_nested_variable_substitution() {
        let mut config = Config::new();
        config.set("host", "localhost").unwrap();
        config.set("base", "http://${host}").unwrap();
        config
            .set("urls", vec!["${base}/api", "${base}/admin"])
            .unwrap();
        let urls = config.get_interpolated::<Vec<String>>("urls").unwrap();
        assert_eq!(
            urls,
            vec!["http://localhost/api", "http://localhost/admin"]
        );
    }

    #[test]
    fn test_get_string_list_preserves_placeholders() {
        let mut config = Config::new();
        config.set("base", "http://localhost").unwrap();
        config
            .set("urls", vec!["${base}/api", "${base}/admin"])
            .unwrap();
        let urls = config.get::<Vec<String>>("urls").unwrap();
        assert_eq!(urls, vec!["${base}/api", "${base}/admin"]);
    }

    #[test]
    fn test_get_string_list_not_found() {
        let config = Config::new();
        let result = config.get::<Vec<String>>("nonexistent");
        assert!(result.is_err());
        assert!(matches!(result, Err(ConfigError::PropertyNotFound(_))));
    }

    #[test]
    fn test_get_string_list_type_mismatch() {
        let mut config = Config::new();
        config.set("test", vec![1, 2, 3]).unwrap();
        let values = config.get::<Vec<String>>("test").unwrap();
        assert_eq!(values, vec!["1", "2", "3"]);
    }

    #[test]
    fn test_get_string_list_empty_list() {
        let mut config = Config::new();
        config.set("test", Vec::<String>::new()).unwrap();
        let values = config.get::<Vec<String>>("test").unwrap();
        assert_eq!(values, Vec::<String>::new());
    }
}

// ============================================================================
// get_string_list_or Tests
// ============================================================================

#[cfg(test)]
mod test_get_string_list_or {
    #[allow(unused_imports)]
    use super::{
        Config,
        ConfigError,
        DataType,
        Deserialize,
        MultiValues,
        Property,
        create_test_config,
        create_test_config_with_description,
    };

    #[test]
    fn test_get_string_list_or_returns_value_when_property_exists() {
        let mut config = Config::new();
        config.set("test", vec!["value1", "value2"]).unwrap();
        let values =
            config.get_or::<Vec<String>>("test", &["default"]).unwrap();
        assert_eq!(values, vec!["value1", "value2"]);
    }

    #[test]
    fn test_get_string_list_or_returns_default_when_property_not_exists() {
        let config = Config::new();
        let values = config
            .get_or::<Vec<String>>("nonexistent", &["default"])
            .unwrap();
        assert_eq!(values, vec!["default"]);
    }

    #[test]
    fn test_get_string_list_or_converts_non_string_values() {
        let mut config = Config::new();
        config.set("test", vec![1, 2, 3]).unwrap();
        let values =
            config.get_or::<Vec<String>>("test", &["default"]).unwrap();
        assert_eq!(values, vec!["1", "2", "3"]);
    }

    #[test]
    fn test_get_string_list_or_with_variable_substitution() {
        let mut config = Config::new();
        config.set("base", "http://localhost").unwrap();
        config
            .set("urls", vec!["${base}/api", "${base}/admin"])
            .unwrap();
        let urls = config
            .get_interpolated_or::<Vec<String>>("urls", &["default"])
            .unwrap();
        assert_eq!(
            urls,
            vec!["http://localhost/api", "http://localhost/admin"]
        );
    }

    #[test]
    fn test_get_string_list_or_with_array_default() {
        let config = Config::new();
        let values = config
            .get_or::<Vec<String>>("nonexistent", &["default1", "default2"])
            .unwrap();
        assert_eq!(values, vec!["default1", "default2"]);
    }

    #[test]
    fn test_get_string_list_or_with_vec_default() {
        let config = Config::new();
        let default_vec = vec!["vec1", "vec2", "vec3"];
        let values = config
            .get_or::<Vec<String>>("nonexistent", &default_vec)
            .unwrap();
        assert_eq!(values, vec!["vec1", "vec2", "vec3"]);
    }
}

// ============================================================================
// Default Trait Tests
// ============================================================================

#[cfg(test)]
mod test_default {
    #[allow(unused_imports)]
    use super::{
        Config,
        ConfigError,
        DataType,
        Deserialize,
        InterpolationSources,
        MultiValues,
        Property,
        create_test_config,
        create_test_config_with_description,
    };

    #[test]
    fn test_default_creates_empty_config() {
        let config = Config::default();
        assert!(config.is_empty());
        assert_eq!(config.len(), 0);
        assert!(config.description().is_none());
        assert_eq!(
            config.default_read_policy().interpolation_sources(),
            InterpolationSources::ConfigOnly
        );
        assert_eq!(config.default_read_policy().max_interpolation_depth(), 64);
    }

    #[test]
    fn test_default_equals_new() {
        let config1 = Config::new();
        let config2 = Config::default();
        assert_eq!(config1, config2);
    }
}

// ============================================================================
// Final Property Tests
// ============================================================================

#[cfg(test)]
mod test_final_property {
    #[allow(unused_imports)]
    use super::{
        Config,
        ConfigError,
        DataType,
        Deserialize,
        MultiValues,
        Property,
        create_test_config,
        create_test_config_with_description,
    };

    #[test]
    fn test_set_final_property_fails() {
        let mut config = Config::new();

        // Set initial value
        config.set("immutable_key", "initial_value").unwrap();

        config.set_final("immutable_key", true).unwrap();

        // Try to set again - should fail
        let result = config.set("immutable_key", "new_value");
        assert!(matches!(result, Err(ConfigError::PropertyIsFinal(_))));

        // Verify error message
        if let Err(ConfigError::PropertyIsFinal(name)) = result {
            assert_eq!(name, "immutable_key");
        }

        // Original value should remain unchanged
        let value: String = config.get("immutable_key").unwrap();
        assert_eq!(value, "initial_value");
    }

    #[test]
    fn test_add_to_final_property_fails() {
        let mut config = Config::new();

        // Set initial value
        config
            .set("immutable_list", vec!["value1", "value2"])
            .unwrap();

        config.set_final("immutable_list", true).unwrap();

        // Try to add - should fail
        let result = config.add("immutable_list", "value3");
        assert!(matches!(result, Err(ConfigError::PropertyIsFinal(_))));

        // Verify error message
        if let Err(ConfigError::PropertyIsFinal(name)) = result {
            assert_eq!(name, "immutable_list");
        }

        // Original values should remain unchanged
        let values: Vec<String> = config.get_list("immutable_list").unwrap();
        assert_eq!(values, vec!["value1", "value2"]);
    }

    #[test]
    fn test_set_non_final_property_succeeds() {
        let mut config = Config::new();

        // Set initial value (not final)
        config.set("mutable_key", "initial_value").unwrap();

        // Should be able to update
        config.set("mutable_key", "new_value").unwrap();

        let value: String = config.get("mutable_key").unwrap();
        assert_eq!(value, "new_value");
    }

    #[test]
    fn test_set_final_missing_property_returns_error() {
        let mut config = Config::new();
        let result = config.set_final("missing", true);
        assert!(matches!(result, Err(ConfigError::PropertyNotFound(_))));
    }

    #[test]
    fn test_set_final_cannot_unset_final_property() {
        let mut config = Config::new();
        config.set("key", "value").unwrap();
        config.set_final("key", false).unwrap();
        config.set_final("key", true).unwrap();
        config.set_final("key", true).unwrap();

        let result = config.set_final("key", false);
        assert!(matches!(result, Err(ConfigError::PropertyIsFinal(_))));
        assert_eq!(config.get::<String>("key").unwrap(), "value");
    }

    #[test]
    fn test_add_to_non_final_property_succeeds() {
        let mut config = Config::new();

        // Set initial value (not final)
        config.set("mutable_list", vec!["value1"]).unwrap();

        // Should be able to add
        config.add("mutable_list", "value2").unwrap();

        let values: Vec<String> = config.get_list("mutable_list").unwrap();
        assert_eq!(values, vec!["value1", "value2"]);
    }

    #[test]
    fn test_final_property_with_different_types() {
        let mut config = Config::new();

        // Test with integer
        config.set("final_int", 42).unwrap();
        config.set_final("final_int", true).unwrap();
        assert!(config.set("final_int", 100).is_err());

        // Test with boolean
        config.set("final_bool", true).unwrap();
        config.set_final("final_bool", true).unwrap();
        assert!(config.set("final_bool", false).is_err());

        // Test with float
        config.set("final_float", 3.15).unwrap();
        config.set_final("final_float", true).unwrap();
        assert!(config.set("final_float", 2.72).is_err());
    }
}

#[cfg(test)]
mod test_iter {
    #[allow(unused_imports)]
    use super::{
        Config,
        ConfigError,
        DataType,
        Deserialize,
        MultiValues,
        Property,
        create_test_config,
        create_test_config_with_description,
    };

    #[test]
    fn test_iter_empty_config() {
        let config = Config::new();
        let entries: Vec<_> = config.iter().collect();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_iter_single_entry() {
        let mut config = Config::new();
        config.set("host", "localhost").unwrap();
        let entries: Vec<_> = config.iter().collect();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].0, "host");
    }

    #[test]
    fn test_iter_multiple_entries() {
        let mut config = Config::new();
        config.set("host", "localhost").unwrap();
        config.set("port", 8080).unwrap();
        config.set("debug", true).unwrap();
        let entries: Vec<_> = config.iter().collect();
        assert_eq!(entries.len(), 3);
        let keys: Vec<&str> = entries.iter().map(|(k, _)| *k).collect();
        assert!(keys.contains(&"host"));
        assert!(keys.contains(&"port"));
        assert!(keys.contains(&"debug"));
    }

    #[test]
    fn test_iter_yields_property_references() {
        let mut config = Config::new();
        config.set("x", 42).unwrap();
        for (key, prop) in config.iter() {
            assert_eq!(key, "x");
            assert!(!prop.is_unset());
        }
    }
}
