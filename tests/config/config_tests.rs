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
pub(crate) use chrono::DateTime;
#[cfg(feature = "chrono")]
pub(crate) use chrono::NaiveDate;
#[cfg(feature = "chrono")]
pub(crate) use chrono::NaiveDateTime;
#[cfg(feature = "chrono")]
pub(crate) use chrono::NaiveTime;
#[cfg(feature = "chrono")]
pub(crate) use chrono::Utc;
pub(crate) use qubit_config::Config;
pub(crate) use qubit_config::ConfigError;
pub(crate) use qubit_config::ConfigReader;
pub(crate) use qubit_config::Property;
pub(crate) use qubit_config::options::InterpolationSources;
pub(crate) use qubit_config::options::ReadPolicy;
pub(crate) use qubit_datatype::BlankStringPolicy;
pub(crate) use qubit_datatype::DataConversionErrorKind;
pub(crate) use qubit_datatype::DataType;
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
    Config::builder().description("Test Configuration").build()
}

/// Changes the interpolation recursion limit while preserving other options.
pub(crate) fn set_max_interpolation_depth(config: &mut Config, max_depth: usize) {
    let options = ReadPolicy::builder_from(config.default_read_policy())
        .max_interpolation_depth(max_depth)
        .build();
    config.set_default_read_policy(options);
}

#[test]
fn test_default_read_policy_is_transient_and_preserved_by_mutations() {
    let policy = ReadPolicy::env_friendly();
    let mut config = Config::builder()
        .default_read_policy(policy.clone())
        .build();
    config
        .set("value", "1")
        .expect("setting the value should succeed");
    config
        .set("other", "2")
        .expect("setting the second value should succeed");

    let clone = config.clone();
    assert_eq!(clone.default_read_policy(), &policy);
    assert_eq!(config, clone);

    config
        .remove("other")
        .expect("removing the value should succeed");
    config.clear().expect("clearing the config should succeed");
    assert_eq!(config.default_read_policy(), &policy);
}

#[test]
fn test_config_equality_ignores_transient_read_policy() {
    let mut default_policy = Config::new();
    default_policy.set("value", 1_i32).expect("set value");

    let mut env_policy = default_policy.clone();
    env_policy.set_default_read_policy(ReadPolicy::env_friendly());

    assert_eq!(default_policy, env_policy);
}

#[test]
fn test_read_with_is_non_mutating_and_overrides_only_the_view() {
    let mut config = Config::builder()
        .default_read_policy(ReadPolicy::env_friendly())
        .build();
    config
        .set("ports", "8080,,8081")
        .expect("setting the list should succeed");
    let default_policy = config.default_read_policy().clone();
    let strict_policy = ReadPolicy::default();

    let strict_view = config.read_with(&strict_policy);
    assert!(strict_view.get::<Vec<u16>>("ports").is_err());
    assert_eq!(config.default_read_policy(), &default_policy);
    assert_eq!(config.get::<Vec<u16>>("ports").unwrap(), vec![8080, 8081]);
}

#[test]
fn test_config_preserves_scalar_and_collection_source_shapes() {
    let mut config = Config::new();
    config.set("scalar", 42_i32).expect("set scalar");
    config
        .set("collection", vec![42_i32])
        .expect("set collection");

    assert_eq!(config.deserialize::<i32>("scalar").expect("scalar"), 42);
    assert_eq!(
        config
            .deserialize::<Vec<i32>>("collection")
            .expect("collection"),
        vec![42]
    );
}

#[test]
fn test_config_splits_scalar_text_but_preserves_collection_items() {
    let mut config = Config::new();
    config.set_default_read_policy(ReadPolicy::env_friendly());
    config.set("scalar_text", "a,b").expect("set scalar text");
    config
        .set("collection_text", vec!["a,b"])
        .expect("set collection text");

    assert_eq!(
        config
            .get_list::<String>("scalar_text")
            .expect("split scalar"),
        vec!["a".to_string(), "b".to_string()]
    );
    assert_eq!(
        config
            .get_list::<String>("collection_text")
            .expect("preserve collection item"),
        vec!["a,b".to_string()]
    );
}

// ============================================================================
// Constructor Tests
// ============================================================================

