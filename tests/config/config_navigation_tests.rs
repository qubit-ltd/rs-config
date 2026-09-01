// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// # [`qubit_config::Config`] unit tests
//
// Covers the public `Config` API (including APIs introduced in v0.3.0).

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
pub(crate) use qubit_datatype::DataConversionError;
pub(crate) use qubit_datatype::DataConversionErrorKind;
pub(crate) use qubit_datatype::DataType;
pub(crate) use qubit_datatype::InvalidValueReason;
pub(crate) use qubit_value::MultiValues;
pub(crate) use qubit_value::ValueError;
pub(crate) use serde::Deserialize;
pub(crate) use serde_json::Value as JsonValue;
pub(crate) use serde_json::from_value;
pub(crate) use serde_json::json;
pub(crate) use serde_json::to_value;

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
pub(crate) fn set_max_interpolation_depth(
    config: &mut Config,
    max_depth: usize,
) {
    let options = ReadPolicy::builder_from(config.default_read_policy())
        .max_interpolation_depth(max_depth)
        .build();
    config.set_default_read_policy(options);
}
// ============================================================================
// iter_prefix() Tests
// ============================================================================

#[cfg(test)]
mod test_iter_prefix {
    use super::Config;
    use super::ConfigError;
    use super::DataType;
    use super::Deserialize;
    use super::MultiValues;
    use super::Property;
    use super::create_test_config;
    use super::create_test_config_with_description;

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
    use super::Config;
    use super::ConfigError;
    use super::ConfigReader;
    use super::DataType;
    use super::Deserialize;
    use super::MultiValues;
    use super::Property;
    use super::create_test_config;
    use super::create_test_config_with_description;

    #[test]
    fn test_contains_prefix_empty_config() {
        let config = Config::new();
        assert!(!config.contains_key_prefix("http."));
    }

    #[test]
    fn test_contains_prefix_match() {
        let mut config = Config::new();
        config.set("http.host", "localhost").unwrap();
        assert!(config.contains_key_prefix("http."));
    }

    #[test]
    fn test_contains_prefix_no_match() {
        let mut config = Config::new();
        config.set("db.host", "dbhost").unwrap();
        assert!(!config.contains_key_prefix("http."));
    }

    #[test]
    fn test_contains_prefix_partial_key_name() {
        let mut config = Config::new();
        config.set("http.host", "localhost").unwrap();
        // "http" is a prefix of "http.host"
        assert!(config.contains_key_prefix("http"));
        // "htt" is also a prefix
        assert!(config.contains_key_prefix("htt"));
    }

    #[test]
    fn test_contains_prefix_empty_prefix() {
        let mut config = Config::new();
        config.set("host", "localhost").unwrap();
        // Empty string is a prefix of everything
        assert!(config.contains_key_prefix(""));
    }

    #[test]
    fn test_contains_section_uses_dotted_key_boundary() {
        let mut config = Config::new();
        config.set("proxy", "scalar").unwrap();
        config.set("proxy2.host", "sibling").unwrap();

        assert!(!config.contains_section("proxy").unwrap());

        config.set("proxy.host", "localhost").unwrap();
        assert!(config.contains_section("proxy").unwrap());
    }

    #[test]
    fn test_section_contains_section_uses_relative_boundary() {
        let mut config = Config::new();
        config.set("http.proxy2.host", "sibling").unwrap();
        let http = config.section("http").unwrap();

        assert!(!http.contains_section("proxy").unwrap());

        config.set("http.proxy.host", "localhost").unwrap();
        let http = config.section("http").unwrap();
        assert!(http.contains_section("proxy").unwrap());
    }

    #[test]
    fn test_section_if_present_excludes_exact_root_property() {
        let mut config = Config::new();
        config.set("proxy", "scalar").unwrap();
        config.set("proxy.host", "localhost").unwrap();

        let proxy = config.section_if_present("proxy").unwrap().unwrap();
        assert_eq!(proxy.get::<String>("host").unwrap(), "localhost");
        assert!(config.section_if_present("proxy2").unwrap().is_none());
    }
}

