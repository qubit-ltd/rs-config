/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! [`qubit_config::ConfigReader`] tests.

use qubit_config::field::ConfigField;
use qubit_config::options::{BlankStringPolicy, ConfigReadOptions};
use qubit_config::{Config, ConfigError, ConfigPrefixView, ConfigReader};
use qubit_datatype::DataType;
use serde::Deserialize;

fn create_test_config() -> Config {
    let mut config = Config::new();
    config.set("string_value", "test").unwrap();
    config.set("int_value", 42).unwrap();
    config.set("bool_value", true).unwrap();
    config.set("float_value", 3.5).unwrap();
    config
}

#[cfg(test)]
mod test_config_reader_smoke {
    #[allow(unused_imports)]
    use super::{
        BlankStringPolicy, Config, ConfigError, ConfigField, ConfigPrefixView, ConfigReadOptions,
        ConfigReader, DataType, Deserialize, create_test_config,
    };

    #[test]
    fn config_exposes_config_reader_string_api() {
        let c = create_test_config();
        assert_eq!(
            ConfigReader::get_string(&c, "string_value").unwrap(),
            "test"
        );
    }
}

#[cfg(test)]
mod test_config_reader {
    #[allow(unused_imports)]
    use super::{
        BlankStringPolicy, Config, ConfigError, ConfigField, ConfigPrefixView, ConfigReadOptions,
        ConfigReader, DataType, Deserialize, create_test_config,
    };

    fn read_host(reader: &impl ConfigReader) -> String {
        reader.get_string("host").unwrap()
    }

    fn read_http_host_via_prefix_view(reader: &impl ConfigReader) -> String {
        reader.prefix_view("http").get_string("host").unwrap()
    }

    #[test]
    fn test_config_implements_config_reader() {
        let mut config = Config::new();
        config.set("host", "localhost").unwrap();
        assert_eq!(read_host(&config), "localhost");
    }

    #[test]
    fn test_config_reader_prefix_view_on_config_and_nested_view() {
        let mut config = Config::new();
        config.set("http.host", "localhost").unwrap();
        config.set("http.proxy.addr", "127.0.0.1").unwrap();
        assert_eq!(read_http_host_via_prefix_view(&config), "localhost");
        let http = ConfigReader::prefix_view(&config, "http");
        let addr: String = ConfigReader::prefix_view(&http, "proxy")
            .get_string("addr")
            .unwrap();
        assert_eq!(addr, "127.0.0.1");
    }
    #[test]
    fn test_trait_default_methods_with_substitution_disabled_branches() {
        let mut config = Config::new();
        config.set_enable_variable_substitution(false);
        config.set("name", "${who}").unwrap();
        config.set("names", vec!["${a}", "${b}"]).unwrap();

        let name = <Config as ConfigReader>::get_string(&config, "name").unwrap();
        let names = <Config as ConfigReader>::get_string_list(&config, "names").unwrap();
        assert_eq!(name, "${who}");
        assert_eq!(names, vec!["${a}".to_string(), "${b}".to_string()]);
    }

    #[test]
    fn test_trait_default_methods_on_config_reader() {
        let mut config = Config::new();
        config.set("name", "alice").unwrap();
        config.set("names", vec!["alice", "bob"]).unwrap();
        config.set("url", "http://${name}").unwrap();

        let reader = &config;
        assert_eq!(
            <Config as ConfigReader>::get_string_or(reader, "missing", "fallback").unwrap(),
            "fallback"
        );
        assert_eq!(
            <Config as ConfigReader>::get_string_list_or(reader, "missing.list", &["x", "y"])
                .unwrap(),
            vec!["x".to_string(), "y".to_string()]
        );
        assert_eq!(
            <Config as ConfigReader>::get_optional_string(reader, "name").unwrap(),
            Some("alice".to_string())
        );
        assert_eq!(
            <Config as ConfigReader>::get_optional_string(reader, "missing").unwrap(),
            None
        );
        assert_eq!(
            <Config as ConfigReader>::get_optional_string_list(reader, "names").unwrap(),
            Some(vec!["alice".to_string(), "bob".to_string()])
        );
        assert_eq!(
            <Config as ConfigReader>::get_optional_string_list(reader, "missing.names").unwrap(),
            None
        );
        assert_eq!(
            <Config as ConfigReader>::get_string_or(reader, "url", "fallback").unwrap(),
            "http://alice"
        );
    }