#[cfg(test)]
mod test_new {
    use super::BlankStringPolicy;
    use super::Config;
    use super::ConfigError;
    use super::DataType;
    use super::Deserialize;
    use super::InterpolationSources;
    use super::MultiValues;
    use super::Property;
    use super::ReadPolicy;
    use super::create_test_config;
    use super::create_test_config_with_description;

    #[test]
    fn test_new_creates_empty_config() {
        let config = Config::new();
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
    fn test_new_has_correct_default_values() {
        let config = Config::new();
        assert_eq!(
            config.default_read_policy().interpolation_sources(),
            InterpolationSources::ConfigOnly
        );
        assert_eq!(config.default_read_policy().max_interpolation_depth(), 64);
    }
}

#[cfg(test)]
mod test_with_description {
    use super::BlankStringPolicy;
    use super::Config;
    use super::ConfigError;
    use super::DataType;
    use super::Deserialize;
    use super::InterpolationSources;
    use super::MultiValues;
    use super::Property;
    use super::ReadPolicy;
    use super::create_test_config;
    use super::create_test_config_with_description;

    #[test]
    fn test_with_description_creates_config_with_description() {
        let config = Config::builder().description("Test Configuration").build();
        assert_eq!(config.description(), Some("Test Configuration"));
        assert!(config.is_empty());
    }

    #[test]
    fn test_with_description_has_correct_default_values() {
        let config = Config::builder().description("Test Configuration").build();
        assert_eq!(
            config.default_read_policy().interpolation_sources(),
            InterpolationSources::ConfigOnly
        );
        assert_eq!(config.default_read_policy().max_interpolation_depth(), 64);
    }

    #[test]
    fn test_with_description_with_empty_string() {
        let config = Config::builder().description("").build();
        assert_eq!(config.description(), Some(""));
    }
}

// ============================================================================
// Basic Property Access Tests
// ============================================================================

#[cfg(test)]
mod test_description {
    use super::BlankStringPolicy;
    use super::Config;
    use super::ConfigError;
    use super::DataType;
    use super::Deserialize;
    use super::MultiValues;
    use super::Property;
    use super::ReadPolicy;
    use super::create_test_config;
    use super::create_test_config_with_description;

    #[test]
    fn test_description_returns_none_for_new_config() {
        let config = Config::new();
        assert!(config.description().is_none());
    }

    #[test]
    fn test_description_returns_some_for_config_with_description() {
        let config = Config::builder().description("Test Configuration").build();
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
        let mut config = Config::builder()
            .description("Original description")
            .build();
        config.set_description(None);
        assert!(config.description().is_none());
    }

    #[test]
    fn test_set_description_updates_description() {
        let mut config = Config::builder()
            .description("Original description")
            .build();
        config.set_description(Some("New description".to_string()));
        assert_eq!(config.description(), Some("New description"));
    }
}

#[cfg(test)]
mod test_variable_substitution {
    use super::Config;
    use super::ConfigError;
    use super::DataType;
    use super::Deserialize;
    use super::InterpolationSources;
    use super::MultiValues;
    use super::Property;
    use super::create_test_config;
    use super::create_test_config_with_description;

    #[test]
    fn test_generic_get_string_preserves_placeholder() {
        let mut config = Config::new();
        config.set("host", "localhost").unwrap();
        config.set("url", "http://${host}/api").unwrap();

        let url: String = config.get("url").unwrap();

        assert_eq!(url, "http://${host}/api");
    }

    #[test]
    fn test_generic_get_does_not_convert_unexpanded_numeric_placeholder() {
        let mut config = Config::new();
        config.set("port", "8080").unwrap();
        config.set("server.port", "${port}").unwrap();

        let result = config.get::<u16>("server.port");

        assert!(matches!(
            result,
            Err(ConfigError::ConversionError { key, .. })
                if key == "server.port"
        ));
    }

    #[test]
    fn test_generic_get_string_list_preserves_placeholders() {
        let mut config = Config::new();
        config.set("root", "/srv/app").unwrap();
        config
            .set("paths", vec!["${root}/bin", "${root}/lib"])
            .unwrap();

        let paths: Vec<String> = config.get("paths").unwrap();

        assert_eq!(paths, vec!["${root}/bin", "${root}/lib"]);
    }

