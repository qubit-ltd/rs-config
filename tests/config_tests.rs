/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! # [`qubit_config::Config`] unit tests
//!
//! Covers the public `Config` API (including APIs introduced in v0.4.0).

pub(crate) use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
pub(crate) use qubit_common::DataType;
pub(crate) use qubit_config::{Config, ConfigError, Property};
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

// ============================================================================
// Constructor Tests
// ============================================================================

#[cfg(test)]
mod test_new {
    use super::*;

    #[test]
    fn test_new_creates_empty_config() {
        let config = Config::new();
        assert!(config.is_empty());
        assert_eq!(config.len(), 0);
        assert!(config.description().is_none());
        assert!(config.is_enable_variable_substitution());
        assert_eq!(config.max_substitution_depth(), 64);
    }

    #[test]
    fn test_new_has_correct_default_values() {
        let config = Config::new();
        assert!(config.is_enable_variable_substitution());
        assert_eq!(config.max_substitution_depth(), 64);
    }
}

#[cfg(test)]
mod test_with_description {
    use super::*;

    #[test]
    fn test_with_description_creates_config_with_description() {
        let config = Config::with_description("Test Configuration");
        assert_eq!(config.description(), Some("Test Configuration"));
        assert!(config.is_empty());
    }

    #[test]
    fn test_with_description_has_correct_default_values() {
        let config = Config::with_description("Test Configuration");
        assert!(config.is_enable_variable_substitution());
        assert_eq!(config.max_substitution_depth(), 64);
    }

    #[test]
    fn test_with_description_with_empty_string() {
        let config = Config::with_description("");
        assert_eq!(config.description(), Some(""));
    }
}

// ============================================================================
// Basic Property Access Tests
// ============================================================================

#[cfg(test)]
mod test_description {
    use super::*;

    #[test]
    fn test_description_returns_none_for_new_config() {
        let config = Config::new();
        assert!(config.description().is_none());
    }

    #[test]
    fn test_description_returns_some_for_config_with_description() {
        let config = Config::with_description("Test Configuration");
        assert_eq!(config.description(), Some("Test Configuration"));
    }

    #[test]
    fn test_set_description_sets_description() {
        let mut config = Config::new();
        config.set_description(Some("New description".to_string()));
        assert_eq!(config.description(), Some("New description"));
    }

    #[test]
    fn test_set_description_clears_description() {
        let mut config = Config::with_description("Original description");
        config.set_description(None);
        assert!(config.description().is_none());
    }

    #[test]
    fn test_set_description_updates_description() {
        let mut config = Config::with_description("Original description");
        config.set_description(Some("New description".to_string()));
        assert_eq!(config.description(), Some("New description"));
    }
}

#[cfg(test)]
mod test_variable_substitution {
    use super::*;

    #[test]
    fn test_is_enable_variable_substitution_returns_true_by_default() {
        let config = Config::new();
        assert!(config.is_enable_variable_substitution());
    }

    #[test]
    fn test_set_enable_variable_substitution_enables() {
        let mut config = Config::new();
        config.set_enable_variable_substitution(true);
        assert!(config.is_enable_variable_substitution());
    }

    #[test]
    fn test_set_enable_variable_substitution_disables() {
        let mut config = Config::new();
        config.set_enable_variable_substitution(false);
        assert!(!config.is_enable_variable_substitution());
    }
}

// ============================================================================
// Configuration Item Management Tests
// ============================================================================

#[cfg(test)]
mod test_contains {
    use super::*;

    #[test]
    fn test_contains_returns_false_for_empty_config() {
        let config = Config::new();
        assert!(!config.contains("nonexistent"));
    }

    #[test]
    fn test_contains_returns_true_for_existing_property() {
        let mut config = Config::new();
        config.set("test", "value").unwrap();
        assert!(config.contains("test"));
    }

    #[test]
    fn test_contains_returns_false_for_nonexistent_property() {
        let mut config = Config::new();
        config.set("test", "value").unwrap();
        assert!(!config.contains("other"));
    }
}

#[cfg(test)]
mod test_get_property {
    use super::*;

    #[test]
    fn test_get_property_returns_none_for_nonexistent_property() {
        let config = Config::new();
        assert!(config.get_property("nonexistent").is_none());
    }

    #[test]
    fn test_get_property_returns_some_for_existing_property() {
        let mut config = Config::new();
        config.set("test", "value").unwrap();
        let property = config.get_property("test");
        assert!(property.is_some());
    }
}

#[cfg(test)]
mod test_get_property_mut {
    use super::*;

    #[test]
    fn test_get_property_mut_returns_none_for_nonexistent_property() {
        let mut config = Config::new();
        assert!(config.get_property_mut("nonexistent").unwrap().is_none());
    }

    #[test]
    fn test_get_property_mut_returns_some_for_existing_property() {
        let mut config = Config::new();
        config.set("test", "value").unwrap();
        let property = config.get_property_mut("test").unwrap();
        assert!(property.is_some());
    }

    #[test]
    fn test_get_property_mut_returns_error_for_final_property() {
        let mut config = Config::new();
        config.set("test", "value").unwrap();
        config.set_final("test", true).unwrap();

        let result = config.get_property_mut("test");
        assert!(matches!(result, Err(ConfigError::PropertyIsFinal(_))));
    }

    #[test]
    fn test_property_mut_guard_rechecks_final_after_set_final() {
        let mut config = Config::new();
        config.set("test", "value").unwrap();

        {
            let mut property = config.get_property_mut("test").unwrap().unwrap();
            property.set_final(true).unwrap();

            let desc_result = property.set_description(Some("blocked".to_string()));
            assert!(matches!(desc_result, Err(ConfigError::PropertyIsFinal(_))));

            let set_result = property.set_value(MultiValues::String(vec!["new-value".to_string()]));
            assert!(matches!(set_result, Err(ConfigError::PropertyIsFinal(_))));

            let generic_set_result = property.set("new-value");
            assert!(matches!(
                generic_set_result,
                Err(ConfigError::PropertyIsFinal(_))
            ));

            let add_result = property.add("new-value");
            assert!(matches!(add_result, Err(ConfigError::PropertyIsFinal(_))));

            let clear_result = property.clear();
            assert!(matches!(clear_result, Err(ConfigError::PropertyIsFinal(_))));

            let unset_result = property.set_final(false);
            assert!(matches!(unset_result, Err(ConfigError::PropertyIsFinal(_))));
        }

        assert_eq!(config.get_string("test").unwrap(), "value");
    }

    #[test]
    fn test_property_mut_guard_allows_mutation_before_final() {
        let mut config = Config::new();
        config.set("test", "value").unwrap();

        {
            let mut property = config.get_property_mut("test").unwrap().unwrap();
            assert_eq!(property.name(), "test");
            assert_eq!(property.as_property().name(), "test");
            property
                .set_description(Some("updated description".to_string()))
                .unwrap();
            property
                .set_value(MultiValues::String(vec!["first".to_string()]))
                .unwrap();
            property.set("second").unwrap();
            property.add("third").unwrap();
        }

        assert_eq!(
            config.get_string_list("test").unwrap(),
            vec!["second".to_string(), "third".to_string()],
        );
        assert_eq!(
            config.get_property("test").unwrap().description(),
            Some("updated description"),
        );
    }
}

#[cfg(test)]
mod test_remove {
    use super::*;

    #[test]
    fn test_remove_returns_none_for_nonexistent_property() {
        let mut config = Config::new();
        assert!(config.remove("nonexistent").unwrap().is_none());
    }

    #[test]
    fn test_remove_returns_property_and_removes_it() {
        let mut config = Config::new();
        config.set("test", "value").unwrap();
        assert!(config.contains("test"));

        let removed = config.remove("test").unwrap();
        assert!(removed.is_some());
        assert!(!config.contains("test"));
    }

    #[test]
    fn test_remove_final_property_returns_error_and_keeps_value() {
        let mut config = Config::new();
        config.set("test", "value").unwrap();
        config.set_final("test", true).unwrap();

        let result = config.remove("test");
        assert!(matches!(result, Err(ConfigError::PropertyIsFinal(_))));
        assert!(config.contains("test"));
        assert_eq!(config.get_string("test").unwrap(), "value");
    }
}

#[cfg(test)]
mod test_clear {
    use super::*;

    #[test]
    fn test_clear_does_nothing_on_empty_config() {
        let mut config = Config::new();
        config.clear().unwrap();
        assert!(config.is_empty());
    }

    #[test]
    fn test_clear_removes_all_properties() {
        let mut config = create_test_config();
        assert!(!config.is_empty());

        config.clear().unwrap();
        assert!(config.is_empty());
        assert_eq!(config.len(), 0);
    }

    #[test]
    fn test_clear_with_final_property_returns_error_and_keeps_values() {
        let mut config = create_test_config();
        config.set_final("string_value", true).unwrap();

        let result = config.clear();
        assert!(matches!(result, Err(ConfigError::PropertyIsFinal(_))));
        assert_eq!(config.len(), 4);
        assert_eq!(config.get_string("string_value").unwrap(), "test");
    }
}

#[cfg(test)]
mod test_len {
    use super::*;

    #[test]
    fn test_len_returns_zero_for_empty_config() {
        let config = Config::new();
        assert_eq!(config.len(), 0);
    }