// ============================================================================
// get() / get_list() additional error paths
// ============================================================================

#[cfg(test)]
mod test_get_and_get_list_error_mapping_additional_paths {
    use super::Config;
    use super::ConfigError;
    use super::DataConversionErrorKind;
    use super::DataType;
    use super::Deserialize;
    use super::MultiValues;
    use super::Property;
    use super::create_test_config;
    use super::create_test_config_with_description;

    #[test]
    fn test_get_and_get_list_error_mapping_additional_paths() {
        use serde_json::Value as JsonValue;

        let mut config = Config::new();
        config
            .set(
                "map_value",
                std::collections::HashMap::from([(
                    "key".to_string(),
                    "value".to_string(),
                )]),
            )
            .unwrap();
        config.set("bad_int", "abc").unwrap();
        config.set("bad_json", "{invalid-json").unwrap();

        // Unsupported conversion path in get().
        let err = config.get::<i32>("map_value").unwrap_err();
        assert!(matches!(
            err,
            ConfigError::ConversionError { .. }
                | ConfigError::TypeMismatch { .. }
        ));

        // Invalid syntax path in get().
        let err = config.get::<i32>("bad_int").unwrap_err();
        assert!(matches!(
            err,
            ConfigError::ConversionError { .. }
                | ConfigError::TypeMismatch { .. }
        ));

        // JSON deserialization error path in get().
        let err = config.get::<JsonValue>("bad_json").unwrap_err();
        assert!(matches!(
            err,
            ConfigError::ConversionError { .. }
                | ConfigError::TypeMismatch { .. }
        ));

        // An unset property remains missing for list conversion.
        config.set_null("empty_list", DataType::String).unwrap();
        assert!(matches!(
            config.get_list::<String>("empty_list"),
            Err(ConfigError::PropertyHasNoValue(key)) if key == "empty_list"
        ));

        let error = config
            .get_list::<i32>("map_value")
            .expect_err("a JSON object cannot convert to an integer list");
        match error {
            ConfigError::ConversionError {
                key,
                source_index,
                source,
            } => {
                assert_eq!(key, "map_value");
                assert_eq!(source_index, None);
                assert_eq!(source.kind(), DataConversionErrorKind::Unsupported,);
            }
            error => panic!("expected ConversionError, got {error:?}"),
        }
    }
}

// ============================================================================
// is_unset() Tests
// ============================================================================

#[cfg(test)]
mod test_is_unset {
    use super::Config;
    use super::ConfigError;
    use super::DataType;
    use super::Deserialize;
    use super::MultiValues;
    use super::Property;
    use super::create_test_config;
    use super::create_test_config_with_description;

    #[test]
    fn test_is_unset_missing_key_returns_false() {
        let config = Config::new();
        assert!(!config.is_unset("missing").unwrap());
    }

    #[test]
    fn test_is_unset_key_with_value_returns_false() {
        let mut config = Config::new();
        config.set("host", "localhost").unwrap();
        assert!(!config.is_unset("host").unwrap());
    }

    #[test]
    fn test_is_unset_empty_property_returns_true() {
        let mut config = Config::new();
        config.set_null("nullable", DataType::String).unwrap();
        assert!(config.is_unset("nullable").unwrap());
    }

    #[test]
    fn test_is_unset_after_unset() {
        let mut config = Config::new();
        config.set("host", "localhost").unwrap();
        config
            .get_property_mut("host")
            .unwrap()
            .unwrap()
            .unset()
            .unwrap();
        assert!(config.is_unset("host").unwrap());
    }
}

// ============================================================================
// get_optional() Tests
// ============================================================================

#[cfg(test)]
mod test_get_optional {
    use super::Config;
    use super::ConfigError;
    use super::DataType;
    use super::Deserialize;
    use super::MultiValues;
    use super::Property;
    use super::create_test_config;
    use super::create_test_config_with_description;

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
    use super::Config;
    use super::ConfigError;
    use super::DataType;
    use super::Deserialize;
    use super::MultiValues;
    use super::Property;
    use super::create_test_config;
    use super::create_test_config_with_description;

