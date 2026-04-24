/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! # Configuration Utility Function Tests
//!
//! Integration tests for deserialize JSON building (`property_to_json_value` /
//! dotted-key insertion). Variable substitution is covered by unit tests in
//! `src/utils.rs`.
//!
//! # Author
//!
//! Haixing Hu

use qubit_common::DataType;
use qubit_config::{Config, ConfigError, Property};
use qubit_value::MultiValues;
use serde::Deserialize;
use std::collections::HashMap;

// ============================================================================
// Config::deserialize() (utils: property_to_json_value, insert_deserialize_value)
// ============================================================================

#[cfg(test)]
mod test_deserialize {
    use super::*;

    #[derive(Deserialize, Debug, PartialEq)]
    struct ServerConfig {
        host: String,
        port: i32,
    }

    #[derive(Deserialize, Debug, PartialEq)]
    struct DatabaseConfig {
        url: String,
        pool: i32,
        timeout: Option<i64>,
    }

    #[derive(Deserialize, Debug, PartialEq)]
    struct AppConfig {
        name: String,
        version: String,
        debug: bool,
    }

    #[derive(Deserialize, Debug, PartialEq)]
    struct NestedServerConfig {
        host: String,
        port: i32,
    }

    #[derive(Deserialize, Debug, PartialEq)]
    struct NestedAppConfig {
        server: NestedServerConfig,
    }

    #[derive(Deserialize, Debug, PartialEq)]
    struct WithOptionals {
        host: String,
        port: Option<i32>,
    }

    #[test]
    fn test_deserialize_basic_struct() {
        let mut config = Config::new();
        config.set("server.host", "localhost").unwrap();
        config.set("server.port", 8080).unwrap();

        let server: ServerConfig = config.deserialize("server").unwrap();
        assert_eq!(server.host, "localhost");
        assert_eq!(server.port, 8080);
    }

    #[test]
    fn test_deserialize_with_optional_present() {
        let mut config = Config::new();
        config.set("db.url", "postgres://localhost/mydb").unwrap();
        config.set("db.pool", 10).unwrap();
        config.set("db.timeout", 30i64).unwrap();

        let db: DatabaseConfig = config.deserialize("db").unwrap();
        assert_eq!(db.url, "postgres://localhost/mydb");
        assert_eq!(db.pool, 10);
        assert_eq!(db.timeout, Some(30));
    }

    #[test]
    fn test_deserialize_with_optional_absent() {
        let mut config = Config::new();
        config.set("db.url", "postgres://localhost/mydb").unwrap();
        config.set("db.pool", 10).unwrap();

        let db: DatabaseConfig = config.deserialize("db").unwrap();
        assert_eq!(db.url, "postgres://localhost/mydb");
        assert_eq!(db.pool, 10);
        assert_eq!(db.timeout, None);
    }

    #[test]
    fn test_deserialize_bool_field() {
        let mut config = Config::new();
        config.set("app.name", "MyApp").unwrap();
        config.set("app.version", "1.0.0").unwrap();
        config.set("app.debug", true).unwrap();

        let app: AppConfig = config.deserialize("app").unwrap();
        assert_eq!(app.name, "MyApp");
        assert_eq!(app.version, "1.0.0");
        assert!(app.debug);
    }

    #[test]
    fn test_deserialize_nested_struct() {
        let mut config = Config::new();
        config.set("app.server.host", "localhost").unwrap();
        config.set("app.server.port", 9090).unwrap();

        let app: NestedAppConfig = config.deserialize("app").unwrap();
        assert_eq!(app.server.host, "localhost");
        assert_eq!(app.server.port, 9090);
    }

    #[test]
    fn test_deserialize_empty_prefix() {
        let mut config = Config::new();
        config.set("host", "localhost").unwrap();
        config.set("port", 8080).unwrap();

        let server: ServerConfig = config.deserialize("").unwrap();
        assert_eq!(server.host, "localhost");
        assert_eq!(server.port, 8080);
    }

    #[test]
    fn test_deserialize_missing_required_field_returns_error() {
        let mut config = Config::new();
        config.set("server.host", "localhost").unwrap();
        // Missing "port"

        let result: Result<ServerConfig, _> = config.deserialize("server");
        assert!(result.is_err());
        match result.unwrap_err() {
            ConfigError::DeserializeError { path, .. } => {
                assert_eq!(path, "server");
            }
            e => panic!("Expected DeserializeError, got {:?}", e),
        }
    }

    #[test]
    fn test_deserialize_with_optional_null_field() {
        let mut config = Config::new();
        config.set("srv.host", "localhost").unwrap();
        // Insert null for port
        config.set_null("srv.port", DataType::Int32).unwrap();

        let result: WithOptionals = config.deserialize("srv").unwrap();
        assert_eq!(result.host, "localhost");
        assert_eq!(result.port, None);
    }

    #[test]
    fn test_deserialize_hashmap() {
        let mut config = Config::new();
        config.set("headers.authorization", "Bearer token").unwrap();
        config
            .set("headers.content-type", "application/json")
            .unwrap();

        let headers: HashMap<String, String> = config.deserialize("headers").unwrap();
        assert_eq!(
            headers.get("authorization"),
            Some(&"Bearer token".to_string())
        );
        assert_eq!(
            headers.get("content-type"),
            Some(&"application/json".to_string())
        );
    }

