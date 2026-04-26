/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! # Configurable Trait Tests
//!
//! Tests for the Configurable trait implementation.
//!
//! # Author
//!
//! Haixing Hu

use qubit_config::{Config, Configurable};

// Test implementation of Configurable trait
struct TestConfigurable {
    config: Config,
    changed_count: usize,
}

impl TestConfigurable {
    fn new() -> Self {
        Self {
            config: Config::new(),
            changed_count: 0,
        }
    }

    fn changed_count(&self) -> usize {
        self.changed_count
    }
}

impl Configurable for TestConfigurable {
    fn config(&self) -> &Config {
        &self.config
    }

    fn config_mut(&mut self) -> &mut Config {
        &mut self.config
    }

    fn set_config(&mut self, config: Config) {
        self.config = config;
        self.on_config_changed();
    }

    fn on_config_changed(&mut self) {
        self.changed_count += 1;
    }
}

#[cfg(test)]
mod test_config {
    use super::*;

    #[test]
    fn test_config_returns_reference() {
        let mut obj = TestConfigurable::new();
        obj.config_mut().set("key", "value").unwrap();

        let config = obj.config();
        let value: String = config.get("key").unwrap();
        assert_eq!(value, "value");
    }

    #[test]
    fn test_config_returns_immutable_reference() {
        let mut obj = TestConfigurable::new();
        obj.config_mut().set("port", 8080).unwrap();

        let config = obj.config();
        let port: i32 = config.get("port").unwrap();
        assert_eq!(port, 8080);
    }

    #[test]
    fn test_config_with_multiple_properties() {
        let mut obj = TestConfigurable::new();
        obj.config_mut().set("host", "localhost").unwrap();
        obj.config_mut().set("port", 3000).unwrap();
        obj.config_mut().set("debug", true).unwrap();

        let config = obj.config();
        assert_eq!(config.get::<String>("host").unwrap(), "localhost");
        assert_eq!(config.get::<i32>("port").unwrap(), 3000);
        assert!(config.get::<bool>("debug").unwrap());
    }
}

#[cfg(test)]
mod test_config_mut {
    use super::*;

    #[test]
    fn test_config_mut_returns_mutable_reference() {
        let mut obj = TestConfigurable::new();

        obj.config_mut().set("key", "value").unwrap();

        let value: String = obj.config().get("key").unwrap();
        assert_eq!(value, "value");
    }

    #[test]
    fn test_config_mut_allows_modification() {
        let mut obj = TestConfigurable::new();

        obj.config_mut().set("counter", 10).unwrap();
        obj.config_mut().set("counter", 20).unwrap();

        let counter: i32 = obj.config().get("counter").unwrap();
        assert_eq!(counter, 20);
    }

    #[test]
    fn test_config_mut_allows_adding_properties() {
        let mut obj = TestConfigurable::new();

        obj.config_mut().set("prop1", "value1").unwrap();
        obj.config_mut().set("prop2", "value2").unwrap();
        obj.config_mut().set("prop3", "value3").unwrap();

        assert_eq!(obj.config().len(), 3);
    }

    #[test]
    fn test_config_mut_allows_removing_properties() {
        let mut obj = TestConfigurable::new();

        obj.config_mut().set("key1", "value1").unwrap();
        obj.config_mut().set("key2", "value2").unwrap();

        obj.config_mut().remove("key1").unwrap();

        assert_eq!(obj.config().len(), 1);
        assert!(obj.config().contains("key2"));
        assert!(!obj.config().contains("key1"));
    }
}

#[cfg(test)]
mod test_set_config {
    use super::*;

    #[test]
    fn test_set_config_replaces_config() {
        let mut obj = TestConfigurable::new();
        obj.config_mut().set("old_key", "old_value").unwrap();

        let mut new_config = Config::new();
        new_config.set("new_key", "new_value").unwrap();

        obj.set_config(new_config);

        assert!(!obj.config().contains("old_key"));
        assert!(obj.config().contains("new_key"));
        assert_eq!(obj.config().get::<String>("new_key").unwrap(), "new_value");
    }

    #[test]
    fn test_set_config_with_empty_config() {
        let mut obj = TestConfigurable::new();
        obj.config_mut().set("key", "value").unwrap();

        obj.set_config(Config::new());

        assert_eq!(obj.config().len(), 0);
        assert!(obj.config().is_empty());
    }

    #[test]
    fn test_set_config_triggers_on_config_changed() {
        let mut obj = TestConfigurable::new();

        assert_eq!(obj.changed_count(), 0);

        obj.set_config(Config::new());

        assert_eq!(obj.changed_count(), 1);
    }

    #[test]
    fn test_set_config_multiple_times() {
        let mut obj = TestConfigurable::new();

        for i in 0..5 {
            let mut config = Config::new();
            config.set("iteration", i).unwrap();
            obj.set_config(config);
        }

        assert_eq!(obj.changed_count(), 5);
        assert_eq!(obj.config().get::<i32>("iteration").unwrap(), 4);
    }

