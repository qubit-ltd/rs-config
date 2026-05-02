/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! [`qubit_config::ConfigPrefixView`] tests.

use qubit_config::{Config, ConfigReader};

#[cfg(test)]
mod test_config_prefix_view_smoke {
    #[allow(unused_imports)]
    use super::{Config, ConfigReader};

    #[test]
    fn config_prefix_view_reads_relative_key() {
        let mut c = Config::new();
        c.set("http.host", "localhost").unwrap();
        let v = c.prefix_view("http");
        assert_eq!(v.get::<String>("host").unwrap(), "localhost");
    }
}

#[cfg(test)]
mod test_config_prefix_view {
    #[allow(unused_imports)]
    use super::{Config, ConfigReader};

    #[test]
    fn test_view_reads_relative_keys_without_copy() {
        let mut config = Config::new();
        config.set("http.host", "localhost").unwrap();
        config.set("http.port", 8080).unwrap();
        config.set("db.host", "db").unwrap();

        let view = config.prefix_view("http");
        assert!(view.contains("host"));
        assert!(view.contains("port"));
        assert!(!view.contains("db.host"));
        let host: String = view.get("host").unwrap();
        let port: i32 = view.get("port").unwrap();
        assert_eq!(host, "localhost");
        assert_eq!(port, 8080);
    }

    #[test]
    fn test_view_nested_view() {
        let mut config = Config::new();
        config.set("http.proxy.host", "proxy.example.com").unwrap();
        config.set("http.proxy.port", 3128).unwrap();
        config.set("http.timeout", 30).unwrap();

        let proxy = config.prefix_view("http").prefix_view("proxy");
        assert_eq!(proxy.prefix(), "http.proxy");
        let host: String = proxy.get("host").unwrap();
        let port: i32 = proxy.get("port").unwrap();
        assert_eq!(host, "proxy.example.com");
        assert_eq!(port, 3128);
        assert!(!proxy.contains("timeout"));
    }

    #[test]
    fn test_view_variable_substitution_uses_view_scope() {
        let mut config = Config::new();
        config.set("http.host", "localhost").unwrap();
        config.set("http.port", "8080").unwrap();
        config
            .set("http.base_url", "http://${host}:${port}")
            .unwrap();

        let view = config.prefix_view("http");
        let base_url = view.get_string("base_url").unwrap();
        assert_eq!(base_url, "http://localhost:8080");
    }

    #[test]
    fn test_view_contains_prefix_and_iter_prefix() {
        let mut config = Config::new();
        config.set("http.proxy.host", "proxy").unwrap();
        config.set("http.proxy.port", 3128).unwrap();
        config.set("http.timeout", 30).unwrap();

        let view = config.prefix_view("http");
        assert!(view.contains_prefix("proxy"));
        assert!(!view.contains_prefix("db"));

        let keys: Vec<&str> = view.iter_prefix("proxy.").map(|(k, _)| k).collect();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"proxy.host"));
        assert!(keys.contains(&"proxy.port"));
    }

    #[test]
    fn test_view_exact_prefix_key_compatibility() {
        let mut config = Config::new();
        config.set("http", "root-value").unwrap();
        config.set("http.host", "localhost").unwrap();

        let view = config.prefix_view("http");
        assert!(view.contains("http"));
        assert_eq!(view.get_string("http").unwrap(), "root-value");
        assert_eq!(view.get_string("host").unwrap(), "localhost");
    }

    #[test]
    fn test_view_empty_prefix_behaves_like_root() {
        let mut config = Config::new();
        config.set("host", "localhost").unwrap();
        config.set("port", 8080).unwrap();

        let root_view = config.prefix_view("");
        assert_eq!(root_view.get_string("host").unwrap(), "localhost");
        let port: i32 = root_view.get("port").unwrap();
        assert_eq!(port, 8080);
    }
    #[test]
    fn test_view_branch_coverage_for_nested_building_and_key_resolution() {
        let mut config = Config::new();
        config.set("http.host", "localhost").unwrap();
        config.set("http.full", "http://${http.host}").unwrap();
        config.set("http", "root").unwrap();

        let from_root = config.prefix_view("").prefix_view("http");
        assert_eq!(from_root.get_string("host").unwrap(), "localhost");
        let same = config.prefix_view("http").prefix_view("");
        assert_eq!(same.prefix(), "http");
        assert_eq!(same.get_string("http.host").unwrap(), "localhost");

        let all_keys: Vec<&str> = same.iter_prefix("").map(|(k, _)| k).collect();
        assert!(all_keys.contains(&"http"));
        assert!(all_keys.contains(&"host"));
    }

    #[test]
    fn test_root_view_iter_prefix_covers_empty_prefix_visible_entries_branch() {
        let mut config = Config::new();
        config.set("alpha", "a").unwrap();
        config.set("beta", "b").unwrap();

        let root_view = config.prefix_view("");
        let keys: Vec<&str> = root_view.iter_prefix("").map(|(k, _)| k).collect();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"alpha"));
        assert!(keys.contains(&"beta"));
    }
}
