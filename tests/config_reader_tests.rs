/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! [`qubit_config::ConfigReader`] tests.

use qubit_config::{Config, ConfigPrefixView, ConfigReader};

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