    #[test]
    fn test_trait_default_methods_on_view_and_forwarding() {
        let mut config = Config::new();
        config.set("http.host", "localhost").unwrap();
        config.set("http.names", vec!["a", "b"]).unwrap();
        config.set("http.timeout", 30).unwrap();
        config.set("http.url", "http://${host}").unwrap();

        let view = config.prefix_view("http");
        assert_eq!(
            <ConfigPrefixView<'_> as ConfigReader>::get_string_or(&view, "missing", "fallback")
                .unwrap(),
            "fallback"
        );
        assert_eq!(
            <ConfigPrefixView<'_> as ConfigReader>::get_string_list_or(&view, "missing", &["m"])
                .unwrap(),
            vec!["m".to_string()]
        );
        assert_eq!(
            <ConfigPrefixView<'_> as ConfigReader>::get_optional_string(&view, "host").unwrap(),
            Some("localhost".to_string())
        );
        assert_eq!(
            <ConfigPrefixView<'_> as ConfigReader>::get_optional_string_list(&view, "names")
                .unwrap(),
            Some(vec!["a".to_string(), "b".to_string()])
        );
        let timeout: i32 = <ConfigPrefixView<'_> as ConfigReader>::get(&view, "timeout").unwrap();
        assert_eq!(timeout, 30);
        let timeout_list: Vec<i32> =
            <ConfigPrefixView<'_> as ConfigReader>::get_list(&view, "timeout").unwrap();
        assert_eq!(timeout_list, vec![30]);
        assert!(<ConfigPrefixView<'_> as ConfigReader>::contains_prefix(
            &view, "na"
        ));
        let keys: Vec<&str> = <ConfigPrefixView<'_> as ConfigReader>::iter_prefix(&view, "h")
            .map(|(k, _)| k)
            .collect();
        assert_eq!(keys, vec!["host"]);
        assert_eq!(
            <ConfigPrefixView<'_> as ConfigReader>::get_string_or(&view, "url", "fallback")
                .unwrap(),
            "http://localhost"
        );
    }

    #[test]
    fn test_config_reader_get_optional_on_config_and_prefix_view() {
        let mut config = Config::new();
        config.set("http.ipv4_only", true).unwrap();
        config.set("http.port", 8080i32).unwrap();

        assert_eq!(
            <Config as ConfigReader>::get_optional::<bool>(&config, "missing").unwrap(),
            None
        );
        assert_eq!(
            <Config as ConfigReader>::get_optional::<bool>(&config, "http.ipv4_only").unwrap(),
            Some(true)
        );

        let http = config.prefix_view("http");
        assert_eq!(
            <ConfigPrefixView<'_> as ConfigReader>::get_optional::<bool>(&http, "ipv4_only")
                .unwrap(),
            Some(true)
        );
        assert_eq!(
            <ConfigPrefixView<'_> as ConfigReader>::get_optional::<i32>(&http, "port").unwrap(),
            Some(8080)
        );
        assert_eq!(
            <ConfigPrefixView<'_> as ConfigReader>::get_optional::<i32>(&http, "nope").unwrap(),
            None
        );
    }

    #[test]
    fn test_config_reader_uses_conversion_and_exposes_strict_reads() {
        let mut config = Config::new();
        config.set("http.enabled", "1").unwrap();
        config.set("http.flags", vec!["true", "0"]).unwrap();

        assert!(<Config as ConfigReader>::get::<bool>(&config, "http.enabled").unwrap());
        assert_eq!(
            <Config as ConfigReader>::get_list::<bool>(&config, "http.flags").unwrap(),
            vec![true, false]
        );
        assert!(<Config as ConfigReader>::get_strict::<bool>(&config, "http.enabled").is_err());
        assert!(<Config as ConfigReader>::get_list_strict::<bool>(&config, "http.flags").is_err());

        let http = config.prefix_view("http");
        assert!(<ConfigPrefixView<'_> as ConfigReader>::get::<bool>(&http, "enabled").unwrap());
        assert_eq!(
            <ConfigPrefixView<'_> as ConfigReader>::get_list::<bool>(&http, "flags").unwrap(),
            vec![true, false]
        );
        assert!(
            <ConfigPrefixView<'_> as ConfigReader>::get_strict::<bool>(&http, "enabled").is_err()
        );
        assert!(
            <ConfigPrefixView<'_> as ConfigReader>::get_list_strict::<bool>(&http, "flags")
                .is_err()
        );
    }

    #[test]
    fn test_config_reader_forwarding_for_config_impl() {
        let mut config = Config::new();
        config.set("db.host", "127.0.0.1").unwrap();
        config.set("db.port", 5432).unwrap();

        let reader = &config;
        assert!(<Config as ConfigReader>::contains(reader, "db.host"));
        assert!(<Config as ConfigReader>::contains_prefix(reader, "db."));
        let host: String = <Config as ConfigReader>::get(reader, "db.host").unwrap();
        let port_list: Vec<i32> = <Config as ConfigReader>::get_list(reader, "db.port").unwrap();
        assert_eq!(host, "127.0.0.1");
        assert_eq!(port_list, vec![5432]);
        let keys: Vec<&str> = <Config as ConfigReader>::iter_prefix(reader, "db.")
            .map(|(k, _)| k)
            .collect();
        assert_eq!(keys.len(), 2);
    }
}

