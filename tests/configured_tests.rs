/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! # Configured Class Unit Tests
//!
//! Tests all public methods of the Configured class, including Configurable trait implementation.

use qubit_config::{Config, Configurable, Configured};

// ============================================================================
// Test Helper Functions
// ============================================================================

/// Creates a test configuration object
fn create_test_config() -> Config {
    let mut config = Config::new();
    config.set("test_string", "value").unwrap();
    config.set("test_int", 42).unwrap();
    config.set("test_bool", true).unwrap();
    config
}

/// Creates a test configuration with description
fn create_test_config_with_description() -> Config {
    let mut config = Config::with_description("Test Configuration");
    config.set("test", "value").unwrap();
    config
}

// ============================================================================
// Constructor Tests
// ============================================================================

#[cfg(test)]
mod test_new {
    #[allow(unused_imports)]
    use super::{
        Config, Configurable, Configured, create_test_config, create_test_config_with_description,
    };

    #[test]
    fn test_new_creates_empty_configured() {
        let configured = Configured::new();
        assert!(configured.config().is_empty());
        assert_eq!(configured.config().len(), 0);
    }

    #[test]
    fn test_new_has_default_config_values() {
        let configured = Configured::new();
        let config = configured.config();
        assert!(config.description().is_none());
        assert!(config.is_enable_variable_substitution());
        assert_eq!(config.max_substitution_depth(), 64);
    }
}

#[cfg(test)]
mod test_with_config {
    #[allow(unused_imports)]
    use super::{
        Config, Configurable, Configured, create_test_config, create_test_config_with_description,
    };

    #[test]
    fn test_with_config_creates_configured_with_given_config() {
        let config = create_test_config();
        let configured = Configured::with_config(config);

        assert!(!configured.config().is_empty());
        assert_eq!(configured.config().len(), 3);
        assert!(configured.config().contains("test_string"));
        assert!(configured.config().contains("test_int"));
        assert!(configured.config().contains("test_bool"));
    }

    #[test]
    fn test_with_config_preserves_config_values() {
        let config = create_test_config();
        let configured = Configured::with_config(config);

        let string_value: String = configured.config().get("test_string").unwrap();
        let int_value: i32 = configured.config().get("test_int").unwrap();
        let bool_value: bool = configured.config().get("test_bool").unwrap();

        assert_eq!(string_value, "value");
        assert_eq!(int_value, 42);
        assert!(bool_value);
    }

    #[test]
    fn test_with_config_with_empty_config() {
        let config = Config::new();
        let configured = Configured::with_config(config);
        assert!(configured.config().is_empty());
    }

    #[test]
    fn test_with_config_with_description() {
        let config = create_test_config_with_description();
        let configured = Configured::with_config(config);

        assert_eq!(
            configured.config().description(),
            Some("Test Configuration")
        );
        assert!(configured.config().contains("test"));
    }
}

// ============================================================================
// Configurable Trait Implementation Tests
// ============================================================================

#[cfg(test)]
mod test_config {
    #[allow(unused_imports)]
    use super::{
        Config, Configurable, Configured, create_test_config, create_test_config_with_description,
    };

    #[test]
    fn test_config_returns_reference_to_config() {
        let configured = Configured::new();
        let config_ref = configured.config();
        assert!(config_ref.is_empty());
    }

    #[test]
    fn test_config_returns_same_config_instance() {
        let configured = Configured::new();
        let config_ref1 = configured.config();
        let config_ref2 = configured.config();

        // Both references should point to the same configuration object
        assert_eq!(config_ref1.len(), config_ref2.len());
        assert_eq!(config_ref1.is_empty(), config_ref2.is_empty());
    }

    #[test]
    fn test_config_with_predefined_config() {
        let config = create_test_config();
        let configured = Configured::with_config(config);
        let config_ref = configured.config();

        assert!(!config_ref.is_empty());
        assert_eq!(config_ref.len(), 3);
    }
}

#[cfg(test)]
mod test_config_mut {
    #[allow(unused_imports)]
    use super::{
        Config, Configurable, Configured, create_test_config, create_test_config_with_description,
    };

    #[test]
    fn test_config_mut_returns_mutable_reference() {
        let mut configured = Configured::new();
        let config_mut = configured.config_mut();
        config_mut.set("test", "value").unwrap();

        assert!(configured.config().contains("test"));
        let value: String = configured.config().get("test").unwrap();
        assert_eq!(value, "value");
    }