    #[test]
    fn test_strict_string_read_preserves_raw_placeholder() {
        let mut config = Config::new();
        config.set("host", "localhost").unwrap();
        config.set("url", "http://${host}/api").unwrap();

        let url: String = config.get_strict("url").unwrap();

        assert_eq!(url, "http://${host}/api");
    }
}

// ============================================================================
// Configuration Item Management Tests
// ============================================================================

#[cfg(test)]
mod test_contains {
    use super::Config;
    use super::ConfigError;
    use super::DataType;
    use super::Deserialize;
    use super::InterpolationSources;
    use super::MultiValues;
    use super::Property;
    use super::create_test_config;
    use super::create_test_config_with_description;

    #[test]
    fn test_contains_returns_false_for_empty_config() {
        let config = Config::new();
        assert!(!config.contains("nonexistent").unwrap());
    }

    #[test]
    fn test_contains_returns_true_for_existing_property() {
        let mut config = Config::new();
        config.set("test", "value").unwrap();
        assert!(config.contains("test").unwrap());
    }

    #[test]
    fn test_contains_returns_false_for_nonexistent_property() {
        let mut config = Config::new();
        config.set("test", "value").unwrap();
        assert!(!config.contains("other").unwrap());
    }
}

#[cfg(test)]
mod test_get_property {
    use super::Config;
    use super::ConfigError;
    use super::DataType;
    use super::Deserialize;
    use super::MultiValues;
    use super::Property;
    use super::create_test_config;
    use super::create_test_config_with_description;

    #[test]
    fn test_get_property_returns_none_for_nonexistent_property() {
        let config = Config::new();
        assert!(config.get_property("nonexistent").unwrap().is_none());
    }

    #[test]
    fn test_get_property_returns_some_for_existing_property() {
        let mut config = Config::new();
        config.set("test", "value").unwrap();
        let property = config.get_property("test").unwrap();
        assert!(property.is_some());
    }
}

#[cfg(test)]
mod test_get_property_mut {
    use super::Config;
    use super::ConfigError;
    use super::DataType;
    use super::Deserialize;
    use super::MultiValues;
    use super::Property;
    use super::create_test_config;
    use super::create_test_config_with_description;

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

            let unset_result = property.unset();
            assert!(matches!(unset_result, Err(ConfigError::PropertyIsFinal(_))));

            let unset_result = property.set_final(false);
            assert!(matches!(unset_result, Err(ConfigError::PropertyIsFinal(_))));
        }

        assert_eq!(config.get::<String>("test").unwrap(), "value");
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
            config.get::<Vec<String>>("test").unwrap(),
            vec!["second".to_string(), "third".to_string()],
        );
        assert_eq!(
            config.get_property("test").unwrap().unwrap().description(),
            Some("updated description"),
        );
    }
}

#[cfg(test)]
mod test_remove {
    use super::Config;
    use super::ConfigError;
    use super::DataType;
    use super::Deserialize;
    use super::MultiValues;
    use super::Property;
    use super::create_test_config;
    use super::create_test_config_with_description;

    #[test]
    fn test_remove_returns_none_for_nonexistent_property() {
        let mut config = Config::new();
        assert!(config.remove("nonexistent").unwrap().is_none());
    }

    #[test]
    fn test_remove_returns_property_and_removes_it() {
        let mut config = Config::new();
        config.set("test", "value").unwrap();
        assert!(config.contains("test").unwrap());

        let removed = config.remove("test").unwrap();
        assert!(removed.is_some());
        assert!(!config.contains("test").unwrap());
    }

    #[test]
    fn test_remove_final_property_returns_error_and_keeps_value() {
        let mut config = Config::new();
        config.set("test", "value").unwrap();
        config.set_final("test", true).unwrap();

        let result = config.remove("test");
        assert!(matches!(result, Err(ConfigError::PropertyIsFinal(_))));
        assert!(config.contains("test").unwrap());
        assert_eq!(config.get::<String>("test").unwrap(), "value");
    }
}