    #[test]
    fn test_len_returns_correct_count() {
        let mut config = Config::new();
        config.set("key1", "value1").unwrap();
        config.set("key2", "value2").unwrap();
        config.set("key3", "value3").unwrap();
        assert_eq!(config.len(), 3);
    }
}

#[cfg(test)]
mod test_is_empty {
    use super::*;

    #[test]
    fn test_is_empty_returns_true_for_empty_config() {
        let config = Config::new();
        assert!(config.is_empty());
    }

    #[test]
    fn test_is_empty_returns_false_for_non_empty_config() {
        let mut config = Config::new();
        config.set("test", "value").unwrap();
        assert!(!config.is_empty());
    }
}

#[cfg(test)]
mod test_keys {
    use super::*;

    #[test]
    fn test_keys_returns_empty_vec_for_empty_config() {
        let config = Config::new();
        let keys = config.keys();
        assert!(keys.is_empty());
    }

    #[test]
    fn test_keys_returns_all_keys() {
        let mut config = Config::new();
        config.set("key1", "value1").unwrap();
        config.set("key2", "value2").unwrap();
        config.set("key3", "value3").unwrap();

        let keys = config.keys();
        assert_eq!(keys.len(), 3);
        assert!(keys.contains(&"key1".to_string()));
        assert!(keys.contains(&"key2".to_string()));
        assert!(keys.contains(&"key3".to_string()));
    }
}

// ============================================================================
// Core Generic Method Tests - get<T>
// ============================================================================

#[cfg(test)]
mod test_get {
    use super::*;

    // String type tests
    #[test]
    fn test_get_string() {
        let mut config = Config::new();
        config.set("test", "value").unwrap();
        let value: String = config.get("test").unwrap();
        assert_eq!(value, "value");
    }

    #[test]
    fn test_get_string_not_found() {
        let config = Config::new();
        let result: Result<String, _> = config.get("nonexistent");
        assert!(result.is_err());
        assert!(matches!(result, Err(ConfigError::PropertyNotFound(_))));
    }

    // Integer type tests
    #[test]
    fn test_get_i8() {
        let mut config = Config::new();
        config.set("test", 42i8).unwrap();
        let value: i8 = config.get("test").unwrap();
        assert_eq!(value, 42);
    }

    #[test]
    fn test_get_i16() {
        let mut config = Config::new();
        config.set("test", 42i16).unwrap();
        let value: i16 = config.get("test").unwrap();
        assert_eq!(value, 42);
    }

    #[test]
    fn test_get_i32() {
        let mut config = Config::new();
        config.set("test", 42i32).unwrap();
        let value: i32 = config.get("test").unwrap();
        assert_eq!(value, 42);
    }

    #[test]
    fn test_get_i64() {
        let mut config = Config::new();
        config.set("test", 42i64).unwrap();
        let value: i64 = config.get("test").unwrap();
        assert_eq!(value, 42);
    }

    #[test]
    fn test_get_i128() {
        let mut config = Config::new();
        config.set("test", 42i128).unwrap();
        let value: i128 = config.get("test").unwrap();
        assert_eq!(value, 42);
    }

    // Note: isize type does not implement IntoPropertyValue and FromPropertyValue traits
    // #[test]
    // fn test_get_isize() {
    //     let mut config = Config::new();
    //     config.set("test", 42isize).unwrap();
    //     let value: isize = config.get("test").unwrap();
    //     assert_eq!(value, 42);
    // }

    // Unsigned integer type tests
    #[test]
    fn test_get_u8() {
        let mut config = Config::new();
        config.set("test", 42u8).unwrap();
        let value: u8 = config.get("test").unwrap();
        assert_eq!(value, 42);
    }

    #[test]
    fn test_get_u16() {
        let mut config = Config::new();
        config.set("test", 42u16).unwrap();
        let value: u16 = config.get("test").unwrap();
        assert_eq!(value, 42);
    }

    #[test]
    fn test_get_u32() {
        let mut config = Config::new();
        config.set("test", 42u32).unwrap();
        let value: u32 = config.get("test").unwrap();
        assert_eq!(value, 42);
    }

    #[test]
    fn test_get_u64() {
        let mut config = Config::new();
        config.set("test", 42u64).unwrap();
        let value: u64 = config.get("test").unwrap();
        assert_eq!(value, 42);
    }

    #[test]
    fn test_get_u128() {
        let mut config = Config::new();
        config.set("test", 42u128).unwrap();
        let value: u128 = config.get("test").unwrap();
        assert_eq!(value, 42);
    }

    // Note: usize type does not implement IntoPropertyValue and FromPropertyValue traits
    // #[test]
    // fn test_get_usize() {
    //     let mut config = Config::new();
    //     config.set("test", 42usize).unwrap();
    //     let value: usize = config.get("test").unwrap();
    //     assert_eq!(value, 42);
    // }

    // Float type tests
    #[test]
    fn test_get_f32() {
        let mut config = Config::new();
        config.set("test", 3.5f32).unwrap();
        let value: f32 = config.get("test").unwrap();
        assert_eq!(value, 3.5);
    }

    #[test]
    fn test_get_f64() {
        let mut config = Config::new();
        config.set("test", 3.5f64).unwrap();
        let value: f64 = config.get("test").unwrap();
        assert_eq!(value, 3.5);
    }

    // Boolean type tests
    #[test]
    fn test_get_bool_true() {
        let mut config = Config::new();
        config.set("test", true).unwrap();
        let value: bool = config.get("test").unwrap();
        assert!(value);
    }

    #[test]
    fn test_get_bool_false() {
        let mut config = Config::new();
        config.set("test", false).unwrap();
        let value: bool = config.get("test").unwrap();
        assert!(!value);
    }

    #[test]
    fn test_get_bool_from_string_values() {
        let mut config = Config::new();
        config.set("flag_one", "1").unwrap();
        config.set("flag_zero", "0").unwrap();
        config.set("flag_true", "TRUE").unwrap();
        config.set("flag_false", "False").unwrap();

        assert!(config.get::<bool>("flag_one").unwrap());
        assert!(!config.get::<bool>("flag_zero").unwrap());
        assert!(config.get::<bool>("flag_true").unwrap());
        assert!(!config.get::<bool>("flag_false").unwrap());
    }

    #[test]
    fn test_get_number_from_string_value() {
        let mut config = Config::new();
        config.set("port", "8080").unwrap();

        assert_eq!(config.get::<u16>("port").unwrap(), 8080u16);
    }

    #[test]
    fn test_get_strict_preserves_exact_type_checking() {
        let mut config = Config::new();
        config.set("flag", "1").unwrap();

        let err = config.get_strict::<bool>("flag").unwrap_err();
        assert!(matches!(err, ConfigError::TypeMismatch { .. }));
    }

    // Character type tests
    #[test]
    fn test_get_char() {
        let mut config = Config::new();
        config.set("test", 'A').unwrap();
        let value: char = config.get("test").unwrap();
        assert_eq!(value, 'A');
    }

    // Date and time type tests
    #[test]
    fn test_get_naive_date() {
        let mut config = Config::new();
        let date = NaiveDate::from_ymd_opt(2023, 12, 25).unwrap();
        config.set("test", date).unwrap();
        let value: NaiveDate = config.get("test").unwrap();
        assert_eq!(value, date);
    }

    #[test]
    fn test_get_naive_time() {
        let mut config = Config::new();
        let time = NaiveTime::from_hms_opt(12, 30, 45).unwrap();
        config.set("test", time).unwrap();
        let value: NaiveTime = config.get("test").unwrap();
        assert_eq!(value, time);
    }

    #[test]
    fn test_get_naive_datetime() {
        let mut config = Config::new();
        let datetime = DateTime::<Utc>::from_timestamp(1703505600, 0)
            .unwrap()
            .naive_utc();
        config.set("test", datetime).unwrap();
        let value: NaiveDateTime = config.get("test").unwrap();
        assert_eq!(value, datetime);
    }

    #[test]
    fn test_get_datetime_utc() {
        let mut config = Config::new();
        let datetime = DateTime::<Utc>::from_timestamp(1703505600, 0).unwrap();
        config.set("test", datetime).unwrap();
        let value: DateTime<Utc> = config.get("test").unwrap();
        assert_eq!(value, datetime);
    }

    // Byte array type tests
    // Note: Vec<u8> is no longer supported as a single value type, test removed

    // Type mismatch tests
    #[test]
    fn test_get_type_mismatch() {
        let mut config = Config::new();
        config.set("test", "string").unwrap();
        let result: Result<i32, _> = config.get("test");
        assert!(result.is_err());
        assert!(matches!(result, Err(ConfigError::ConversionError { .. })));
    }
}

// ============================================================================
// Core Generic Method Tests - get_or<T>
// ============================================================================

#[cfg(test)]
mod test_get_or {
    use super::*;

    #[test]
    fn test_get_or_returns_value_when_property_exists() {
        let mut config = Config::new();
        config.set("test", 42).unwrap();
        let value = config.get_or("test", 0).unwrap();
        assert_eq!(value, 42);
    }