    #[test]
    fn test_set_config_with_populated_config() {
        let mut obj = TestConfigurable::new();

        let mut new_config = Config::new();
        new_config.set("host", "127.0.0.1").unwrap();
        new_config.set("port", 9000).unwrap();
        new_config.set("timeout", 30).unwrap();

        obj.set_config(new_config);

        assert_eq!(obj.config().len(), 3);
        assert_eq!(obj.config().get::<String>("host").unwrap(), "127.0.0.1");
        assert_eq!(obj.config().get::<i32>("port").unwrap(), 9000);
        assert_eq!(obj.config().get::<i32>("timeout").unwrap(), 30);
    }
}

#[cfg(test)]
mod test_on_config_changed {
    use super::*;

    #[test]
    fn test_on_config_changed_default_implementation() {
        struct SimpleConfigurable {
            config: Config,
        }

        impl Configurable for SimpleConfigurable {
            fn config(&self) -> &Config {
                &self.config
            }

            fn config_mut(&mut self) -> &mut Config {
                &mut self.config
            }

            fn set_config(&mut self, config: Config) {
                self.config = config;
                self.on_config_changed(); // Should do nothing by default
            }
        }

        let mut obj = SimpleConfigurable {
            config: Config::new(),
        };

        // Should not panic
        obj.set_config(Config::new());
    }

    #[test]
    fn test_on_config_changed_called_by_set_config() {
        let mut obj = TestConfigurable::new();

        assert_eq!(obj.changed_count(), 0);

        obj.set_config(Config::new());
        assert_eq!(obj.changed_count(), 1);

        obj.set_config(Config::new());
        assert_eq!(obj.changed_count(), 2);
    }

    #[test]
    fn test_on_config_changed_custom_implementation() {
        struct CustomConfigurable {
            config: Config,
            validation_called: bool,
        }

        impl CustomConfigurable {
            fn new() -> Self {
                Self {
                    config: Config::new(),
                    validation_called: false,
                }
            }

            fn is_validation_called(&self) -> bool {
                self.validation_called
            }
        }

        impl Configurable for CustomConfigurable {
            fn config(&self) -> &Config {
                &self.config
            }

            fn config_mut(&mut self) -> &mut Config {
                &mut self.config
            }

            fn set_config(&mut self, config: Config) {
                self.config = config;
                self.on_config_changed();
            }

            fn on_config_changed(&mut self) {
                self.validation_called = true;
            }
        }

        let mut obj = CustomConfigurable::new();
        assert!(!obj.is_validation_called());

        obj.set_config(Config::new());
        assert!(obj.is_validation_called());
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_full_configurable_workflow() {
        let mut obj = TestConfigurable::new();

        // Initial state
        assert!(obj.config().is_empty());
        assert_eq!(obj.changed_count(), 0);

        // Modify through config_mut
        obj.config_mut().set("app_name", "test_app").unwrap();
        obj.config_mut().set("version", "1.0.0").unwrap();

        // Verify changes
        assert_eq!(obj.config().len(), 2);
        assert_eq!(obj.changed_count(), 0); // config_mut doesn't trigger callback

        // Replace entire config
        let mut new_config = Config::new();
        new_config.set("app_name", "new_app").unwrap();
        new_config.set("version", "2.0.0").unwrap();
        new_config.set("env", "production").unwrap();

        obj.set_config(new_config);

        // Verify replacement
        assert_eq!(obj.config().len(), 3);
        assert_eq!(obj.config().get::<String>("app_name").unwrap(), "new_app");
        assert_eq!(obj.config().get::<String>("version").unwrap(), "2.0.0");
        assert_eq!(obj.config().get::<String>("env").unwrap(), "production");
        assert_eq!(obj.changed_count(), 1);
    }

    #[test]
    fn test_configurable_with_different_data_types() {
        let mut obj = TestConfigurable::new();

        obj.config_mut().set("string_val", "hello").unwrap();
        obj.config_mut().set("int_val", 42).unwrap();
        obj.config_mut().set("float_val", 3.15).unwrap();
        obj.config_mut().set("bool_val", true).unwrap();

        assert_eq!(obj.config().get::<String>("string_val").unwrap(), "hello");
        assert_eq!(obj.config().get::<i32>("int_val").unwrap(), 42);
        assert_eq!(obj.config().get::<f64>("float_val").unwrap(), 3.15);
        assert!(obj.config().get::<bool>("bool_val").unwrap());
    }

    #[test]
    fn test_configurable_with_vectors() {
        let mut obj = TestConfigurable::new();

        obj.config_mut()
            .set("ports", vec![8080, 8081, 8082])
            .unwrap();
        obj.config_mut()
            .set("hosts", vec!["host1", "host2", "host3"])
            .unwrap();

        let ports: Vec<i32> = obj.config().get_list("ports").unwrap();
        let hosts: Vec<String> = obj.config().get_list("hosts").unwrap();

        assert_eq!(ports, vec![8080, 8081, 8082]);
        assert_eq!(hosts, vec!["host1", "host2", "host3"]);
    }
}