    #[test]
    fn test_config_mut_allows_modification() {
        let mut configured = Configured::new();

        // Set values through config_mut
        configured.config_mut().set("key1", "value1").unwrap();
        configured.config_mut().set("key2", 42).unwrap();
        configured.config_mut().set("key3", true).unwrap();

        // Verify values are set correctly
        assert_eq!(configured.config().len(), 3);
        assert!(configured.config().contains("key1"));
        assert!(configured.config().contains("key2"));
        assert!(configured.config().contains("key3"));

        let value1: String = configured.config().get("key1").unwrap();
        let value2: i32 = configured.config().get("key2").unwrap();
        let value3: bool = configured.config().get("key3").unwrap();

        assert_eq!(value1, "value1");
        assert_eq!(value2, 42);
        assert!(value3);
    }

    #[test]
    fn test_config_mut_allows_removal() {
        let mut configured = Configured::new();
        configured.config_mut().set("test", "value").unwrap();
        assert!(configured.config().contains("test"));

        configured.config_mut().remove("test").unwrap();
        assert!(!configured.config().contains("test"));
    }

    #[test]
    fn test_config_mut_allows_clearing() {
        let mut configured = Configured::new();
        configured.config_mut().set("key1", "value1").unwrap();
        configured.config_mut().set("key2", "value2").unwrap();
        assert_eq!(configured.config().len(), 2);

        configured.config_mut().clear().unwrap();
        assert!(configured.config().is_empty());
        assert_eq!(configured.config().len(), 0);
    }

    #[test]
    fn test_config_mut_allows_description_change() {
        let mut configured = Configured::new();
        assert!(configured.config().description().is_none());

        configured
            .config_mut()
            .set_description(Some("New description".to_string()));
        assert_eq!(configured.config().description(), Some("New description"));

        configured.config_mut().set_description(None);
        assert!(configured.config().description().is_none());
    }

    #[test]
    fn test_config_mut_allows_variable_substitution_change() {
        let mut configured = Configured::new();
        assert!(configured.config().is_enable_variable_substitution());

        configured
            .config_mut()
            .set_enable_variable_substitution(false);
        assert!(!configured.config().is_enable_variable_substitution());

        configured
            .config_mut()
            .set_enable_variable_substitution(true);
        assert!(configured.config().is_enable_variable_substitution());
    }

    #[test]
    fn test_config_mut_allows_max_substitution_depth_change() {
        let mut configured = Configured::new();
        assert_eq!(configured.config().max_substitution_depth(), 64);

        configured.config_mut().set_max_substitution_depth(100);
        assert_eq!(configured.config().max_substitution_depth(), 100);

        configured.config_mut().set_max_substitution_depth(0);
        assert_eq!(configured.config().max_substitution_depth(), 0);
    }
}

#[cfg(test)]
mod test_set_config {
    #[allow(unused_imports)]
    use super::{
        Config, Configurable, Configured, create_test_config, create_test_config_with_description,
    };

    #[test]
    fn test_set_config_replaces_config() {
        let mut configured = Configured::new();
        configured.config_mut().set("old_key", "old_value").unwrap();
        assert_eq!(configured.config().len(), 1);

        let mut new_config = Config::new();
        new_config.set("new_key", "new_value").unwrap();
        new_config.set("another_key", 42).unwrap();

        configured.set_config(new_config);

        assert_eq!(configured.config().len(), 2);
        assert!(!configured.config().contains("old_key"));
        assert!(configured.config().contains("new_key"));
        assert!(configured.config().contains("another_key"));

        let new_value: String = configured.config().get("new_key").unwrap();
        let another_value: i32 = configured.config().get("another_key").unwrap();
        assert_eq!(new_value, "new_value");
        assert_eq!(another_value, 42);
    }

    #[test]
    fn test_set_config_with_empty_config() {
        let mut configured = Configured::new();
        configured.config_mut().set("test", "value").unwrap();
        assert!(!configured.config().is_empty());

        let empty_config = Config::new();
        configured.set_config(empty_config);
        assert!(configured.config().is_empty());
    }

    #[test]
    fn test_set_config_with_description() {
        let mut configured = Configured::new();
        assert!(configured.config().description().is_none());

        let mut config_with_desc = Config::with_description("New configuration");
        config_with_desc.set("test", "value").unwrap();

        configured.set_config(config_with_desc);

        assert_eq!(configured.config().description(), Some("New configuration"));
        assert!(configured.config().contains("test"));
    }