/// Exercises [`ConfigReader`] methods added to mirror [`Config`]'s read-only API.
#[cfg(test)]
mod test_config_reader_extended_surface {
    #[allow(unused_imports)]
    use super::{
        BlankStringPolicy, Config, ConfigError, ConfigField, ConfigPrefixView, ConfigReadOptions,
        ConfigReader, DataType, Deserialize, create_test_config,
    };

    #[derive(Deserialize, Debug, PartialEq, Eq)]
    struct Server {
        host: String,
        port: i32,
    }

    #[test]
    fn description_matches_underlying_config_on_root_and_prefix_view() {
        let config = Config::with_description("app");
        assert_eq!(<Config as ConfigReader>::description(&config), Some("app"));
        let view = config.prefix_view("any");
        assert_eq!(
            <ConfigPrefixView<'_> as ConfigReader>::description(&view),
            Some("app")
        );
    }

    #[test]
    fn resolve_key_returns_root_relative_paths() {
        let mut config = Config::new();
        config.set("http.proxy.host", "proxy").unwrap();

        assert_eq!(<Config as ConfigReader>::resolve_key(&config, "k"), "k");
        assert_eq!(<Config as ConfigReader>::resolve_key(&config, ""), "");

        let http = config.prefix_view("http");
        assert_eq!(
            <ConfigPrefixView<'_> as ConfigReader>::resolve_key(&http, "proxy.host"),
            "http.proxy.host"
        );
        assert_eq!(
            <ConfigPrefixView<'_> as ConfigReader>::resolve_key(&http, ""),
            "http"
        );

        let proxy = http.prefix_view("proxy");
        assert_eq!(
            <ConfigPrefixView<'_> as ConfigReader>::resolve_key(&proxy, "host"),
            "http.proxy.host"
        );
        assert_eq!(
            <ConfigPrefixView<'_> as ConfigReader>::resolve_key(&proxy, ""),
            "http.proxy"
        );
    }