#[cfg(test)]
mod test_clear {
    use super::Config;
    use super::ConfigError;
    use super::DataType;
    use super::Deserialize;
    use super::MultiValues;
    use super::Property;
    use super::create_test_config;
    use super::create_test_config_with_description;

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
        assert_eq!(config.get::<String>("string_value").unwrap(), "test");
    }
}

#[cfg(test)]
mod test_len {
    use super::Config;
    use super::ConfigError;
    use super::DataType;
    use super::Deserialize;
    use super::MultiValues;
    use super::Property;
    use super::create_test_config;
    use super::create_test_config_with_description;

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
    use super::Config;
    use super::ConfigError;
    use super::DataType;
    use super::Deserialize;
    use super::MultiValues;
    use super::Property;
    use super::create_test_config;
    use super::create_test_config_with_description;

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
    use super::Config;
    use super::ConfigError;
    use super::DataType;
    use super::Deserialize;
    use super::MultiValues;
    use super::Property;
    use super::create_test_config;
    use super::create_test_config_with_description;

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
    use super::Config;
    use super::ConfigError;
    use super::DataType;
    #[cfg(feature = "chrono")]
    use super::DateTime;
    use super::Deserialize;
    use super::MultiValues;
    #[cfg(feature = "chrono")]
    use super::NaiveDate;
    #[cfg(feature = "chrono")]
    use super::NaiveDateTime;
    #[cfg(feature = "chrono")]
    use super::NaiveTime;
    use super::Property;
    #[cfg(feature = "chrono")]
    use super::Utc;
    use super::create_test_config;
    use super::create_test_config_with_description;

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
    #[cfg(feature = "chrono")]
    #[test]
    fn test_get_naive_date() {
        let mut config = Config::new();
        let date = NaiveDate::from_ymd_opt(2023, 12, 25).unwrap();
        config.set("test", date).unwrap();
        let value: NaiveDate = config.get("test").unwrap();
        assert_eq!(value, date);
    }

    #[cfg(feature = "chrono")]
    #[test]
    fn test_get_naive_time() {
        let mut config = Config::new();
        let time = NaiveTime::from_hms_opt(12, 30, 45).unwrap();
        config.set("test", time).unwrap();
        let value: NaiveTime = config.get("test").unwrap();
        assert_eq!(value, time);
    }

    #[cfg(feature = "chrono")]
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

    #[cfg(feature = "chrono")]
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
    use super::Config;
    use super::ConfigError;
    use super::DataType;
    use super::Deserialize;
    use super::MultiValues;
    use super::Property;
    use super::create_test_config;
    use super::create_test_config_with_description;

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
    fn test_get_or_with_str_default_for_string() {
        let config = Config::new();

        let value = config.get_or::<String>("nonexistent", "default").unwrap();

        assert_eq!(value, "default");
    }

    #[test]
    fn test_get_or_accepts_owned_and_borrowed_names() {
        let mut config = Config::new();
        config.set("server.port", "8080").unwrap();

        let owned = "server.port".to_string();
        let value = config.get_or::<u16>(owned, 9000).unwrap();
        assert_eq!(value, 8080);

        let borrowed = "missing.port".to_string();
        let value = config.get_or::<u16>(&borrowed, 9000).unwrap();
        assert_eq!(value, 9000);
    }

    #[test]
    fn test_get_or_with_str_array_default_for_string_list() {
        let config = Config::new();

        let values = config
            .get_or::<Vec<String>>("nonexistent", ["default1", "default2"])
            .unwrap();

        assert_eq!(values, vec!["default1".to_string(), "default2".to_string()]);
    }

    #[test]
    fn test_get_or_with_str_slice_default_for_string_list() {
        let config = Config::new();
        let defaults = ["default1", "default2"];

        let values = config
            .get_or::<Vec<String>>("nonexistent", defaults.as_slice())
            .unwrap();

        assert_eq!(values, vec!["default1".to_string(), "default2".to_string()]);
    }

    #[test]
    fn test_get_or_with_string_vec_ref_default_for_string_list() {
        let config = Config::new();
        let defaults = vec!["default1".to_string(), "default2".to_string()];

        let values = config
            .get_or::<Vec<String>>("nonexistent", &defaults)
            .unwrap();

        assert_eq!(values, defaults);
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