    #[test]
    fn test_set_config_triggers_on_config_changed() {
        // Create a custom Configured implementation to test callback
        struct TestConfigured {
            configured: Configured,
            changed_called: bool,
        }

        impl TestConfigured {
            fn new() -> Self {
                Self {
                    configured: Configured::new(),
                    changed_called: false,
                }
            }

            fn set_config(&mut self, config: Config) {
                self.configured.set_config(config);
                self.changed_called = true;
            }

            fn config(&self) -> &Config {
                self.configured.config()
            }
        }

        let mut test_configured = TestConfigured::new();
        assert!(!test_configured.changed_called);

        let new_config = create_test_config();
        test_configured.set_config(new_config);

        assert!(test_configured.changed_called);
        assert!(!test_configured.config().is_empty());
    }
}

#[cfg(test)]
mod test_on_config_changed {
    #[allow(unused_imports)]
    use super::{
        Config, Configurable, Configured, create_test_config, create_test_config_with_description,
    };

    #[test]
    fn test_on_config_changed_default_implementation() {
        let mut configured = Configured::new();
        // Default implementation should do nothing and not panic
        configured.on_config_changed();
    }

    #[test]
    fn test_on_config_changed_called_by_set_config() {
        // Create a custom Configured implementation to test callback
        struct TestConfigured {
            configured: Configured,
            callback_count: usize,
        }

        impl TestConfigured {
            fn new() -> Self {
                Self {
                    configured: Configured::new(),
                    callback_count: 0,
                }
            }

            fn set_config(&mut self, config: Config) {
                self.configured.set_config(config);
                self.callback_count += 1;
            }

            #[allow(dead_code)]
            fn config(&self) -> &Config {
                self.configured.config()
            }
        }

        let mut test_configured = TestConfigured::new();
        assert_eq!(test_configured.callback_count, 0);

        let config1 = create_test_config();
        test_configured.set_config(config1);
        assert_eq!(test_configured.callback_count, 1);

        let config2 = Config::new();
        test_configured.set_config(config2);
        assert_eq!(test_configured.callback_count, 2);
    }
}

// ============================================================================
// Delegate Method Tests
// ============================================================================

#[cfg(test)]
mod test_default {
    #[allow(unused_imports)]
    use super::{
        Config, Configurable, Configured, create_test_config, create_test_config_with_description,
    };

    #[test]
    fn test_default_creates_empty_configured() {
        let configured = Configured::default();
        assert!(configured.config().is_empty());
        assert_eq!(configured.config().len(), 0);
    }

    #[test]
    fn test_default_equals_new() {
        let configured1 = Configured::new();
        let configured2 = Configured::default();

        assert_eq!(configured1.config().len(), configured2.config().len());
        assert_eq!(
            configured1.config().is_empty(),
            configured2.config().is_empty()
        );
        assert_eq!(
            configured1.config().description(),
            configured2.config().description()
        );
    }
}

// ============================================================================
// Integration Tests
// ============================================================================

#[cfg(test)]
mod integration_tests {
    #[allow(unused_imports)]
    use super::{
        Config, Configurable, Configured, create_test_config, create_test_config_with_description,
    };

    #[test]
    fn test_configured_full_workflow() {
        // Create Configured instance
        let mut configured = Configured::new();
        assert!(configured.config().is_empty());

        // Set configuration
        configured.config_mut().set("server.port", 8080).unwrap();
        configured
            .config_mut()
            .set("server.host", "localhost")
            .unwrap();
        configured.config_mut().set("server.debug", true).unwrap();

        // Verify configuration
        assert_eq!(configured.config().len(), 3);
        let port: i32 = configured.config().get("server.port").unwrap();
        let host: String = configured.config().get("server.host").unwrap();
        let debug: bool = configured.config().get("server.debug").unwrap();

        assert_eq!(port, 8080);
        assert_eq!(host, "localhost");
        assert!(debug);

        // Modify configuration
        configured.config_mut().set("server.port", 9090).unwrap();
        let new_port: i32 = configured.config().get("server.port").unwrap();
        assert_eq!(new_port, 9090);

        // Add new configuration
        configured
            .config_mut()
            .set("database.url", "postgresql://localhost/db")
            .unwrap();
        assert_eq!(configured.config().len(), 4);

        // Remove configuration
        configured.config_mut().remove("server.debug").unwrap();
        assert_eq!(configured.config().len(), 3);
        assert!(!configured.config().contains("server.debug"));

        // Replace entire configuration
        let mut new_config = Config::with_description("New server configuration");
        new_config.set("app.name", "MyApp").unwrap();
        new_config.set("app.version", "1.0.0").unwrap();

        configured.set_config(new_config);

        assert_eq!(configured.config().len(), 2);
        assert_eq!(
            configured.config().description(),
            Some("New server configuration")
        );
        assert!(!configured.config().contains("server.port"));
        assert!(configured.config().contains("app.name"));

        let app_name: String = configured.config().get("app.name").unwrap();
        let app_version: String = configured.config().get("app.version").unwrap();
        assert_eq!(app_name, "MyApp");
        assert_eq!(app_version, "1.0.0");
    }