    #[test]
    fn get_property_len_keys_iter_get_or_is_null_and_optional_string_empty() {
        let mut config = Config::new();
        config.set("k", 1i32).unwrap();
        config.set_null("nullish", DataType::Int32).unwrap();
        config.set_null("empty.names", DataType::String).unwrap();

        assert!(<Config as ConfigReader>::get_property(&config, "k").is_some());
        assert!(<Config as ConfigReader>::get_property(&config, "nullish").is_some());
        assert!(<Config as ConfigReader>::is_null(&config, "nullish"));
        assert!(!<Config as ConfigReader>::is_null(&config, "k"));
        assert!(!<Config as ConfigReader>::is_null(&config, "missing"));

        assert_eq!(<Config as ConfigReader>::len(&config), 3);
        assert!(!<Config as ConfigReader>::is_empty(&config));

        let mut keys = <Config as ConfigReader>::keys(&config);
        keys.sort();
        assert_eq!(
            keys,
            vec![
                "empty.names".to_string(),
                "k".to_string(),
                "nullish".to_string(),
            ],
        );

        assert_eq!(<Config as ConfigReader>::iter(&config).count(), 3);

        assert_eq!(
            <Config as ConfigReader>::get_or(&config, "k", 0i32).unwrap(),
            1
        );
        assert_eq!(
            <Config as ConfigReader>::get_or(&config, "missing", 99i32).unwrap(),
            99
        );

        assert_eq!(
            <Config as ConfigReader>::get_optional_string(&config, "nullish").unwrap(),
            None
        );
        assert_eq!(
            <Config as ConfigReader>::get_optional_string_list(&config, "empty.names").unwrap(),
            None
        );
    }