    #[test]
    fn test_deserialize_conflicting_dotted_key_falls_back_to_flat_key() {
        #[derive(Deserialize, Debug, PartialEq)]
        struct CtxConfig {
            a: i32,
        }

        let mut config = Config::new();
        config.set("ctx.a", 1).unwrap();
        config.set("ctx.a.b", "unexpected-but-ignored").unwrap();

        let ctx: CtxConfig = config.deserialize("ctx").unwrap();
        assert_eq!(ctx.a, 1);
    }

    #[test]
    fn test_deserialize_malformed_dotted_key_falls_back_to_flat_key() {
        let mut config = Config::new();
        config.set("bad..key", "value").unwrap();

        let root: HashMap<String, serde_json::Value> = config.deserialize("").unwrap();
        assert_eq!(root.get("bad..key"), Some(&serde_json::json!("value")));
    }

    #[test]
    fn test_deserialize_multivalue_as_array() {
        #[derive(Deserialize, Debug, PartialEq)]
        struct WithList {
            ports: Vec<i32>,
        }

        let mut config = Config::new();
        config.set("svc.ports", vec![8080, 8081, 8082]).unwrap();

        let svc: WithList = config.deserialize("svc").unwrap();
        assert_eq!(svc.ports, vec![8080, 8081, 8082]);
    }
}

// ============================================================================
// property_to_json_value coverage: various MultiValues types via deserialize
// ============================================================================

#[cfg(test)]
mod test_property_to_json_value_coverage {
    use super::*;
    use bigdecimal::BigDecimal;
    use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
    use num_bigint::BigInt;
    use std::str::FromStr;
    use std::time::Duration;
    use url::Url;

    #[derive(Deserialize, Debug, PartialEq)]
    struct AnyStruct {
        val: serde_json::Value,
    }

    fn config_with_mv(key: &str, mv: MultiValues) -> Config {
        let mut config = Config::new();
        config
            .insert_property(key, Property::with_value(key, mv))
            .unwrap();
        config
    }

    #[test]
    fn test_deserialize_bool_single() {
        let mut config = Config::new();
        config.set("x.val", true).unwrap();
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert_eq!(s.val, serde_json::Value::Bool(true));
    }

    #[test]
    fn test_deserialize_bool_multi() {
        let mut config = Config::new();
        config.set("x.val", vec![true, false]).unwrap();
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert!(s.val.is_array());
    }

    #[test]
    fn test_deserialize_int8() {
        let config = config_with_mv("x.val", MultiValues::Int8(vec![42i8]));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert_eq!(s.val, serde_json::json!(42));
    }

    #[test]
    fn test_deserialize_int16() {
        let config = config_with_mv("x.val", MultiValues::Int16(vec![1000i16]));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert_eq!(s.val, serde_json::json!(1000));
    }

    #[test]
    fn test_deserialize_int32() {
        let config = config_with_mv("x.val", MultiValues::Int32(vec![8080i32]));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert_eq!(s.val, serde_json::json!(8080));
    }

    #[test]
    fn test_deserialize_int64() {
        let config = config_with_mv("x.val", MultiValues::Int64(vec![9999i64]));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert_eq!(s.val, serde_json::json!(9999));
    }

    #[test]
    fn test_deserialize_intsize() {
        let config = config_with_mv("x.val", MultiValues::IntSize(vec![42isize]));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert_eq!(s.val, serde_json::json!(42));
    }

    #[test]
    fn test_deserialize_uint8() {
        let config = config_with_mv("x.val", MultiValues::UInt8(vec![255u8]));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert_eq!(s.val, serde_json::json!(255));
    }

    #[test]
    fn test_deserialize_uint16() {
        let config = config_with_mv("x.val", MultiValues::UInt16(vec![1000u16]));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert_eq!(s.val, serde_json::json!(1000));
    }

    #[test]
    fn test_deserialize_uint32() {
        let config = config_with_mv("x.val", MultiValues::UInt32(vec![42u32]));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert_eq!(s.val, serde_json::json!(42));
    }

    #[test]
    fn test_deserialize_uint64() {
        let config = config_with_mv("x.val", MultiValues::UInt64(vec![42u64]));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert_eq!(s.val, serde_json::json!(42));
    }

    #[test]
    fn test_deserialize_uintsize() {
        let config = config_with_mv("x.val", MultiValues::UIntSize(vec![42usize]));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert_eq!(s.val, serde_json::json!(42));
    }

    #[test]
    fn test_deserialize_float32() {
        let config = config_with_mv("x.val", MultiValues::Float32(vec![1.5f32]));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert!(s.val.is_number());
    }

    #[test]
    fn test_deserialize_float64() {
        let config = config_with_mv("x.val", MultiValues::Float64(vec![1.5f64]));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert!(s.val.is_number());
    }