    #[test]
    fn test_get_optional_list_missing_key_returns_none() {
        let config = Config::new();
        let result: Option<Vec<i32>> =
            config.get_optional_list("missing").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_get_optional_list_existing_key_returns_some() {
        let mut config = Config::new();
        config.set("ports", vec![8080, 8081, 8082]).unwrap();
        let result: Option<Vec<i32>> =
            config.get_optional_list("ports").unwrap();
        assert_eq!(result, Some(vec![8080, 8081, 8082]));
    }

    #[test]
    fn test_get_optional_list_null_property_returns_none() {
        let mut config = Config::new();
        config.set_null("nullable", DataType::Int32).unwrap();
        let result: Option<Vec<i32>> =
            config.get_optional_list("nullable").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_get_optional_list_single_value() {
        let mut config = Config::new();
        config.set("port", 8080).unwrap();
        let result: Option<Vec<i32>> =
            config.get_optional_list("port").unwrap();
        assert_eq!(result, Some(vec![8080]));
    }

    #[test]
    fn test_get_optional_list_type_mismatch_returns_error() {
        let mut config = Config::new();
        config.set("ports", vec!["yes", "no"]).unwrap();
        let result: Result<Option<Vec<bool>>, _> =
            config.get_optional_list("ports");
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
    use super::Config;
    use super::ConfigError;
    use super::DataType;
    use super::Deserialize;
    use super::MultiValues;
    use super::Property;
    use super::create_test_config;
    use super::create_test_config_with_description;

    #[test]
    fn test_get_optional_string_missing_returns_none() {
        let config = Config::new();
        assert_eq!(config.get_optional::<String>("missing").unwrap(), None);
    }

    #[test]
    fn test_get_optional_string_null_returns_none() {
        let mut config = Config::new();
        config.set_null("n", DataType::String).unwrap();
        assert_eq!(config.get_optional::<String>("n").unwrap(), None);
    }

    #[test]
    fn test_get_optional_string_null_non_string_empty_still_none() {
        let mut config = Config::new();
        config.set_null("nullable", DataType::Int32).unwrap();
        assert_eq!(config.get_optional::<String>("nullable").unwrap(), None);
    }

    #[test]
    fn test_get_optional_string_plain_no_variables() {
        let mut config = Config::new();
        config.set("greeting", "hello").unwrap();
        assert_eq!(
            config
                .get_optional::<String>("greeting")
                .unwrap()
                .as_deref(),
            Some("hello")
        );
    }

    #[test]
    fn test_get_optional_string_empty_string_is_some() {
        let mut config = Config::new();
        config.set("empty", "").unwrap();
        assert_eq!(
            config.get_optional::<String>("empty").unwrap().as_deref(),
            Some("")
        );
    }

    #[test]
    fn test_get_optional_string_substitution() {
        let mut config = Config::new();
        config.set("base", "http://localhost").unwrap();
        config.set("api", "${base}/api").unwrap();
        assert_eq!(
            config
                .get_optional_interpolated::<String>("api")
                .unwrap()
                .as_deref(),
            Some("http://localhost/api")
        );
    }

    #[test]
    fn test_get_optional_string_preserves_placeholders() {
        let mut config = Config::new();
        config.set("raw", "${not_replaced}").unwrap();
        assert_eq!(
            config.get_optional::<String>("raw").unwrap().as_deref(),
            Some("${not_replaced}")
        );
    }

    #[test]
    fn test_get_optional_string_type_mismatch_returns_error() {
        let mut config = Config::new();
        config.set("port", 8080i32).unwrap();
        assert_eq!(
            config.get_optional::<String>("port").unwrap(),
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
        let result = config.get_optional_interpolated::<String>("bad");
        assert!(matches!(
            result,
            Err(ConfigError::SubstitutionError { path, .. }) if path == "bad"
        ));
    }

    #[test]
    fn test_get_optional_string_substitution_depth_exceeded() {
        let mut config = Config::new();
        crate::set_max_interpolation_depth(&mut config, 0);
        config.set("a", "v").unwrap();
        config.set("b", "${a}").unwrap();
        let result = config.get_optional_interpolated::<String>("b");
        assert!(matches!(
            result,
            Err(ConfigError::SubstitutionDepthExceeded {
                path,
                max_depth: 0,
            }) if path == "b"
        ));
    }

    #[test]
    fn test_get_optional_string_list_missing_returns_none() {
        let config = Config::new();
        assert_eq!(
            config.get_optional::<Vec<String>>("missing").unwrap(),
            None
        );
    }

    #[test]
    fn test_get_optional_string_list_null_returns_none() {
        let mut config = Config::new();
        config.set_null("nullable", DataType::String).unwrap();
        assert_eq!(
            config.get_optional::<Vec<String>>("nullable").unwrap(),
            None
        );
    }

    #[test]
    fn test_get_optional_string_list_substitution() {
        let mut config = Config::new();
        config.set("root", "/opt/app").unwrap();
        config
            .set("paths", vec!["${root}/bin", "${root}/lib"])
            .unwrap();
        assert_eq!(
            config
                .get_optional_interpolated::<Vec<String>>("paths")
                .unwrap(),
            Some(vec!["/opt/app/bin".to_string(), "/opt/app/lib".to_string()])
        );
    }

    #[test]
    fn test_get_optional_string_list_plain_no_variables() {
        let mut config = Config::new();
        config.set("items", vec!["a", "b"]).unwrap();
        assert_eq!(
            config.get_optional::<Vec<String>>("items").unwrap(),
            Some(vec!["a".to_string(), "b".to_string()])
        );
    }

    #[test]
    fn test_get_optional_string_list_single_scalar_coerced_to_one_element() {
        let mut config = Config::new();
        config.set("only", "solo").unwrap();
        assert_eq!(
            config.get_optional::<Vec<String>>("only").unwrap(),
            Some(vec!["solo".to_string()])
        );
    }

    #[test]
    fn test_get_optional_string_list_preserves_concrete_empty_collection() {
        let mut config = Config::new();
        config.set("empty_list", Vec::<String>::new()).unwrap();

        assert_eq!(
            config.get_optional::<Vec<String>>("empty_list").unwrap(),
            Some(Vec::new())
        );
    }

    #[test]
    fn test_get_optional_string_list_preserves_placeholders() {
        let mut config = Config::new();
        config.set("items", vec!["${x}", "y"]).unwrap();
        assert_eq!(
            config.get_optional::<Vec<String>>("items").unwrap(),
            Some(vec!["${x}".to_string(), "y".to_string()])
        );
    }

    #[test]
    fn test_get_optional_string_list_type_mismatch_returns_error() {
        let mut config = Config::new();
        config.set("ports", vec![1i32, 2i32]).unwrap();
        assert_eq!(
            config.get_optional::<Vec<String>>("ports").unwrap(),
            Some(vec!["1".to_string(), "2".to_string()])
        );
    }

    #[test]
    fn test_get_optional_string_list_unresolved_variable_in_element_returns_error()
     {
        let mut config = Config::new();
        config
            .set(
                "items",
                vec!["ok", "${qubit_cfg_list_bad_var_that_must_not_exist_9f8e7d6c5b4a}"],
            )
            .unwrap();
        let result = config.get_optional_interpolated::<Vec<String>>("items");
        assert!(matches!(
            result,
            Err(ConfigError::SubstitutionError { path, .. })
                if path == "items"
        ));
    }

    #[test]
    fn test_get_optional_string_list_substitution_depth_exceeded() {
        let mut config = Config::new();
        crate::set_max_interpolation_depth(&mut config, 0);
        config.set("a", "x").unwrap();
        config.set("items", vec!["${a}"]).unwrap();
        let result = config.get_optional_interpolated::<Vec<String>>("items");
        assert!(matches!(
            result,
            Err(ConfigError::SubstitutionDepthExceeded {
                path,
                max_depth: 0,
            }) if path == "items"
        ));
    }
}