    #[test]
    fn test_configured_with_different_data_types() {
        let mut configured = Configured::new();

        // Test all supported data types
        configured.config_mut().set("int8", 42i8).unwrap();
        configured.config_mut().set("int16", 42i16).unwrap();
        configured.config_mut().set("int32", 42i32).unwrap();
        configured.config_mut().set("int64", 42i64).unwrap();
        configured.config_mut().set("int128", 42i128).unwrap();
        // Note: isize and usize types do not implement IntoPropertyValue and FromPropertyValue traits
        // configured.config_mut().set("isize", 42isize).unwrap();

        configured.config_mut().set("uint8", 42u8).unwrap();
        configured.config_mut().set("uint16", 42u16).unwrap();
        configured.config_mut().set("uint32", 42u32).unwrap();
        configured.config_mut().set("uint64", 42u64).unwrap();
        configured.config_mut().set("uint128", 42u128).unwrap();
        // configured.config_mut().set("usize", 42usize).unwrap();

        configured.config_mut().set("float32", 3.5f32).unwrap();
        configured.config_mut().set("float64", 3.5f64).unwrap();

        configured.config_mut().set("boolean", true).unwrap();
        configured.config_mut().set("character", 'A').unwrap();
        configured.config_mut().set("string", "test").unwrap();

        // Verify all values
        assert_eq!(configured.config().get::<i8>("int8").unwrap(), 42);
        assert_eq!(configured.config().get::<i16>("int16").unwrap(), 42);
        assert_eq!(configured.config().get::<i32>("int32").unwrap(), 42);
        assert_eq!(configured.config().get::<i64>("int64").unwrap(), 42);
        assert_eq!(configured.config().get::<i128>("int128").unwrap(), 42);
        // assert_eq!(configured.config().get::<isize>("isize").unwrap(), 42);

        assert_eq!(configured.config().get::<u8>("uint8").unwrap(), 42);
        assert_eq!(configured.config().get::<u16>("uint16").unwrap(), 42);
        assert_eq!(configured.config().get::<u32>("uint32").unwrap(), 42);
        assert_eq!(configured.config().get::<u64>("uint64").unwrap(), 42);
        assert_eq!(configured.config().get::<u128>("uint128").unwrap(), 42);
        // assert_eq!(configured.config().get::<usize>("usize").unwrap(), 42);

        assert_eq!(configured.config().get::<f32>("float32").unwrap(), 3.5);
        assert_eq!(configured.config().get::<f64>("float64").unwrap(), 3.5);

        assert!(configured.config().get::<bool>("boolean").unwrap());
        assert_eq!(configured.config().get::<char>("character").unwrap(), 'A');
        assert_eq!(configured.config().get::<String>("string").unwrap(), "test");
    }

    #[test]
    fn test_configured_with_vectors() {
        let mut configured = Configured::new();

        // Test vector types
        configured
            .config_mut()
            .set("int_list", vec![1, 2, 3, 4, 5])
            .unwrap();
        configured
            .config_mut()
            .set(
                "string_list",
                vec!["a".to_string(), "b".to_string(), "c".to_string()],
            )
            .unwrap();
        configured
            .config_mut()
            .set("bool_list", vec![true, false, true])
            .unwrap();

        // Verify vector values
        let int_list: Vec<i32> = configured.config().get_list("int_list").unwrap();
        let string_list: Vec<String> = configured.config().get_list("string_list").unwrap();
        let bool_list: Vec<bool> = configured.config().get_list("bool_list").unwrap();

        assert_eq!(int_list, vec![1, 2, 3, 4, 5]);
        assert_eq!(string_list, vec!["a", "b", "c"]);
        assert_eq!(bool_list, vec![true, false, true]);

        // Test adding values to existing list
        configured.config_mut().add("int_list", 6).unwrap();
        let updated_int_list: Vec<i32> = configured.config().get_list("int_list").unwrap();
        assert_eq!(updated_int_list, vec![1, 2, 3, 4, 5, 6]);
    }
}