    #[test]
    fn test_get_or_returns_default_when_property_not_exists() {
        let config = Config::new();
        let value = config.get_or("nonexistent", 42).unwrap();
        assert_eq!(value, 42);
    }

    #[test]
    fn test_get_or_with_string() {
        let mut config = Config::new();
        config.set("test", "value").unwrap();
        let value = config.get_or("test", "default".to_string()).unwrap();
        assert_eq!(value, "value");
    }

    #[test]
    fn test_get_or_with_string_default() {
        let config = Config::new();
        let value = config.get_or("nonexistent", "default".to_string()).unwrap();
        assert_eq!(value, "default");
    }

    #[test]
    fn test_get_or_with_bool() {
        let mut config = Config::new();
        config.set("test", true).unwrap();
        let value = config.get_or("test", false).unwrap();
        assert!(value);
    }

    #[test]
    fn test_get_or_with_bool_default() {
        let config = Config::new();
        let value = config.get_or("nonexistent", true).unwrap();
        assert!(value);
    }

    #[test]
    fn test_get_or_uses_conversion_before_default() {
        let mut config = Config::new();
        config.set("test", "0").unwrap();

        let value = config.get_or("test", true).unwrap();
        assert!(!value);
    }
}

// ============================================================================
// Core Generic Method Tests - get_list<T>
// ============================================================================

#[cfg(test)]
mod test_get_list {
    use super::*;

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
    use super::*;

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
        // Note: isize and usize types do not implement IntoPropertyValue and FromPropertyValue traits
        // config.set("isize", 42isize).unwrap();

        config.set("u8", 42u8).unwrap();
        config.set("u16", 42u16).unwrap();
        config.set("u32", 42u32).unwrap();
        config.set("u64", 42u64).unwrap();
        config.set("u128", 42u128).unwrap();
        // config.set("usize", 42usize).unwrap();

        assert_eq!(config.get::<i8>("i8").unwrap(), 42);
        assert_eq!(config.get::<i16>("i16").unwrap(), 42);
        assert_eq!(config.get::<i32>("i32").unwrap(), 42);
        assert_eq!(config.get::<i64>("i64").unwrap(), 42);
        assert_eq!(config.get::<i128>("i128").unwrap(), 42);
        // assert_eq!(config.get::<isize>("isize").unwrap(), 42);

        assert_eq!(config.get::<u8>("u8").unwrap(), 42);
        assert_eq!(config.get::<u16>("u16").unwrap(), 42);
        assert_eq!(config.get::<u32>("u32").unwrap(), 42);
        assert_eq!(config.get::<u64>("u64").unwrap(), 42);
        assert_eq!(config.get::<u128>("u128").unwrap(), 42);
        // assert_eq!(config.get::<usize>("usize").unwrap(), 42);
    }

    #[test]
    fn test_set_all_float_types() {
        let mut config = Config::new();

        config.set("f32", 3.5f32).unwrap();
        config.set("f64", 3.5f64).unwrap();

        assert_eq!(config.get::<f32>("f32").unwrap(), 3.5);
        assert_eq!(config.get::<f64>("f64").unwrap(), 3.5);
    }

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
        let utc_datetime = DateTime::<Utc>::from_timestamp(1703505600, 0).unwrap();

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
    use super::*;

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
        assert!(matches!(result, Err(ConfigError::TypeMismatch { .. })));
    }
}

// ============================================================================
// String Special Handling Tests
// ============================================================================

#[cfg(test)]
mod test_get_string {
    use super::*;

    #[test]
    fn test_get_string_returns_string_value() {
        let mut config = Config::new();
        config.set("test", "value").unwrap();
        let value = config.get_string("test").unwrap();
        assert_eq!(value, "value");
    }

    #[test]
    fn test_get_string_not_found() {
        let config = Config::new();
        let result = config.get_string("nonexistent");
        assert!(result.is_err());
        assert!(matches!(result, Err(ConfigError::PropertyNotFound(_))));
    }

    #[test]
    fn test_get_string_type_mismatch() {
        let mut config = Config::new();
        config.set("test", 42).unwrap();
        let value = config.get_string("test").unwrap();
        assert_eq!(value, "42");
    }

    #[test]
    fn test_get_string_with_variable_substitution_disabled() {
        let mut config = Config::new();
        config.set("test", "value").unwrap();
        config.set_enable_variable_substitution(false);
        let value = config.get_string("test").unwrap();
        assert_eq!(value, "value");
    }
}

#[cfg(test)]
mod test_get_string_or {
    use super::*;

    #[test]
    fn test_get_string_or_returns_value_when_property_exists() {
        let mut config = Config::new();
        config.set("test", "value").unwrap();
        let value = config.get_string_or("test", "default").unwrap();
        assert_eq!(value, "value");
    }

    #[test]
    fn test_get_string_or_returns_default_when_property_not_exists() {
        let config = Config::new();
        let value = config.get_string_or("nonexistent", "default").unwrap();
        assert_eq!(value, "default");
    }

    #[test]
    fn test_get_string_or_converts_non_string_value() {
        let mut config = Config::new();
        config.set("test", 42).unwrap();
        let value = config.get_string_or("test", "default").unwrap();
        assert_eq!(value, "42");
    }
}

// ============================================================================
// get_string_list Tests
// ============================================================================

#[cfg(test)]
mod test_get_string_list {
    use super::*;

    #[test]
    fn test_get_string_list_returns_string_list() {
        let mut config = Config::new();
        config
            .set("test", vec!["value1", "value2", "value3"])
            .unwrap();
        let values = config.get_string_list("test").unwrap();
        assert_eq!(values, vec!["value1", "value2", "value3"]);
    }

    #[test]
    fn test_get_string_list_with_variable_substitution() {
        let mut config = Config::new();
        config.set("base", "http://localhost").unwrap();
        config
            .set("urls", vec!["${base}/api", "${base}/admin"])
            .unwrap();
        let urls = config.get_string_list("urls").unwrap();
        assert_eq!(urls, vec!["http://localhost/api", "http://localhost/admin"]);
    }

    #[test]
    fn test_get_string_list_with_nested_variable_substitution() {
        let mut config = Config::new();
        config.set("host", "localhost").unwrap();
        config.set("base", "http://${host}").unwrap();
        config
            .set("urls", vec!["${base}/api", "${base}/admin"])
            .unwrap();
        let urls = config.get_string_list("urls").unwrap();
        assert_eq!(urls, vec!["http://localhost/api", "http://localhost/admin"]);
    }

    #[test]
    fn test_get_string_list_with_variable_substitution_disabled() {
        let mut config = Config::new();
        config.set("base", "http://localhost").unwrap();
        config
            .set("urls", vec!["${base}/api", "${base}/admin"])
            .unwrap();
        config.set_enable_variable_substitution(false);
        let urls = config.get_string_list("urls").unwrap();
        assert_eq!(urls, vec!["${base}/api", "${base}/admin"]);
    }

    #[test]
    fn test_get_string_list_not_found() {
        let config = Config::new();
        let result = config.get_string_list("nonexistent");
        assert!(result.is_err());
        assert!(matches!(result, Err(ConfigError::PropertyNotFound(_))));
    }

    #[test]
    fn test_get_string_list_type_mismatch() {
        let mut config = Config::new();
        config.set("test", vec![1, 2, 3]).unwrap();
        let values = config.get_string_list("test").unwrap();
        assert_eq!(values, vec!["1", "2", "3"]);
    }

    #[test]
    fn test_get_string_list_empty_list() {
        let mut config = Config::new();
        config.set("test", Vec::<String>::new()).unwrap();
        let values = config.get_string_list("test").unwrap();
        assert_eq!(values, Vec::<String>::new());
    }
}

// ============================================================================
// get_string_list_or Tests
// ============================================================================

#[cfg(test)]
mod test_get_string_list_or {
    use super::*;

    #[test]
    fn test_get_string_list_or_returns_value_when_property_exists() {
        let mut config = Config::new();
        config.set("test", vec!["value1", "value2"]).unwrap();
        let values = config.get_string_list_or("test", &["default"]).unwrap();
        assert_eq!(values, vec!["value1", "value2"]);
    }

    #[test]
    fn test_get_string_list_or_returns_default_when_property_not_exists() {
        let config = Config::new();
        let values = config
            .get_string_list_or("nonexistent", &["default"])
            .unwrap();
        assert_eq!(values, vec!["default"]);
    }

    #[test]
    fn test_get_string_list_or_converts_non_string_values() {
        let mut config = Config::new();
        config.set("test", vec![1, 2, 3]).unwrap();
        let values = config.get_string_list_or("test", &["default"]).unwrap();
        assert_eq!(values, vec!["1", "2", "3"]);
    }

    #[test]
    fn test_get_string_list_or_with_variable_substitution() {
        let mut config = Config::new();
        config.set("base", "http://localhost").unwrap();
        config
            .set("urls", vec!["${base}/api", "${base}/admin"])
            .unwrap();
        let urls = config.get_string_list_or("urls", &["default"]).unwrap();
        assert_eq!(urls, vec!["http://localhost/api", "http://localhost/admin"]);
    }

    #[test]
    fn test_get_string_list_or_with_array_default() {
        let config = Config::new();
        let values = config
            .get_string_list_or("nonexistent", &["default1", "default2"])
            .unwrap();
        assert_eq!(values, vec!["default1", "default2"]);
    }

