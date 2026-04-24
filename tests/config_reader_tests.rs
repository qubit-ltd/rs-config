/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! [`qubit_config::ConfigReader`] tests.

use qubit_common::DataType;
use qubit_config::{Config, ConfigPrefixView, ConfigReader};
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
    use super::*;

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
    use super::*;

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
            <Config as ConfigReader>::get_string_or(reader, "missing", "fallback"),
            "fallback"
        );
        assert_eq!(
            <Config as ConfigReader>::get_string_list_or(reader, "missing.list", &["x", "y"]),
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
            <Config as ConfigReader>::get_string_or(reader, "url", "fallback"),
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
            <ConfigPrefixView<'_> as ConfigReader>::get_string_or(&view, "missing", "fallback"),
            "fallback"
        );
        assert_eq!(
            <ConfigPrefixView<'_> as ConfigReader>::get_string_list_or(&view, "missing", &["m"]),
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
            <ConfigPrefixView<'_> as ConfigReader>::get_string_or(&view, "url", "fallback"),
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
    use super::*;

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

        assert!(<Config as ConfigReader>::get_property(&config, "k").is_some());
        assert!(<Config as ConfigReader>::get_property(&config, "nullish").is_some());
        assert!(<Config as ConfigReader>::is_null(&config, "nullish"));
        assert!(!<Config as ConfigReader>::is_null(&config, "k"));
        assert!(!<Config as ConfigReader>::is_null(&config, "missing"));

        assert_eq!(<Config as ConfigReader>::len(&config), 2);
        assert!(!<Config as ConfigReader>::is_empty(&config));

        let mut keys = <Config as ConfigReader>::keys(&config);
        keys.sort();
        assert_eq!(keys, vec!["k".to_string(), "nullish".to_string()]);

        assert_eq!(<Config as ConfigReader>::iter(&config).count(), 2);

        assert_eq!(<Config as ConfigReader>::get_or(&config, "k", 0i32), 1);
        assert_eq!(
            <Config as ConfigReader>::get_or(&config, "missing", 99i32),
            99
        );

        assert_eq!(
            <Config as ConfigReader>::get_optional_string(&config, "nullish").unwrap(),
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