    #[test]
    fn test_deserialize_float32_nan_becomes_null() {
        let config = config_with_mv("x.val", MultiValues::Float32(vec![f32::NAN]));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert!(s.val.is_null());
    }

    #[test]
    fn test_deserialize_float64_nan_becomes_null() {
        let config = config_with_mv("x.val", MultiValues::Float64(vec![f64::NAN]));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert!(s.val.is_null());
    }

    #[test]
    fn test_deserialize_duration() {
        let config = config_with_mv(
            "x.val",
            MultiValues::Duration(vec![Duration::from_millis(500)]),
        );
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert_eq!(s.val, serde_json::json!("500ms"));
    }

    #[test]
    fn test_deserialize_url() {
        let url = Url::parse("https://example.com").unwrap();
        let config = config_with_mv("x.val", MultiValues::Url(vec![url]));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert!(s.val.as_str().unwrap().contains("example.com"));
    }

    #[test]
    fn test_deserialize_string_map_single() {
        let mut map = std::collections::HashMap::new();
        map.insert("key".to_string(), "value".to_string());
        let config = config_with_mv("x.val", MultiValues::StringMap(vec![map]));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert!(s.val.is_object());
        assert_eq!(s.val["key"], serde_json::json!("value"));
    }

    #[test]
    fn test_deserialize_string_map_multi() {
        let mut map1 = std::collections::HashMap::new();
        map1.insert("k1".to_string(), "v1".to_string());
        let mut map2 = std::collections::HashMap::new();
        map2.insert("k2".to_string(), "v2".to_string());
        let config = config_with_mv("x.val", MultiValues::StringMap(vec![map1, map2]));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert!(s.val.is_array());
    }

    #[test]
    fn test_deserialize_json_single() {
        let json_val = serde_json::json!({"nested": true});
        let config = config_with_mv("x.val", MultiValues::Json(vec![json_val.clone()]));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert_eq!(s.val, json_val);
    }

    #[test]
    fn test_deserialize_json_multi() {
        let j1 = serde_json::json!(1);
        let j2 = serde_json::json!(2);
        let config = config_with_mv("x.val", MultiValues::Json(vec![j1, j2]));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert!(s.val.is_array());
    }

    #[test]
    fn test_deserialize_char() {
        let config = config_with_mv("x.val", MultiValues::Char(vec!['A']));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert_eq!(s.val, serde_json::json!("A"));
    }

    #[test]
    fn test_deserialize_big_integer() {
        let big = BigInt::from(12345678901234567i64);
        let config = config_with_mv("x.val", MultiValues::BigInteger(vec![big]));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert!(s.val.is_string());
    }

    #[test]
    fn test_deserialize_big_decimal() {
        let dec = BigDecimal::from_str("3.14159265358979").unwrap();
        let config = config_with_mv("x.val", MultiValues::BigDecimal(vec![dec]));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert!(s.val.is_string());
    }

    #[test]
    fn test_deserialize_datetime() {
        let dt = NaiveDateTime::parse_from_str("2026-04-09 12:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let config = config_with_mv("x.val", MultiValues::DateTime(vec![dt]));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert!(s.val.is_string());
    }

    #[test]
    fn test_deserialize_date() {
        let d = NaiveDate::from_ymd_opt(2026, 4, 9).unwrap();
        let config = config_with_mv("x.val", MultiValues::Date(vec![d]));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert!(s.val.is_string());
    }

    #[test]
    fn test_deserialize_time() {
        let t = NaiveTime::from_hms_opt(12, 0, 0).unwrap();
        let config = config_with_mv("x.val", MultiValues::Time(vec![t]));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert!(s.val.is_string());
    }

    #[test]
    fn test_deserialize_instant() {
        let instant: DateTime<Utc> = DateTime::parse_from_rfc3339("2026-04-09T12:00:00Z")
            .unwrap()
            .into();
        let config = config_with_mv("x.val", MultiValues::Instant(vec![instant]));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert!(s.val.is_string());
    }

    #[test]
    fn test_deserialize_int128() {
        let config = config_with_mv("x.val", MultiValues::Int128(vec![42i128]));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert!(s.val.is_string());
    }

    #[test]
    fn test_deserialize_uint128() {
        let config = config_with_mv("x.val", MultiValues::UInt128(vec![42u128]));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert!(s.val.is_string());
    }

    #[test]
    fn test_deserialize_empty_multivalue_is_null() {
        let config = config_with_mv("x.val", MultiValues::Empty(DataType::String));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert!(s.val.is_null());
    }

    #[test]
    fn test_deserialize_multi_int32_array() {
        let config = config_with_mv("x.val", MultiValues::Int32(vec![1, 2, 3]));
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert!(s.val.is_array());
        assert_eq!(s.val.as_array().unwrap().len(), 3);
    }

    #[test]
    fn test_deserialize_multi_string_array() {
        let config = config_with_mv(
            "x.val",
            MultiValues::String(vec!["a".to_string(), "b".to_string()]),
        );
        let s: AnyStruct = config.deserialize("x").unwrap();
        assert!(s.val.is_array());
    }
}