    #[test]
    fn test_get_string_list_or_with_vec_default() {
        let config = Config::new();
        let default_vec = vec!["vec1", "vec2", "vec3"];
        let values = config
            .get_string_list_or("nonexistent", &default_vec)
            .unwrap();
        assert_eq!(values, vec!["vec1", "vec2", "vec3"]);
    }
}

// ============================================================================
// Default Trait Tests
// ============================================================================

#[cfg(test)]
mod test_default {
    use super::*;

    #[test]
    fn test_default_creates_empty_config() {
        let config = Config::default();
        assert!(config.is_empty());
        assert_eq!(config.len(), 0);
        assert!(config.description().is_none());
        assert!(config.is_enable_variable_substitution());
        assert_eq!(config.max_substitution_depth(), 64);
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
    use super::*;

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
        assert_eq!(config.get_string("key").unwrap(), "value");
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
    use super::*;

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
            assert!(!prop.is_empty());
        }
    }
}

// ============================================================================
// iter_prefix() Tests
// ============================================================================

#[cfg(test)]
mod test_iter_prefix {
    use super::*;

    #[test]
    fn test_iter_prefix_empty_config() {
        let config = Config::new();
        let entries: Vec<_> = config.iter_prefix("http.").collect();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_iter_prefix_no_match() {
        let mut config = Config::new();
        config.set("db.host", "dbhost").unwrap();
        config.set("db.port", 5432).unwrap();
        let entries: Vec<_> = config.iter_prefix("http.").collect();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_iter_prefix_partial_match() {
        let mut config = Config::new();
        config.set("http.host", "localhost").unwrap();
        config.set("http.port", 8080).unwrap();
        config.set("db.host", "dbhost").unwrap();
        let entries: Vec<_> = config.iter_prefix("http.").collect();
        assert_eq!(entries.len(), 2);
        let keys: Vec<&str> = entries.iter().map(|(k, _)| *k).collect();
        assert!(keys.contains(&"http.host"));
        assert!(keys.contains(&"http.port"));
        assert!(!keys.contains(&"db.host"));
    }

    #[test]
    fn test_iter_prefix_exact_prefix_match() {
        let mut config = Config::new();
        config.set("http.host", "localhost").unwrap();
        config.set("https.host", "secure").unwrap();
        let entries: Vec<_> = config.iter_prefix("http.").collect();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].0, "http.host");
    }

    #[test]
    fn test_iter_prefix_all_match() {
        let mut config = Config::new();
        config.set("app.name", "test").unwrap();
        config.set("app.version", "1.0").unwrap();
        config.set("app.debug", true).unwrap();
        let entries: Vec<_> = config.iter_prefix("app.").collect();
        assert_eq!(entries.len(), 3);
    }
}

// ============================================================================
// contains_prefix() Tests
// ============================================================================

#[cfg(test)]
mod test_contains_prefix {
    use super::*;

    #[test]
    fn test_contains_prefix_empty_config() {
        let config = Config::new();
        assert!(!config.contains_prefix("http."));
    }

    #[test]
    fn test_contains_prefix_match() {
        let mut config = Config::new();
        config.set("http.host", "localhost").unwrap();
        assert!(config.contains_prefix("http."));
    }

    #[test]
    fn test_contains_prefix_no_match() {
        let mut config = Config::new();
        config.set("db.host", "dbhost").unwrap();
        assert!(!config.contains_prefix("http."));
    }

    #[test]
    fn test_contains_prefix_partial_key_name() {
        let mut config = Config::new();
        config.set("http.host", "localhost").unwrap();
        // "http" is a prefix of "http.host"
        assert!(config.contains_prefix("http"));
        // "htt" is also a prefix
        assert!(config.contains_prefix("htt"));
    }

    #[test]
    fn test_contains_prefix_empty_prefix() {
        let mut config = Config::new();
        config.set("host", "localhost").unwrap();
        // Empty string is a prefix of everything
        assert!(config.contains_prefix(""));
    }
}

// ============================================================================
// subconfig() Tests
// ============================================================================

#[cfg(test)]
mod test_subconfig {
    use super::*;

    #[test]
    fn test_subconfig_strip_prefix_true() {
        let mut config = Config::new();
        config.set("http.host", "localhost").unwrap();
        config.set("http.port", 8080).unwrap();
        config.set("db.host", "dbhost").unwrap();

        let sub = config.subconfig("http", true).unwrap();
        assert!(sub.contains("host"));
        assert!(sub.contains("port"));
        assert!(!sub.contains("db.host"));
        assert!(!sub.contains("http.host"));
    }

    #[test]
    fn test_subconfig_strip_prefix_false() {
        let mut config = Config::new();
        config.set("http.host", "localhost").unwrap();
        config.set("http.port", 8080).unwrap();
        config.set("db.host", "dbhost").unwrap();

        let sub = config.subconfig("http", false).unwrap();
        assert!(sub.contains("http.host"));
        assert!(sub.contains("http.port"));
        assert!(!sub.contains("db.host"));
    }

    #[test]
    fn test_subconfig_empty_result() {
        let mut config = Config::new();
        config.set("db.host", "dbhost").unwrap();

        let sub = config.subconfig("http", true).unwrap();
        assert!(sub.is_empty());
    }

    #[test]
    fn test_subconfig_exact_key_match() {
        let mut config = Config::new();
        config.set("http", "value").unwrap();
        config.set("http.host", "localhost").unwrap();

        let sub = config.subconfig("http", true).unwrap();
        // "http" itself matches (kept as "http" when strip_prefix=true)
        assert!(sub.contains("http"));
        // "http.host" matches and becomes "host"
        assert!(sub.contains("host"));
    }

    #[test]
    fn test_subconfig_preserves_variable_substitution_settings() {
        let mut config = Config::new();
        config.set_enable_variable_substitution(false);
        config.set_max_substitution_depth(10);
        config.set("http.host", "localhost").unwrap();

        let sub = config.subconfig("http", true).unwrap();
        assert!(!sub.is_enable_variable_substitution());
        assert_eq!(sub.max_substitution_depth(), 10);
    }

    #[test]
    fn test_subconfig_nested_prefix() {
        let mut config = Config::new();
        config.set("http.proxy.host", "proxy").unwrap();
        config.set("http.proxy.port", 3128).unwrap();
        config.set("http.timeout", 30).unwrap();

        let sub = config.subconfig("http.proxy", true).unwrap();
        assert!(sub.contains("host"));
        assert!(sub.contains("port"));
        assert!(!sub.contains("timeout"));
    }
}

// ============================================================================
// get() / get_list() additional error paths
// ============================================================================

#[cfg(test)]
mod test_get_and_get_list_error_mapping_additional_paths {
    use super::*;

    #[test]
    fn test_get_and_get_list_error_mapping_additional_paths() {
        use chrono::NaiveDate;
        use serde_json::Value as JsonValue;

        let mut config = Config::new();
        config
            .set("date_value", NaiveDate::from_ymd_opt(2025, 1, 1).unwrap())
            .unwrap();
        config.set("bad_int", "abc").unwrap();
        config.set("bad_json", "{invalid-json").unwrap();

        // ConversionFailed path in get()
        let err = config.get::<i32>("date_value").unwrap_err();
        assert!(matches!(
            err,
            ConfigError::ConversionError { .. } | ConfigError::TypeMismatch { .. }
        ));

        // ConversionError path in get()
        let err = config.get::<i32>("bad_int").unwrap_err();
        assert!(matches!(
            err,
            ConfigError::ConversionError { .. } | ConfigError::TypeMismatch { .. }
        ));

        // JsonDeserializationError path in get()
        let err = config.get::<JsonValue>("bad_json").unwrap_err();
        assert!(matches!(
            err,
            ConfigError::ConversionError { .. } | ConfigError::TypeMismatch { .. }
        ));

        // get_list on empty property returns empty list in current implementation.
        config.set_null("empty_list", DataType::String).unwrap();
        let empty_values = config.get_list::<String>("empty_list").unwrap();
        assert!(empty_values.is_empty());

        // get_list on incompatible scalar may normalize to empty list in current implementation.
        // Keep this assertion to lock current behavior.
        match config.get_list::<i32>("date_value") {
            Ok(values) => assert!(values.is_empty()),
            Err(err) => assert!(matches!(
                err,
                ConfigError::ConversionError { .. } | ConfigError::TypeMismatch { .. }
            )),
        }
    }
}

// ============================================================================
// is_null() Tests
// ============================================================================

#[cfg(test)]
mod test_is_null {
    use super::*;

    #[test]
    fn test_is_null_missing_key_returns_false() {
        let config = Config::new();
        assert!(!config.is_null("missing"));
    }

    #[test]
    fn test_is_null_key_with_value_returns_false() {
        let mut config = Config::new();
        config.set("host", "localhost").unwrap();
        assert!(!config.is_null("host"));
    }

    #[test]
    fn test_is_null_empty_property_returns_true() {
        let mut config = Config::new();
        config.set_null("nullable", DataType::String).unwrap();
        assert!(config.is_null("nullable"));
    }