    #[test]
    fn prefix_view_len_keys_iter_get_property_is_null_subconfig() {
        let mut config = Config::new();
        config.set("a.x", 1i32).unwrap();
        config.set("a.y", 2i32).unwrap();
        config.set("b.z", 3i32).unwrap();
        config.set_null("a.empty", DataType::String).unwrap();

        let view = config.prefix_view("a");
        assert_eq!(<ConfigPrefixView<'_> as ConfigReader>::len(&view), 3);
        let mut keys = <ConfigPrefixView<'_> as ConfigReader>::keys(&view);
        keys.sort();
        assert_eq!(
            keys,
            vec!["empty".to_string(), "x".to_string(), "y".to_string()]
        );
        assert_eq!(
            <ConfigPrefixView<'_> as ConfigReader>::iter(&view).count(),
            3
        );

        assert!(<ConfigPrefixView<'_> as ConfigReader>::get_property(&view, "x").is_some());
        assert!(<ConfigPrefixView<'_> as ConfigReader>::is_null(
            &view, "empty"
        ));
        assert_eq!(
            <ConfigPrefixView<'_> as ConfigReader>::get_optional_string(&view, "empty").unwrap(),
            None
        );

        let sub = <ConfigPrefixView<'_> as ConfigReader>::subconfig(&view, "", true).unwrap();
        assert!(sub.contains("x") && sub.contains("y") && sub.contains("empty"));
        assert!(!sub.contains("a.x"));

        let nested =
            <ConfigPrefixView<'_> as ConfigReader>::subconfig(&view, "nope", true).unwrap();
        assert!(nested.is_empty());
    }

    #[test]
    fn test_config_reader_config_get_optional_list_and_subconfig_forwarding() {
        let mut config = Config::new();
        config.set("svc.ports", vec![8080i32, 8081]).unwrap();
        config.set("other.port", 9090i32).unwrap();

        assert_eq!(
            <Config as ConfigReader>::get_optional_list::<i32>(&config, "svc.ports").unwrap(),
            Some(vec![8080, 8081])
        );
        assert_eq!(
            <Config as ConfigReader>::get_optional_list::<i32>(&config, "missing").unwrap(),
            None
        );

        let sub = <Config as ConfigReader>::subconfig(&config, "svc", true).unwrap();
        assert_eq!(sub.get_list::<i32>("ports").unwrap(), vec![8080, 8081]);
        assert!(!sub.contains("other.port"));
    }

    #[test]
    fn test_config_reader_prefix_view_empty_and_optional_list_forwarding() {
        let mut config = Config::new();
        config.set("svc.ports", vec![8080i32, 8081]).unwrap();

        let missing = config.prefix_view("missing");
        assert!(<ConfigPrefixView<'_> as ConfigReader>::is_empty(&missing));

        let view = config.prefix_view("svc");
        assert!(!<ConfigPrefixView<'_> as ConfigReader>::is_empty(&view));
        assert_eq!(
            <ConfigPrefixView<'_> as ConfigReader>::get_optional_list::<i32>(&view, "ports")
                .unwrap(),
            Some(vec![8080, 8081])
        );
        assert_eq!(
            <ConfigPrefixView<'_> as ConfigReader>::get_optional_list::<i32>(&view, "missing")
                .unwrap(),
            None
        );

        let root_view = config.prefix_view("");
        let sub =
            <ConfigPrefixView<'_> as ConfigReader>::subconfig(&root_view, "svc", true).unwrap();
        assert_eq!(sub.get_list::<i32>("ports").unwrap(), vec![8080, 8081]);
    }

    #[test]
    fn deserialize_via_trait_on_config_and_prefix_view() {
        let mut config = Config::new();
        config.set("srv.host", "h").unwrap();
        config.set("srv.port", 5i32).unwrap();

        let server: Server = <Config as ConfigReader>::deserialize(&config, "srv").unwrap();
        assert_eq!(
            server,
            Server {
                host: "h".to_string(),
                port: 5,
            }
        );

        let view = config.prefix_view("srv");
        let again: Server = <ConfigPrefixView<'_> as ConfigReader>::deserialize(&view, "").unwrap();
        assert_eq!(again, server);
    }
}

#[cfg(test)]
mod test_config_reader_alias_reads {
    #[allow(unused_imports)]
    use super::{
        BlankStringPolicy, Config, ConfigError, ConfigField, ConfigPrefixView, ConfigReadOptions,
        ConfigReader, DataType, Deserialize, create_test_config,
    };

    #[test]
    fn test_get_any_or_uses_default_only_when_all_names_missing_or_empty() {
        let config = Config::new();

        let value = config
            .get_any_or::<i32>(&["new.port", "old.port"], 8080)
            .expect("missing aliases should use default");

        assert_eq!(value, 8080);
    }

    #[test]
    fn test_get_any_or_accepts_str_array_default_for_string_list() {
        let config = Config::new();

        let value = config
            .get_any_or::<Vec<String>>(["new.paths", "old.paths"], ["bin", "lib"])
            .expect("missing aliases should use string list default");

        assert_eq!(value, vec!["bin".to_string(), "lib".to_string()]);
    }

    #[test]
    fn test_any_reads_accept_convenient_name_lists() {
        let mut config = Config::new();
        config
            .set("server.port", "8080")
            .expect("setting value should succeed");

        let direct_array = config
            .get_any_or::<u16>(["missing.port", "server.port"], 9000)
            .expect("array names should be accepted directly");
        assert_eq!(direct_array, 8080);

        let array_ref = config
            .get_any_or::<u16>(&["missing.port", "server.port"], 9000)
            .expect("array references should be accepted directly");
        assert_eq!(array_ref, 8080);

        let str_slice_names = ["missing.port", "server.port"];
        let str_slice = config
            .get_any_or::<u16>(str_slice_names.as_slice(), 9000)
            .expect("string slices should be accepted directly");
        assert_eq!(str_slice, 8080);

        let str_vec_names = vec!["missing.port", "server.port"];
        let str_vec = config
            .get_any_or::<u16>(str_vec_names.clone(), 9000)
            .expect("borrowed string vectors should be accepted by value");
        assert_eq!(str_vec, 8080);

        let str_vec_ref = config
            .get_any_or::<u16>(&str_vec_names, 9000)
            .expect("borrowed string vectors should be accepted by reference");
        assert_eq!(str_vec_ref, 8080);

        let owned_vec_names = vec!["missing.port".to_string(), "server.port".to_string()];
        let owned_vec = config
            .get_any_or::<u16>(owned_vec_names.clone(), 9000)
            .expect("owned name vectors should be accepted by value");
        assert_eq!(owned_vec, 8080);

        let owned_vec_ref = config
            .get_any_or::<u16>(&owned_vec_names, 9000)
            .expect("owned name vectors should be accepted by reference");
        assert_eq!(owned_vec_ref, 8080);

        let owned_slice = config
            .get_any_or::<u16>(owned_vec_names.as_slice(), 9000)
            .expect("owned name slices should be accepted directly");
        assert_eq!(owned_slice, 8080);

        let owned_array = ["missing.port".to_string(), "server.port".to_string()];
        let owned_array_value = config
            .get_any_or::<u16>(owned_array.clone(), 9000)
            .expect("owned name arrays should be accepted directly");
        assert_eq!(owned_array_value, 8080);

        let owned_array_ref = config
            .get_any_or::<u16>(&owned_array, 9000)
            .expect("owned name array references should be accepted directly");
        assert_eq!(owned_array_ref, 8080);

        let trait_call = <Config as ConfigReader>::get_any_or::<Vec<String>>(
            &config,
            ["missing.paths", "legacy.paths"],
            ["cache", "tmp"],
        )
        .expect("ConfigReader should accept direct arrays for names and defaults");
        assert_eq!(trait_call, vec!["cache".to_string(), "tmp".to_string()]);
    }

    #[test]
    fn test_get_any_or_with_accepts_str_slice_default_for_string_list() {
        let config = Config::new();
        let defaults = ["bin", "lib"];

        let value = config
            .get_any_or_with::<Vec<String>>(
                &["new.paths", "old.paths"],
                defaults.as_slice(),
                &ConfigReadOptions::default(),
            )
            .expect("missing aliases should use string slice default");

        assert_eq!(value, vec!["bin".to_string(), "lib".to_string()]);
    }

    #[test]
    fn test_get_any_and_get_optional_any_use_alias_order() {
        let mut config = Config::new();
        config
            .set("PORT", "8080")
            .expect("setting alias value should succeed");

        let value = config
            .get_any::<u16>(&["server.port", "PORT"])
            .expect("alias value should parse");
        let optional = config
            .get_optional_any::<u16>(&["server.port", "PORT"])
            .expect("optional alias value should parse");
        let missing = config.get_any::<u16>(&["server.host", "HOST"]);

        assert_eq!(value, 8080);
        assert_eq!(optional, Some(8080));
        assert!(matches!(missing, Err(ConfigError::PropertyNotFound(_))));
    }

    #[test]
    fn test_get_any_or_returns_error_for_present_invalid_value_before_default() {
        let mut config = Config::new();
        config
            .set("feature.enabled", "maybe")
            .expect("setting test config should succeed");
        config
            .set("FEATURE_ENABLED", "true")
            .expect("setting fallback config should succeed");

        let result = config
            .with_read_options(ConfigReadOptions::env_friendly())
            .get_any_or::<bool>(&["feature.enabled", "FEATURE_ENABLED"], false);

        assert!(matches!(
            result,
            Err(ConfigError::ConversionError { key, .. }) if key == "feature.enabled"
        ));
    }

    #[test]
    fn test_get_any_or_with_uses_explicit_read_options() {
        let mut config = Config::new();
        config
            .set("server.port", "   ")
            .expect("setting blank value should succeed");

        let value = config
            .get_any_or_with::<u16>(
                &["server.port", "SERVER_PORT"],
                8080,
                &ConfigReadOptions::default()
                    .with_blank_string_policy(BlankStringPolicy::TreatAsMissing),
            )
            .expect("blank value should use default with explicit options");

        assert_eq!(value, 8080);
    }

    #[test]
    fn test_get_reports_missing_for_blank_string_when_policy_treats_missing() {
        let mut config = Config::new();
        config
            .set("server.host", "   ")
            .expect("setting blank value should succeed");

        let result = config
            .with_read_options(
                ConfigReadOptions::default()
                    .with_blank_string_policy(BlankStringPolicy::TreatAsMissing),
            )
            .get::<String>("server.host");

        assert!(matches!(
            result,
            Err(ConfigError::PropertyHasNoValue(key)) if key == "server.host"
        ));
    }

    #[test]
    fn test_get_optional_reports_rejected_blank_string() {
        let mut config = Config::new();
        config
            .set("server.host", "   ")
            .expect("setting blank value should succeed");

        let result = config
            .with_read_options(
                ConfigReadOptions::default().with_blank_string_policy(BlankStringPolicy::Reject),
            )
            .get_optional::<String>("server.host");

        assert!(matches!(
            result,
            Err(ConfigError::ConversionError { key, message })
                if key == "server.host" && message == "blank string is not allowed"
        ));
    }

    #[test]
    fn test_optional_string_uses_same_missing_semantics_as_optional_get() {
        let mut config = Config::new();
        config
            .set_read_options(
                ConfigReadOptions::default()
                    .with_blank_string_policy(BlankStringPolicy::TreatAsMissing),
            )
            .set("svc.name", "   ")
            .expect("setting blank value should succeed");

        assert_eq!(config.get_optional::<String>("svc.name").unwrap(), None);
        assert_eq!(config.get_optional_string("svc.name").unwrap(), None);

        let svc = config.prefix_view("svc");
        assert_eq!(svc.get_optional::<String>("name").unwrap(), None);
        assert_eq!(svc.get_optional_string("name").unwrap(), None);
    }

    #[test]
    fn test_get_or_returns_error_for_present_invalid_value() {
        let mut config = Config::new();
        config
            .set("server.port", "invalid")
            .expect("setting invalid value should succeed");

        let result = config.get_or::<u16>("server.port", 8080);

        assert!(matches!(
            result,
            Err(ConfigError::ConversionError { key, .. }) if key == "server.port"
        ));
    }

    #[test]
    fn test_read_optional_uses_alias_and_default() {
        let mut config = Config::new();
        config
            .set("APP_PORT", "8080")
            .expect("setting alias value should succeed");

        let alias_value = config
            .read_optional(
                ConfigField::<u16>::builder()
                    .name("server.port")
                    .alias("APP_PORT")
                    .build(),
            )
            .expect("optional alias field should parse");
        let default_value = config
            .read_optional(
                ConfigField::<u16>::builder()
                    .name("server.timeout")
                    .default(30)
                    .build(),
            )
            .expect("optional field default should parse");

        assert_eq!(alias_value, Some(8080));
        assert_eq!(default_value, Some(30));
    }

    #[test]
    fn test_read_uses_global_options_and_reports_missing_field() {
        let mut config = Config::new();
        config
            .set_read_options(ConfigReadOptions::env_friendly())
            .set("FEATURE_ENABLED", "yes")
            .expect("setting alias value should succeed");

        let enabled = config
            .read(
                ConfigField::<bool>::builder()
                    .name("feature.enabled")
                    .alias("FEATURE_ENABLED")
                    .build(),
            )
            .expect("field should use global read options");
        let missing = config.read(
            ConfigField::<u16>::builder()
                .name("server.port")
                .alias("PORT")
                .build(),
        );

        assert!(enabled);
        assert!(matches!(missing, Err(ConfigError::PropertyNotFound(_))));
    }

    #[test]
    fn test_get_string_any_or_applies_alias_order_and_substitution() {
        let mut config = Config::new();
        config
            .set("host", "localhost")
            .expect("setting host should succeed");
        config
            .set("SERVICE_URL", "http://${host}:8080")
            .expect("setting alias value should succeed");

        let value = config
            .get_string_any_or(["service.url", "SERVICE_URL"], "http://fallback")
            .expect("string alias should resolve");
        let fallback = config
            .get_string_any_or(["missing.url", "MISSING_URL"], "http://fallback")
            .expect("missing aliases should use fallback");

        assert_eq!(value, "http://localhost:8080");
        assert_eq!(fallback, "http://fallback");
    }

    #[test]
    fn test_get_string_any_and_optional_string_any_use_alias_order() {
        let mut config = Config::new();
        config
            .set("host", "localhost")
            .expect("setting host should succeed");
        config
            .set("SERVICE_URL", "http://${host}:8080")
            .expect("setting alias value should succeed");

        let value = config
            .get_string_any(["service.url", "SERVICE_URL"])
            .expect("string alias should resolve");
        let optional = config
            .get_optional_string_any(["service.url", "SERVICE_URL"])
            .expect("optional string alias should resolve");
        let missing = config
            .get_optional_string_any(["server.url", "SERVER_URL"])
            .expect("missing optional string aliases should not fail");

        assert_eq!(value, "http://localhost:8080");
        assert_eq!(optional.as_deref(), Some("http://localhost:8080"));
        assert_eq!(missing, None);
    }

    #[test]
    fn test_string_helpers_cover_missing_after_substitution() {
        let mut config = Config::new();
        config
            .set_read_options(
                ConfigReadOptions::default()
                    .with_blank_string_policy(BlankStringPolicy::TreatAsMissing),
            )
            .set("empty.string", "${blank}")
            .expect("setting empty string should succeed");
        config
            .set("empty.list", vec!["${blank}"])
            .expect("setting empty string list should succeed");
        config
            .set("blank", "")
            .expect("setting blank source should succeed");
        config
            .set("fallback", "value")
            .expect("setting fallback should succeed");

        let missing_string = config
            .get_string("empty.string")
            .expect_err("blank substitution should be treated as no value");
        let missing_list = config
            .get_string_list("empty.list")
            .expect_err("blank list substitution should be treated as no value");
        let first_present = config
            .get_string_any(["missing", "empty.string", "fallback"])
            .expect("alias reader should skip effectively missing values");
        let missing_any = config
            .get_string_any(["missing", "also.missing"])
            .expect_err("all missing aliases should report not found");

        assert!(
            matches!(missing_string, ConfigError::PropertyHasNoValue(key) if key == "empty.string")
        );
        assert!(
            matches!(missing_list, ConfigError::PropertyHasNoValue(key) if key == "empty.list")
        );
        assert_eq!(first_present, "value");
        assert!(
            matches!(missing_any, ConfigError::PropertyNotFound(message) if message == "one of: missing, also.missing")
        );
    }

    #[test]
    fn test_get_list_strict_returns_empty_vector_for_null_property() {
        let mut config = Config::new();
        config
            .set_null("empty.list", DataType::String)
            .expect("setting null value should succeed");

        let values = config
            .get_list_strict::<String>("empty.list")
            .expect("null list should parse as empty vector");

        assert!(values.is_empty());
    }

    #[test]
    fn test_get_list_strict_reports_missing_key() {
        let config = Config::new();

        let result = config.get_list_strict::<String>("missing.list");

        assert!(matches!(result, Err(ConfigError::PropertyNotFound(key)) if key == "missing.list"));
    }

    #[test]
    fn test_prefix_view_optional_error_uses_root_relative_key() {
        let mut config = Config::new();
        config
            .set("db.port", "invalid")
            .expect("setting invalid value should succeed");

        let db = config.prefix_view("db");
        let result = db.get_optional::<u16>("port");

        assert!(matches!(
            result,
            Err(ConfigError::ConversionError { key, .. }) if key == "db.port"
        ));
    }
}