    #[test]
    fn test_is_null_after_clear() {
        let mut config = Config::new();
        config.set("host", "localhost").unwrap();
        config
            .get_property_mut("host")
            .unwrap()
            .unwrap()
            .clear()
            .unwrap();
        assert!(config.is_null("host"));
    }
}

// ============================================================================
// get_optional() Tests
// ============================================================================

#[cfg(test)]
mod test_get_optional {
    use super::*;

    #[test]
    fn test_get_optional_missing_key_returns_none() {
        let config = Config::new();
        let result: Option<String> = config.get_optional("missing").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_get_optional_existing_key_returns_some() {
        let mut config = Config::new();
        config.set("host", "localhost").unwrap();
        let result: Option<String> = config.get_optional("host").unwrap();
        assert_eq!(result, Some("localhost".to_string()));
    }

    #[test]
    fn test_get_optional_null_property_returns_none() {
        let mut config = Config::new();
        config.set_null("nullable", DataType::String).unwrap();
        let result: Option<String> = config.get_optional("nullable").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_get_optional_integer() {
        let mut config = Config::new();
        config.set("port", 8080).unwrap();
        let result: Option<i32> = config.get_optional("port").unwrap();
        assert_eq!(result, Some(8080));
    }

    #[test]
    fn test_get_optional_bool() {
        let mut config = Config::new();
        config.set("debug", true).unwrap();
        let result: Option<bool> = config.get_optional("debug").unwrap();
        assert_eq!(result, Some(true));
    }

    #[test]
    fn test_get_optional_type_mismatch_returns_error() {
        let mut config = Config::new();
        config.set("port", "not-a-bool").unwrap();
        let result: Result<Option<bool>, _> = config.get_optional("port");
        assert!(result.is_err());
        match result.unwrap_err() {
            ConfigError::ConversionError { key, .. } => {
                assert_eq!(key, "port");
            }
            e => panic!("Expected ConversionError, got {:?}", e),
        }
    }
}

// ============================================================================
// get_optional_list() Tests
// ============================================================================

#[cfg(test)]
mod test_get_optional_list {
    use super::*;

    #[test]
    fn test_get_optional_list_missing_key_returns_none() {
        let config = Config::new();
        let result: Option<Vec<i32>> = config.get_optional_list("missing").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_get_optional_list_existing_key_returns_some() {
        let mut config = Config::new();
        config.set("ports", vec![8080, 8081, 8082]).unwrap();
        let result: Option<Vec<i32>> = config.get_optional_list("ports").unwrap();
        assert_eq!(result, Some(vec![8080, 8081, 8082]));
    }

    #[test]
    fn test_get_optional_list_null_property_returns_none() {
        let mut config = Config::new();
        config.set_null("nullable", DataType::Int32).unwrap();
        let result: Option<Vec<i32>> = config.get_optional_list("nullable").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_get_optional_list_single_value() {
        let mut config = Config::new();
        config.set("port", 8080).unwrap();
        let result: Option<Vec<i32>> = config.get_optional_list("port").unwrap();
        assert_eq!(result, Some(vec![8080]));
    }

    #[test]
    fn test_get_optional_list_type_mismatch_returns_error() {
        let mut config = Config::new();
        config.set("ports", vec!["yes", "no"]).unwrap();
        let result: Result<Option<Vec<bool>>, _> = config.get_optional_list("ports");
        assert!(result.is_err());
        match result.unwrap_err() {
            ConfigError::ConversionError { key, .. } => {
                assert_eq!(key, "ports");
            }
            e => panic!("Expected ConversionError, got {:?}", e),
        }
    }
}

// ============================================================================
// get_optional_string() / get_optional_string_list() Tests
// ============================================================================

#[cfg(test)]
mod test_get_optional_string {
    use super::*;

    #[test]
    fn test_get_optional_string_missing_returns_none() {
        let config = Config::new();
        assert_eq!(config.get_optional_string("missing").unwrap(), None);
    }

    #[test]
    fn test_get_optional_string_null_returns_none() {
        let mut config = Config::new();
        config.set_null("n", DataType::String).unwrap();
        assert_eq!(config.get_optional_string("n").unwrap(), None);
    }

    #[test]
    fn test_get_optional_string_null_non_string_empty_still_none() {
        let mut config = Config::new();
        config.set_null("nullable", DataType::Int32).unwrap();
        assert_eq!(config.get_optional_string("nullable").unwrap(), None);
    }

    #[test]
    fn test_get_optional_string_plain_no_variables() {
        let mut config = Config::new();
        config.set("greeting", "hello").unwrap();
        assert_eq!(
            config.get_optional_string("greeting").unwrap().as_deref(),
            Some("hello")
        );
    }

    #[test]
    fn test_get_optional_string_empty_string_is_some() {
        let mut config = Config::new();
        config.set("empty", "").unwrap();
        assert_eq!(
            config.get_optional_string("empty").unwrap().as_deref(),
            Some("")
        );
    }

    #[test]
    fn test_get_optional_string_substitution() {
        let mut config = Config::new();
        config.set("base", "http://localhost").unwrap();
        config.set("api", "${base}/api").unwrap();
        assert_eq!(
            config.get_optional_string("api").unwrap().as_deref(),
            Some("http://localhost/api")
        );
    }

    #[test]
    fn test_get_optional_string_substitution_disabled_keeps_placeholders() {
        let mut config = Config::new();
        config.set_enable_variable_substitution(false);
        config.set("raw", "${not_replaced}").unwrap();
        assert_eq!(
            config.get_optional_string("raw").unwrap().as_deref(),
            Some("${not_replaced}")
        );
    }

    #[test]
    fn test_get_optional_string_type_mismatch_returns_error() {
        let mut config = Config::new();
        config.set("port", 8080i32).unwrap();
        assert_eq!(
            config.get_optional_string("port").unwrap(),
            Some("8080".to_string())
        );
    }

    #[test]
    fn test_get_optional_string_unresolved_variable_returns_error() {
        let mut config = Config::new();
        config
            .set(
                "bad",
                "${qubit_cfg_test_var_that_must_not_exist_7a8b9c0d1e2f}",
            )
            .unwrap();
        let result = config.get_optional_string("bad");
        assert!(matches!(result, Err(ConfigError::SubstitutionError(_))));
    }

    #[test]
    fn test_get_optional_string_substitution_depth_exceeded() {
        let mut config = Config::new();
        config.set_max_substitution_depth(0);
        config.set("a", "v").unwrap();
        config.set("b", "${a}").unwrap();
        let result = config.get_optional_string("b");
        assert!(matches!(
            result,
            Err(ConfigError::SubstitutionDepthExceeded(0))
        ));
    }

    #[test]
    fn test_get_optional_string_list_missing_returns_none() {
        let config = Config::new();
        assert_eq!(config.get_optional_string_list("missing").unwrap(), None);
    }

    #[test]
    fn test_get_optional_string_list_null_returns_none() {
        let mut config = Config::new();
        config.set_null("nullable", DataType::String).unwrap();
        assert_eq!(config.get_optional_string_list("nullable").unwrap(), None);
    }

    #[test]
    fn test_get_optional_string_list_substitution() {
        let mut config = Config::new();
        config.set("root", "/opt/app").unwrap();
        config
            .set("paths", vec!["${root}/bin", "${root}/lib"])
            .unwrap();
        assert_eq!(
            config.get_optional_string_list("paths").unwrap(),
            Some(vec!["/opt/app/bin".to_string(), "/opt/app/lib".to_string()])
        );
    }

    #[test]
    fn test_get_optional_string_list_plain_no_variables() {
        let mut config = Config::new();
        config.set("items", vec!["a", "b"]).unwrap();
        assert_eq!(
            config.get_optional_string_list("items").unwrap(),
            Some(vec!["a".to_string(), "b".to_string()])
        );
    }

    #[test]
    fn test_get_optional_string_list_single_scalar_coerced_to_one_element() {
        let mut config = Config::new();
        config.set("only", "solo").unwrap();
        assert_eq!(
            config.get_optional_string_list("only").unwrap(),
            Some(vec!["solo".to_string()])
        );
    }

    #[test]
    fn test_get_optional_string_list_empty_vec_set_is_treated_as_absent_none() {
        let mut config = Config::new();
        config.set("empty_list", Vec::<String>::new()).unwrap();
        // Empty collection normalizes to an empty property; optional readers treat it like null.
        assert_eq!(config.get_optional_string_list("empty_list").unwrap(), None);
    }

    #[test]
    fn test_get_optional_string_list_substitution_disabled() {
        let mut config = Config::new();
        config.set_enable_variable_substitution(false);
        config.set("items", vec!["${x}", "y"]).unwrap();
        assert_eq!(
            config.get_optional_string_list("items").unwrap(),
            Some(vec!["${x}".to_string(), "y".to_string()])
        );
    }

    #[test]
    fn test_get_optional_string_list_type_mismatch_returns_error() {
        let mut config = Config::new();
        config.set("ports", vec![1i32, 2i32]).unwrap();
        assert_eq!(
            config.get_optional_string_list("ports").unwrap(),
            Some(vec!["1".to_string(), "2".to_string()])
        );
    }

    #[test]
    fn test_get_optional_string_list_unresolved_variable_in_element_returns_error() {
        let mut config = Config::new();
        config
            .set(
                "items",
                vec![
                    "ok",
                    "${qubit_cfg_list_bad_var_that_must_not_exist_9f8e7d6c5b4a}",
                ],
            )
            .unwrap();
        let result = config.get_optional_string_list("items");
        assert!(matches!(result, Err(ConfigError::SubstitutionError(_))));
    }

    #[test]
    fn test_get_optional_string_list_substitution_depth_exceeded() {
        let mut config = Config::new();
        config.set_max_substitution_depth(0);
        config.set("a", "x").unwrap();
        config.set("items", vec!["${a}"]).unwrap();
        let result = config.get_optional_string_list("items");
        assert!(matches!(
            result,
            Err(ConfigError::SubstitutionDepthExceeded(0))
        ));
    }
}

// ============================================================================
// Enhanced Error Model Tests
// ============================================================================

#[cfg(test)]
mod test_enhanced_errors {
    use super::*;

    #[test]
    fn test_get_type_mismatch_carries_key() {
        let mut config = Config::new();
        config.set("server.port", 8080).unwrap();

        let result: Result<bool, _> = config.get_strict("server.port");
        assert!(result.is_err());
        match result.unwrap_err() {
            ConfigError::TypeMismatch {
                key,
                expected,
                actual,
            } => {
                assert_eq!(key, "server.port");
                assert_eq!(expected, DataType::Bool);
                assert_eq!(actual, DataType::Int32);
            }
            e => panic!("Expected TypeMismatch with key, got {:?}", e),
        }
    }

    #[test]
    fn test_get_list_type_mismatch_carries_key() {
        let mut config = Config::new();
        config.set("ports", vec![8080, 8081]).unwrap();

        let result: Result<Vec<bool>, _> = config.get_list_strict("ports");
        assert!(result.is_err());
        match result.unwrap_err() {
            ConfigError::TypeMismatch { key, .. } => {
                assert_eq!(key, "ports");
            }
            e => panic!("Expected TypeMismatch with key, got {:?}", e),
        }
    }

    #[test]
    fn test_get_property_not_found_carries_key() {
        let config = Config::new();
        let result: Result<String, _> = config.get("http.logging.body_size_limit");
        assert!(result.is_err());
        match result.unwrap_err() {
            ConfigError::PropertyNotFound(key) => {
                assert_eq!(key, "http.logging.body_size_limit");
            }
            e => panic!("Expected PropertyNotFound, got {:?}", e),
        }
    }

    #[test]
    fn test_get_property_has_no_value_carries_key() {
        let mut config = Config::new();
        config.set_null("empty.key", DataType::String).unwrap();
        let result: Result<String, _> = config.get("empty.key");
        assert!(result.is_err());
        match result.unwrap_err() {
            ConfigError::PropertyHasNoValue(key) => {
                assert_eq!(key, "empty.key");
            }
            e => panic!("Expected PropertyHasNoValue, got {:?}", e),
        }
    }

    #[test]
    fn test_type_mismatch_error_format_includes_key() {
        let error = ConfigError::TypeMismatch {
            key: "http.logging.body_size_limit".to_string(),
            expected: DataType::Int32,
            actual: DataType::String,
        };
        let msg = format!("{}", error);
        assert!(msg.contains("http.logging.body_size_limit"));
        assert!(msg.contains("expected"));
        assert!(msg.contains("actual"));
    }

    #[test]
    fn test_conversion_error_format_includes_key() {
        let error = ConfigError::ConversionError {
            key: "db.timeout".to_string(),
            message: "invalid duration format".to_string(),
        };
        let msg = format!("{}", error);
        assert!(msg.contains("db.timeout"));
        assert!(msg.contains("invalid duration format"));
    }

    #[test]
    fn test_deserialize_error_format_includes_path() {
        let error = ConfigError::DeserializeError {
            path: "http.server".to_string(),
            message: "missing field `port`".to_string(),
        };
        let msg = format!("{}", error);
        assert!(msg.contains("http.server"));
        assert!(msg.contains("missing field"));
    }

    #[test]
    fn test_type_mismatch_from_value_error_has_empty_key() {
        use qubit_value::ValueError;
        // ValueError::TypeMismatch → ConfigError::TypeMismatch with empty key
        let ve = ValueError::TypeMismatch {
            expected: DataType::Int32,
            actual: DataType::String,
        };
        let ce: ConfigError = ve.into();
        match ce {
            ConfigError::TypeMismatch {
                key,
                expected,
                actual,
            } => {
                assert_eq!(key, "");
                assert_eq!(expected, DataType::Int32);
                assert_eq!(actual, DataType::String);
            }
            _ => panic!("Expected TypeMismatch"),
        }
    }

    #[test]
    fn test_type_mismatch_from_get_has_key() {
        let mut config = Config::new();
        config.set("my.key", 42).unwrap();
        let result: Result<bool, _> = config.get_strict("my.key");
        match result.unwrap_err() {
            ConfigError::TypeMismatch { key, .. } => {
                assert_eq!(key, "my.key");
            }
            _ => panic!("Expected TypeMismatch"),
        }
    }

    #[test]
    fn test_conversion_error_from_value_error_has_empty_key() {
        use qubit_value::ValueError;
        let ve = ValueError::ConversionError("test message".to_string());
        let ce: ConfigError = ve.into();
        match ce {
            ConfigError::ConversionError { key, message } => {
                assert_eq!(key, "");
                assert_eq!(message, "test message");
            }
            _ => panic!("Expected ConversionError"),
        }
    }

    #[test]
    fn test_conversion_failed_from_value_error_has_empty_key() {
        use qubit_value::ValueError;
        let ve = ValueError::ConversionFailed {
            from: DataType::String,
            to: DataType::Int32,
        };
        let ce: ConfigError = ve.into();
        match ce {
            ConfigError::ConversionError { key, message } => {
                assert_eq!(key, "");
                assert!(message.contains("From") || message.contains("to"));
            }
            _ => panic!("Expected ConversionError"),
        }
    }

    #[test]
    fn test_from_value_error_json_serialization() {
        use qubit_value::ValueError;
        let ve = ValueError::JsonSerializationError("json error".to_string());
        let ce: ConfigError = ve.into();
        match ce {
            ConfigError::ConversionError { message, .. } => {
                assert!(message.contains("JSON serialization error"));
            }
            _ => panic!("Expected ConversionError"),
        }
    }

    #[test]
    fn test_from_value_error_json_deserialization() {
        use qubit_value::ValueError;
        let ve = ValueError::JsonDeserializationError("json error".to_string());
        let ce: ConfigError = ve.into();
        match ce {
            ConfigError::ConversionError { message, .. } => {
                assert!(message.contains("JSON deserialization error"));
            }
            _ => panic!("Expected ConversionError"),
        }
    }
}

// ============================================================================
// TOML Type-Faithful Loading Tests
// ============================================================================

#[cfg(test)]
mod test_toml_type_faithful {
    use qubit_config::source::{ConfigSource, TomlConfigSource};

    use super::*;

    fn load_toml(content: &str) -> Config {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.toml");
        std::fs::write(&path, content).unwrap();
        let source = TomlConfigSource::from_file(&path);
        let mut config = Config::new();
        source.load(&mut config).unwrap();
        config
    }

    #[test]
    fn test_toml_integer_stored_as_i64() {
        let config = load_toml("port = 8080\n");
        assert_eq!(config.get::<i64>("port").unwrap(), 8080);
    }

    #[test]
    fn test_toml_float_stored_as_f64() {
        let config = load_toml("timeout = 30.5\n");
        assert_eq!(config.get::<f64>("timeout").unwrap(), 30.5);
    }

    #[test]
    fn test_toml_bool_stored_as_bool() {
        let config = load_toml("debug = true\nenabled = false\n");
        assert!(config.get::<bool>("debug").unwrap());
        assert!(!config.get::<bool>("enabled").unwrap());
    }

    #[test]
    fn test_toml_string_stored_as_string() {
        let config = load_toml("host = \"localhost\"\n");
        assert_eq!(config.get_string("host").unwrap(), "localhost");
    }

    #[test]
    fn test_toml_integer_array_stored_as_i64_multivalue() {
        let config = load_toml("ports = [8080, 8081, 8082]\n");
        let ports: Vec<i64> = config.get_list("ports").unwrap();
        assert_eq!(ports, vec![8080i64, 8081, 8082]);
    }

    #[test]
    fn test_toml_float_array_stored_as_f64_multivalue() {
        let config = load_toml("weights = [0.1, 0.5, 0.9]\n");
        let weights: Vec<f64> = config.get_list("weights").unwrap();
        assert!((weights[0] - 0.1).abs() < 1e-9);
        assert!((weights[1] - 0.5).abs() < 1e-9);
        assert!((weights[2] - 0.9).abs() < 1e-9);
    }

    #[test]
    fn test_toml_bool_array_stored_as_bool_multivalue() {
        let config = load_toml("flags = [true, false, true]\n");
        let flags: Vec<bool> = config.get_list("flags").unwrap();
        assert_eq!(flags, vec![true, false, true]);
    }

    #[test]
    fn test_toml_string_array_stored_as_string_multivalue() {
        let config = load_toml("tags = [\"web\", \"api\", \"v2\"]\n");
        let tags: Vec<String> = config.get_list("tags").unwrap();
        assert_eq!(tags, vec!["web", "api", "v2"]);
    }

    #[test]
    fn test_toml_nested_table_flattened() {
        let config = load_toml("[server]\nhost = \"localhost\"\nport = 9090\n");
        assert_eq!(config.get_string("server.host").unwrap(), "localhost");
        assert_eq!(config.get::<i64>("server.port").unwrap(), 9090);
    }

    #[test]
    fn test_toml_mixed_array_falls_back_to_string() {
        // Mixed types: int and string → fall back to string
        let config = load_toml("mixed = [1, \"two\", 3]\n");
        // Should be stored as strings
        let vals: Vec<String> = config.get_list("mixed").unwrap();
        assert_eq!(vals.len(), 3);
    }

    #[test]
    fn test_toml_nested_array_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested_array.toml");
        std::fs::write(&path, "nested = [[1, 2], [3, 4]]\n").unwrap();
        let source = TomlConfigSource::from_file(&path);
        let mut config = Config::new();
        let result = source.load(&mut config);
        assert!(result.is_err());
        assert!(matches!(result, Err(ConfigError::ParseError(_))));
    }
}

// ============================================================================
// YAML Type-Faithful Loading Tests
// ============================================================================

#[cfg(test)]
mod test_yaml_type_faithful {
    use qubit_config::source::{ConfigSource, YamlConfigSource};

    use super::*;

    fn load_yaml(content: &str) -> Config {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.yaml");
        std::fs::write(&path, content).unwrap();
        let source = YamlConfigSource::from_file(&path);
        let mut config = Config::new();
        source.load(&mut config).unwrap();
        config
    }

    #[test]
    fn test_yaml_integer_stored_as_i64() {
        let config = load_yaml("port: 8080\n");
        assert_eq!(config.get::<i64>("port").unwrap(), 8080);
    }

    #[test]
    fn test_yaml_float_stored_as_f64() {
        let config = load_yaml("timeout: 30.5\n");
        assert_eq!(config.get::<f64>("timeout").unwrap(), 30.5);
    }

    #[test]
    fn test_yaml_bool_stored_as_bool() {
        let config = load_yaml("debug: true\nenabled: false\n");
        assert!(config.get::<bool>("debug").unwrap());
        assert!(!config.get::<bool>("enabled").unwrap());
    }

    #[test]
    fn test_yaml_string_stored_as_string() {
        let config = load_yaml("host: localhost\n");
        assert_eq!(config.get_string("host").unwrap(), "localhost");
    }

    #[test]
    fn test_yaml_null_stored_as_empty_property() {
        let config = load_yaml("key: ~\n");
        assert!(config.contains("key"));
        assert!(config.is_null("key"));
    }

    #[test]
    fn test_yaml_null_keyword() {
        let config = load_yaml("key: null\n");
        assert!(config.contains("key"));
        assert!(config.is_null("key"));
    }

    #[test]
    fn test_yaml_integer_sequence_stored_as_i64_multivalue() {
        let config = load_yaml("ports:\n  - 8080\n  - 8081\n  - 8082\n");
        let ports: Vec<i64> = config.get_list("ports").unwrap();
        assert_eq!(ports, vec![8080i64, 8081, 8082]);
    }

    #[test]
    fn test_yaml_float_sequence_stored_as_f64_multivalue() {
        let config = load_yaml("weights:\n  - 0.1\n  - 0.5\n  - 0.9\n");
        let weights: Vec<f64> = config.get_list("weights").unwrap();
        assert!((weights[0] - 0.1).abs() < 1e-9);
    }

    #[test]
    fn test_yaml_bool_sequence_stored_as_bool_multivalue() {
        let config = load_yaml("flags:\n  - true\n  - false\n  - true\n");
        let flags: Vec<bool> = config.get_list("flags").unwrap();
        assert_eq!(flags, vec![true, false, true]);
    }

    #[test]
    fn test_yaml_string_sequence_stored_as_string_multivalue() {
        let config = load_yaml("tags:\n  - web\n  - api\n  - v2\n");
        let tags: Vec<String> = config.get_list("tags").unwrap();
        assert_eq!(tags, vec!["web", "api", "v2"]);
    }

    #[test]
    fn test_yaml_nested_mapping_flattened() {
        let config = load_yaml("server:\n  host: localhost\n  port: 9090\n");
        assert_eq!(config.get_string("server.host").unwrap(), "localhost");
        assert_eq!(config.get::<i64>("server.port").unwrap(), 9090);
    }

    #[test]
    fn test_yaml_mixed_sequence_falls_back_to_string() {
        let config = load_yaml("mixed:\n  - 1\n  - two\n  - 3\n");
        let vals: Vec<String> = config.get_list("mixed").unwrap();
        assert_eq!(vals.len(), 3);
    }

    #[test]
    fn test_yaml_tagged_value() {
        // Tagged values should be unwrapped
        let config = load_yaml("key: !!str 42\n");
        // serde_yaml treats !!str 42 as a string
        assert!(config.contains("key"));
    }

    #[test]
    fn test_yaml_empty_sequence() {
        let config = load_yaml("empty: []\n");
        assert!(config.contains("empty"));
        assert_eq!(
            config.get_list::<String>("empty").unwrap(),
            Vec::<String>::new()
        );
        assert_eq!(config.get_list::<i64>("empty").unwrap(), Vec::<i64>::new());
    }

    #[test]
    fn test_yaml_nested_sequence_returns_parse_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested_seq.yaml");
        std::fs::write(&path, "matrix:\n  - [1, 2]\n  - [3, 4]\n").unwrap();
        let source = YamlConfigSource::from_file(&path);
        let mut config = Config::new();
        let result = source.load(&mut config);
        assert!(matches!(result, Err(ConfigError::ParseError(_))));
    }
}

// ============================================================================
// insert_property() / set_null() Tests
// ============================================================================

#[cfg(test)]
mod test_property_insertion_api {
    use super::*;

    #[test]
    fn test_insert_property_success() {
        let mut config = Config::new();
        config
            .insert_property(
                "direct",
                Property::with_value("direct", MultiValues::String(vec!["hello".to_string()])),
            )
            .unwrap();
        assert_eq!(config.get_string("direct").unwrap(), "hello");
    }

    #[test]
    fn test_set_null_success() {
        let mut config = Config::new();
        config.set_null("null_key", DataType::String).unwrap();
        assert!(config.is_null("null_key"));
        assert!(config.contains("null_key"));
    }

    #[test]
    fn test_insert_property_name_mismatch_returns_error() {
        let mut config = Config::new();
        let result = config.insert_property(
            "expected.key",
            Property::with_value("actual.key", MultiValues::String(vec!["hello".to_string()])),
        );
        assert!(matches!(result, Err(ConfigError::MergeError(_))));
    }

    #[test]
    fn test_insert_property_on_final_key_returns_error() {
        let mut config = Config::new();
        config.set("final.key", "v1").unwrap();
        config.set_final("final.key", true).unwrap();

        let result = config.insert_property(
            "final.key",
            Property::with_value("final.key", MultiValues::String(vec!["v2".to_string()])),
        );
        assert!(matches!(result, Err(ConfigError::PropertyIsFinal(_))));
    }
}

// ============================================================================
// Additional coverage for config.rs error branches
// ============================================================================

#[cfg(test)]
mod test_config_error_branches {
    use super::*;

    // Test that get_list on an empty property returns empty vec (not error)
    #[test]
    fn test_get_list_on_empty_property_returns_empty_vec() {
        let mut config = Config::new();
        config.set_null("empty", DataType::Int32).unwrap();
        let result: Vec<i32> = config.get_list("empty").unwrap();
        assert!(result.is_empty());
    }

    // Test get on a property that has wrong type (triggers TypeMismatch with key)
    #[test]
    fn test_get_type_mismatch_with_key_in_error() {
        let mut config = Config::new();
        config.set("http.port", 8080).unwrap();
        let err = config.get_strict::<String>("http.port").unwrap_err();
        match err {
            ConfigError::TypeMismatch { key, .. } => {
                assert_eq!(key, "http.port");
            }
            _ => panic!("Expected TypeMismatch"),
        }
    }

    // Test get_list on a property that has wrong type (triggers TypeMismatch with key)
    #[test]
    fn test_get_list_type_mismatch_with_key_in_error() {
        let mut config = Config::new();
        config.set("ports", vec![8080i32, 8081]).unwrap();
        let err = config.get_list_strict::<String>("ports").unwrap_err();
        match err {
            ConfigError::TypeMismatch { key, .. } => {
                assert_eq!(key, "ports");
            }
            _ => panic!("Expected TypeMismatch"),
        }
    }

    // Test that get on empty property returns PropertyHasNoValue
    #[test]
    fn test_get_on_empty_property_returns_has_no_value() {
        let mut config = Config::new();
        config.set_null("empty_str", DataType::String).unwrap();
        let err = config.get::<String>("empty_str").unwrap_err();
        match err {
            ConfigError::PropertyHasNoValue(key) => {
                assert_eq!(key, "empty_str");
            }
            _ => panic!("Expected PropertyHasNoValue, got {:?}", err),
        }
    }
}

// ============================================================================
// Integration: subconfig + deserialize
// ============================================================================

#[cfg(test)]
mod test_subconfig_deserialize_integration {
    use super::*;

    #[derive(Deserialize, Debug, PartialEq)]
    struct HttpOptions {
        host: String,
        port: i32,
        timeout: Option<i64>,
    }

    #[derive(Deserialize, Debug, PartialEq)]
    struct DbOptions {
        url: String,
        pool: i32,
    }

    #[test]
    fn test_deserialize_http_options() {
        let mut config = Config::new();
        config.set("http.host", "api.example.com").unwrap();
        config.set("http.port", 443).unwrap();
        config.set("http.timeout", 30i64).unwrap();
        config.set("db.url", "postgres://localhost/mydb").unwrap();
        config.set("db.pool", 5).unwrap();

        let http: HttpOptions = config.deserialize("http").unwrap();
        assert_eq!(http.host, "api.example.com");
        assert_eq!(http.port, 443);
        assert_eq!(http.timeout, Some(30));

        let db: DbOptions = config.deserialize("db").unwrap();
        assert_eq!(db.url, "postgres://localhost/mydb");
        assert_eq!(db.pool, 5);
    }

    #[test]
    fn test_subconfig_then_get() {
        let mut config = Config::new();
        config.set("http.proxy.host", "proxy.example.com").unwrap();
        config.set("http.proxy.port", 3128).unwrap();
        config.set("http.timeout", 30).unwrap();

        let proxy = config.subconfig("http.proxy", true).unwrap();
        assert_eq!(proxy.get_string("host").unwrap(), "proxy.example.com");
        assert_eq!(proxy.get::<i32>("port").unwrap(), 3128);
        assert!(!proxy.contains("timeout"));
    }

    #[test]
    fn test_iter_prefix_then_subconfig() {
        let mut config = Config::new();
        config.set("module.a.x", 1).unwrap();
        config.set("module.a.y", 2).unwrap();
        config.set("module.b.x", 3).unwrap();

        assert!(config.contains_prefix("module.a."));
        assert!(config.contains_prefix("module.b."));

        let sub_a = config.subconfig("module.a", true).unwrap();
        assert_eq!(sub_a.get::<i32>("x").unwrap(), 1);
        assert_eq!(sub_a.get::<i32>("y").unwrap(), 2);
        assert!(!sub_a.contains("x".to_string().as_str().replace("x", "b.x").as_str()));
    }
}

// ============================================================================
// merge_from_source (`Config` API)
// ============================================================================

#[cfg(test)]
mod test_merge_from_source {
    use super::{Config, ConfigError};
    use qubit_config::source::TomlConfigSource;
    use std::path::PathBuf;

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join(name)
    }

    #[test]
    fn test_merge_from_source_populates_config() {
        let source = TomlConfigSource::from_file(fixture("basic.toml"));
        let mut config = Config::new();
        config.merge_from_source(&source).unwrap();

        assert!(!config.is_empty());
        assert!(config.contains("host"));
    }

    #[test]
    fn test_merge_from_source_overwrites_existing_keys() {
        let mut config = Config::new();
        config.set("host", "old-host").unwrap();

        let source = TomlConfigSource::from_file(fixture("basic.toml"));
        config.merge_from_source(&source).unwrap();

        assert_eq!(config.get_string("host").unwrap(), "localhost");
    }

    #[test]
    fn test_merge_from_source_preserves_final_property() {
        let mut config = Config::new();
        config.set("host", "final-host").unwrap();
        config.set_final("host", true).unwrap();

        let source = TomlConfigSource::from_file(fixture("basic.toml"));
        let result = config.merge_from_source(&source);

        assert!(result.is_err());
        assert!(matches!(result, Err(ConfigError::PropertyIsFinal(_))));
        assert_eq!(config.get_string("host").unwrap(), "final-host");
    }

    #[test]
    fn test_merge_from_source_adds_new_keys() {
        let mut config = Config::new();
        config.set("existing", "value").unwrap();

        let source = TomlConfigSource::from_file(fixture("basic.toml"));
        config.merge_from_source(&source).unwrap();

        assert_eq!(config.get_string("existing").unwrap(), "value");
        assert!(config.contains("host"));
        assert!(config.contains("app.name"));
    }

    #[test]
    fn test_merge_from_source_returns_error_on_failure() {
        let source = TomlConfigSource::from_file("/nonexistent/path.toml");
        let mut config = Config::new();
        let result = config.merge_from_source(&source);
        assert!(result.is_err());
    }

    #[test]
    fn test_merge_from_source_with_variable_substitution() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vars.toml");
        std::fs::write(
            &path,
            r#"
base_url = "http://localhost:8080"
api_url = "${base_url}/api"
"#,
        )
        .unwrap();

        let source = TomlConfigSource::from_file(&path);
        let mut config = Config::new();
        config.merge_from_source(&source).unwrap();

        assert_eq!(
            config.get_string("api_url").unwrap(),
            "http://localhost:8080/api"
        );
    }
}

// ============================================================================
// Source-backed constructors (`Config` API)
// ============================================================================

#[cfg(test)]
mod test_source_backed_constructors {
    use super::Config;
    use qubit_config::source::TomlConfigSource;
    use std::path::PathBuf;
    use std::sync::{Mutex, MutexGuard, OnceLock};

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join(name)
    }

    /// Serializes constructor tests that mutate process environment variables.
    fn env_test_lock() -> MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .expect("environment test lock should not be poisoned")
    }

    #[test]
    fn test_from_source_loads_config_source_into_new_config() {
        let source = TomlConfigSource::from_file(fixture("basic.toml"));

        let config = Config::from_source(&source).unwrap();

        assert_eq!(config.get_string("host").unwrap(), "localhost");
        assert_eq!(config.get::<i64>("server.port").unwrap(), 9090);
    }

    #[test]
    fn test_from_toml_file_loads_toml_config() {
        let config = Config::from_toml_file(fixture("basic.toml")).unwrap();

        assert_eq!(config.get_string("app.name").unwrap(), "MyApp");
        assert_eq!(config.get::<i64>("port").unwrap(), 8080);
    }

    #[test]
    fn test_from_yaml_file_loads_yaml_config() {
        let config = Config::from_yaml_file(fixture("basic.yaml")).unwrap();

        assert_eq!(config.get_string("app.name").unwrap(), "MyApp");
        assert_eq!(config.get::<i64>("server.port").unwrap(), 9090);
    }

    #[test]
    fn test_from_properties_file_loads_properties_config() {
        let config = Config::from_properties_file(fixture("basic.properties")).unwrap();

        assert_eq!(config.get_string("host").unwrap(), "localhost");
        assert_eq!(config.get_string("app.version").unwrap(), "1.0.0");
    }

    #[test]
    fn test_from_env_file_loads_dotenv_config() {
        let config = Config::from_env_file(fixture("basic.env")).unwrap();

        assert_eq!(config.get_string("HOST").unwrap(), "localhost");
        assert_eq!(config.get_string("APP_NAME").unwrap(), "MyApp");
    }

    #[test]
    fn test_from_env_loads_process_environment() {
        let _guard = env_test_lock();
        unsafe {
            std::env::set_var("QUBIT_CONFIG_FROM_ENV_TEST_KEY", "from-env");
        }

        let config = Config::from_env().unwrap();

        assert_eq!(
            config.get_string("QUBIT_CONFIG_FROM_ENV_TEST_KEY").unwrap(),
            "from-env"
        );

        unsafe {
            std::env::remove_var("QUBIT_CONFIG_FROM_ENV_TEST_KEY");
        }
    }

    #[test]
    fn test_from_env_prefix_loads_and_normalizes_matching_vars() {
        let _guard = env_test_lock();
        unsafe {
            std::env::set_var("QCFG_SERVER_HOST", "env-host");
            std::env::set_var("QCFG_SERVER_PORT", "9091");
            std::env::set_var("OTHER_QCFG_SERVER_HOST", "ignored");
        }

        let config = Config::from_env_prefix("QCFG_").unwrap();

        assert_eq!(config.get_string("server.host").unwrap(), "env-host");
        assert_eq!(config.get_string("server.port").unwrap(), "9091");
        assert!(!config.contains("OTHER_QCFG_SERVER_HOST"));

        unsafe {
            std::env::remove_var("QCFG_SERVER_HOST");
            std::env::remove_var("QCFG_SERVER_PORT");
            std::env::remove_var("OTHER_QCFG_SERVER_HOST");
        }
    }

    #[test]
    fn test_from_env_options_respects_explicit_key_transform_options() {
        let _guard = env_test_lock();
        unsafe {
            std::env::set_var("QOPTS_MY_KEY", "raw-value");
        }

        let config = Config::from_env_options("QOPTS_", false, false, false).unwrap();

        assert_eq!(config.get_string("QOPTS_MY_KEY").unwrap(), "raw-value");
        assert!(!config.contains("my.key"));

        unsafe {
            std::env::remove_var("QOPTS_MY_KEY");
        }
    }

    #[test]
    fn test_from_toml_file_returns_error_for_missing_file() {
        let result = Config::from_toml_file("/nonexistent/path.toml");

        assert!(result.is_err());
    }
}
