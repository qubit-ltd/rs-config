/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! # Property Unit Tests
//!
//! Tests all public methods of the Property struct, including methods delegated to MultiValues.
//!

use bigdecimal::BigDecimal;
use chrono::{
    DateTime,
    NaiveDate,
    NaiveTime,
};
use num_bigint::BigInt;
use qubit_config::Property;
use qubit_datatype::DataType;
use qubit_value::MultiValues;
use std::str::FromStr;

// ============================================================================
// Property Basic Method Tests
// ============================================================================

#[test]
fn test_property_new() {
    let prop = Property::new("test.property");
    assert_eq!(prop.name(), "test.property");
    assert!(prop.is_empty());
    assert_eq!(prop.count(), 0);
    assert_eq!(prop.data_type(), DataType::Int32);
    assert!(prop.description().is_none());
    assert!(!prop.is_final());
}

#[test]
fn test_property_with_value() {
    let value = MultiValues::String(vec!["hello".to_string(), "world".to_string()]);
    let prop = Property::with_value("test.string", value);

    assert_eq!(prop.name(), "test.string");
    assert_eq!(prop.count(), 2);
    assert_eq!(prop.data_type(), DataType::String);
    assert!(!prop.is_empty());
}

#[test]
fn test_property_name() {
    let prop = Property::new("my.property.name");
    assert_eq!(prop.name(), "my.property.name");
}

#[test]
fn test_property_value() {
    let mut prop = Property::new("test");
    let value = MultiValues::Int32(vec![42, 43]);
    prop.set_value(value.clone());

    assert_eq!(prop.value(), &value);
}

#[test]
fn test_property_value_mut() {
    let mut prop = Property::new("test");
    let value = MultiValues::Int32(vec![42]);
    prop.set_value(value);

    let value_mut = prop.value_mut();
    value_mut.add_int32(43).unwrap();

    assert_eq!(prop.count(), 2);
}

#[test]
fn test_property_set_value() {
    let mut prop = Property::new("test");
    let value1 = MultiValues::Int32(vec![1, 2]);
    let value2 = MultiValues::String(vec!["hello".to_string()]);

    prop.set_value(value1.clone());
    assert_eq!(prop.data_type(), DataType::Int32);
    assert_eq!(prop.count(), 2);

    prop.set_value(value2.clone());
    assert_eq!(prop.data_type(), DataType::String);
    assert_eq!(prop.count(), 1);
}

#[test]
fn test_property_description() {
    let mut prop = Property::new("test");

    // Initial state
    assert!(prop.description().is_none());

    // Set description
    prop.set_description(Some("Test property".to_string()));
    assert_eq!(prop.description(), Some("Test property"));

    // Clear description
    prop.set_description(None);
    assert!(prop.description().is_none());
}

#[test]
fn test_property_is_final() {
    let mut prop = Property::new("test");

    // Initial state
    assert!(!prop.is_final());

    // Set as final
    prop.set_final(true);
    assert!(prop.is_final());

    // Unset final
    prop.set_final(false);
    assert!(!prop.is_final());
}

#[test]
fn test_property_data_type() {
    let mut prop = Property::new("test");

    // Default type
    assert_eq!(prop.data_type(), DataType::Int32);

    // Set different types
    prop.set_value(MultiValues::String(vec!["test".to_string()]));
    assert_eq!(prop.data_type(), DataType::String);

    prop.set_value(MultiValues::Bool(vec![true, false]));
    assert_eq!(prop.data_type(), DataType::Bool);
}

#[test]
fn test_property_count() {
    let mut prop = Property::new("test");

    // Empty value
    assert_eq!(prop.count(), 0);

    // Single value
    prop.set_value(MultiValues::Int32(vec![42]));
    assert_eq!(prop.count(), 1);

    // Multiple values
    prop.set_value(MultiValues::Int32(vec![1, 2, 3, 4, 5]));
    assert_eq!(prop.count(), 5);
}

#[test]
fn test_property_is_empty() {
    let mut prop = Property::new("test");

    // Empty
    assert!(prop.is_empty());

    // Has value
    prop.set_value(MultiValues::Int32(vec![42]));
    assert!(!prop.is_empty());

    // After clearing
    prop.clear();
    assert!(prop.is_empty());
}

#[test]
fn test_property_clear() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Int32(vec![1, 2, 3]));

    assert_eq!(prop.count(), 3);
    prop.clear();
    assert_eq!(prop.count(), 0);
    assert_eq!(prop.data_type(), DataType::Int32); // Type remains unchanged
}

// ============================================================================
// Deref Delegation Method Tests - Bool Type
// ============================================================================

#[test]
fn test_property_bool_get() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Bool(vec![true, false, true]));

    let values = prop.get_bools().unwrap();
    assert_eq!(values, &[true, false, true]);
}

#[test]
fn test_property_bool_get_first() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Bool(vec![false, true]));

    let first = prop.get_first_bool().unwrap();
    assert!(!first);
}

#[test]
fn test_property_bool_add() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Bool(vec![true]));

    prop.add_bool(false).unwrap();
    prop.add_bool(true).unwrap();

    let values = prop.get_bools().unwrap();
    assert_eq!(values, &[true, false, true]);
}

#[test]
fn test_property_bool_set() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Bool(vec![true, false]));

    prop.set_bool(false).unwrap();
    let values = prop.get_bools().unwrap();
    assert_eq!(values, &[false]);
}

// ============================================================================
// Deref Delegation Method Tests - Char Type
// ============================================================================

#[test]
fn test_property_char_get() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Char(vec!['a', 'b', 'c']));

    let values = prop.get_chars().unwrap();
    assert_eq!(values, &['a', 'b', 'c']);
}

#[test]
fn test_property_char_get_first() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Char(vec!['x', 'y']));

    let first = prop.get_first_char().unwrap();
    assert_eq!(first, 'x');
}

#[test]
fn test_property_char_add() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Char(vec!['a']));

    prop.add_char('b').unwrap();
    prop.add_char('c').unwrap();

    let values = prop.get_chars().unwrap();
    assert_eq!(values, &['a', 'b', 'c']);
}

#[test]
fn test_property_char_set() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Char(vec!['a', 'b']));

    prop.set_char('z').unwrap();
    let values = prop.get_chars().unwrap();
    assert_eq!(values, &['z']);
}

// ============================================================================
// Deref Delegation Method Tests - Int8 Type
// ============================================================================

#[test]
fn test_property_int8_get() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Int8(vec![1, 2, 3]));

    let values = prop.get_int8s().unwrap();
    assert_eq!(values, &[1, 2, 3]);
}

#[test]
fn test_property_int8_get_first() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Int8(vec![42, 43]));

    let first = prop.get_first_int8().unwrap();
    assert_eq!(first, 42);
}

#[test]
fn test_property_int8_add() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Int8(vec![1]));

    prop.add_int8(2).unwrap();
    prop.add_int8(3).unwrap();

    let values = prop.get_int8s().unwrap();
    assert_eq!(values, &[1, 2, 3]);
}

#[test]
fn test_property_int8_set() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Int8(vec![1, 2]));

    prop.set_int8(99).unwrap();
    let values = prop.get_int8s().unwrap();
    assert_eq!(values, &[99]);
}

// ============================================================================
// Deref Delegation Method Tests - Int16 Type
// ============================================================================

#[test]
fn test_property_int16_get() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Int16(vec![1000, 2000, 3000]));

    let values = prop.get_int16s().unwrap();
    assert_eq!(values, &[1000, 2000, 3000]);
}

#[test]
fn test_property_int16_get_first() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Int16(vec![1234, 5678]));

    let first = prop.get_first_int16().unwrap();
    assert_eq!(first, 1234);
}

#[test]
fn test_property_int16_add() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Int16(vec![1000]));

    prop.add_int16(2000).unwrap();
    prop.add_int16(3000).unwrap();

    let values = prop.get_int16s().unwrap();
    assert_eq!(values, &[1000, 2000, 3000]);
}

#[test]
fn test_property_int16_set() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Int16(vec![1000, 2000]));

    prop.set_int16(9999).unwrap();
    let values = prop.get_int16s().unwrap();
    assert_eq!(values, &[9999]);
}

// ============================================================================
// Deref Delegation Method Tests - Int32 Type
// ============================================================================

#[test]
fn test_property_int32_get() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Int32(vec![100000, 200000, 300000]));

    let values = prop.get_int32s().unwrap();
    assert_eq!(values, &[100000, 200000, 300000]);
}

#[test]
fn test_property_int32_get_first() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Int32(vec![123456, 789012]));

    let first = prop.get_first_int32().unwrap();
    assert_eq!(first, 123456);
}

#[test]
fn test_property_int32_add() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Int32(vec![100000]));

    prop.add_int32(200000).unwrap();
    prop.add_int32(300000).unwrap();

    let values = prop.get_int32s().unwrap();
    assert_eq!(values, &[100000, 200000, 300000]);
}

#[test]
fn test_property_int32_set() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Int32(vec![100000, 200000]));

    prop.set_int32(999999).unwrap();
    let values = prop.get_int32s().unwrap();
    assert_eq!(values, &[999999]);
}

// ============================================================================
// Deref Delegation Method Tests - Int64 Type
// ============================================================================

#[test]
fn test_property_int64_get() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Int64(vec![1000000000, 2000000000, 3000000000]));

    let values = prop.get_int64s().unwrap();
    assert_eq!(values, &[1000000000, 2000000000, 3000000000]);
}

#[test]
fn test_property_int64_get_first() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Int64(vec![1234567890, 9876543210]));

    let first = prop.get_first_int64().unwrap();
    assert_eq!(first, 1234567890);
}

#[test]
fn test_property_int64_add() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Int64(vec![1000000000]));

    prop.add_int64(2000000000).unwrap();
    prop.add_int64(3000000000).unwrap();

    let values = prop.get_int64s().unwrap();
    assert_eq!(values, &[1000000000, 2000000000, 3000000000]);
}

#[test]
fn test_property_int64_set() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Int64(vec![1000000000, 2000000000]));

    prop.set_int64(9999999999).unwrap();
    let values = prop.get_int64s().unwrap();
    assert_eq!(values, &[9999999999]);
}

// ============================================================================
// Deref Delegation Method Tests - Int128 Type
// ============================================================================

#[test]
fn test_property_int128_get() {
    let mut prop = Property::new("test");
    let values = vec![
        1000000000000000000000000000000_i128,
        2000000000000000000000000000000_i128,
        3000000000000000000000000000000_i128,
    ];
    prop.set_value(MultiValues::Int128(values.clone()));

    let result = prop.get_int128s().unwrap();
    assert_eq!(result, &values);
}

#[test]
fn test_property_int128_get_first() {
    let mut prop = Property::new("test");
    let values = vec![
        123456789012345678901234567890_i128,
        987654321098765432109876543210_i128,
    ];
    prop.set_value(MultiValues::Int128(values));

    let first = prop.get_first_int128().unwrap();
    assert_eq!(first, 123456789012345678901234567890_i128);
}

#[test]
fn test_property_int128_add() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Int128(vec![
        1000000000000000000000000000000_i128,
    ]));

    prop.add_int128(2000000000000000000000000000000_i128)
        .unwrap();
    prop.add_int128(3000000000000000000000000000000_i128)
        .unwrap();

    let values = prop.get_int128s().unwrap();
    assert_eq!(values.len(), 3);
}

#[test]
fn test_property_int128_set() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Int128(vec![
        1000000000000000000000000000000_i128,
        2000000000000000000000000000000_i128,
    ]));

    prop.set_int128(9999999999999999999999999999999_i128)
        .unwrap();
    let values = prop.get_int128s().unwrap();
    assert_eq!(values.len(), 1);
}

// ============================================================================
// Deref Delegation Method Tests - UInt8 Type
// ============================================================================

#[test]
fn test_property_uint8_get() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::UInt8(vec![1, 2, 3]));

    let values = prop.get_uint8s().unwrap();
    assert_eq!(values, &[1, 2, 3]);
}

#[test]
fn test_property_uint8_get_first() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::UInt8(vec![42, 43]));

    let first = prop.get_first_uint8().unwrap();
    assert_eq!(first, 42);
}

#[test]
fn test_property_uint8_add() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::UInt8(vec![1]));

    prop.add_uint8(2).unwrap();
    prop.add_uint8(3).unwrap();

    let values = prop.get_uint8s().unwrap();
    assert_eq!(values, &[1, 2, 3]);
}

#[test]
fn test_property_uint8_set() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::UInt8(vec![1, 2]));

    prop.set_uint8(255).unwrap();
    let values = prop.get_uint8s().unwrap();
    assert_eq!(values, &[255]);
}

// ============================================================================
// Deref Delegation Method Tests - UInt16 Type
// ============================================================================

#[test]
fn test_property_uint16_get() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::UInt16(vec![1000, 2000, 3000]));

    let values = prop.get_uint16s().unwrap();
    assert_eq!(values, &[1000, 2000, 3000]);
}

#[test]
fn test_property_uint16_get_first() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::UInt16(vec![1234, 5678]));

    let first = prop.get_first_uint16().unwrap();
    assert_eq!(first, 1234);
}

#[test]
fn test_property_uint16_add() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::UInt16(vec![1000]));

    prop.add_uint16(2000).unwrap();
    prop.add_uint16(3000).unwrap();

    let values = prop.get_uint16s().unwrap();
    assert_eq!(values, &[1000, 2000, 3000]);
}

#[test]
fn test_property_uint16_set() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::UInt16(vec![1000, 2000]));

    prop.set_uint16(65535).unwrap();
    let values = prop.get_uint16s().unwrap();
    assert_eq!(values, &[65535]);
}

// ============================================================================
// Deref Delegation Method Tests - UInt32 Type
// ============================================================================

#[test]
fn test_property_uint32_get() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::UInt32(vec![100000, 200000, 300000]));

    let values = prop.get_uint32s().unwrap();
    assert_eq!(values, &[100000, 200000, 300000]);
}

#[test]
fn test_property_uint32_get_first() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::UInt32(vec![123456, 789012]));

    let first = prop.get_first_uint32().unwrap();
    assert_eq!(first, 123456);
}

#[test]
fn test_property_uint32_add() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::UInt32(vec![100000]));

    prop.add_uint32(200000).unwrap();
    prop.add_uint32(300000).unwrap();

    let values = prop.get_uint32s().unwrap();
    assert_eq!(values, &[100000, 200000, 300000]);
}

#[test]
fn test_property_uint32_set() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::UInt32(vec![100000, 200000]));

    prop.set_uint32(4294967295).unwrap();
    let values = prop.get_uint32s().unwrap();
    assert_eq!(values, &[4294967295]);
}

// ============================================================================
// Deref Delegation Method Tests - UInt64 Type
// ============================================================================

#[test]
fn test_property_uint64_get() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::UInt64(vec![
        1000000000, 2000000000, 3000000000,
    ]));

    let values = prop.get_uint64s().unwrap();
    assert_eq!(values, &[1000000000, 2000000000, 3000000000]);
}

#[test]
fn test_property_uint64_get_first() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::UInt64(vec![1234567890, 9876543210]));

    let first = prop.get_first_uint64().unwrap();
    assert_eq!(first, 1234567890);
}

#[test]
fn test_property_uint64_add() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::UInt64(vec![1000000000]));

    prop.add_uint64(2000000000).unwrap();
    prop.add_uint64(3000000000).unwrap();

    let values = prop.get_uint64s().unwrap();
    assert_eq!(values, &[1000000000, 2000000000, 3000000000]);
}

#[test]
fn test_property_uint64_set() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::UInt64(vec![1000000000, 2000000000]));

    prop.set_uint64(18446744073709551615).unwrap();
    let values = prop.get_uint64s().unwrap();
    assert_eq!(values, &[18446744073709551615]);
}

// ============================================================================
// Deref Delegation Method Tests - UInt128 Type
// ============================================================================

#[test]
fn test_property_uint128_get() {
    let mut prop = Property::new("test");
    let values = vec![
        1000000000000000000000000000000_u128,
        2000000000000000000000000000000_u128,
        3000000000000000000000000000000_u128,
    ];
    prop.set_value(MultiValues::UInt128(values.clone()));

    let result = prop.get_uint128s().unwrap();
    assert_eq!(result, &values);
}

#[test]
fn test_property_uint128_get_first() {
    let mut prop = Property::new("test");
    let values = vec![
        123456789012345678901234567890_u128,
        987654321098765432109876543210_u128,
    ];
    prop.set_value(MultiValues::UInt128(values));

    let first = prop.get_first_uint128().unwrap();
    assert_eq!(first, 123456789012345678901234567890_u128);
}

#[test]
fn test_property_uint128_add() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::UInt128(vec![
        1000000000000000000000000000000_u128,
    ]));

    prop.add_uint128(2000000000000000000000000000000_u128)
        .unwrap();
    prop.add_uint128(3000000000000000000000000000000_u128)
        .unwrap();

    let values = prop.get_uint128s().unwrap();
    assert_eq!(values.len(), 3);
}

#[test]
fn test_property_uint128_set() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::UInt128(vec![
        1000000000000000000000000000000_u128,
        2000000000000000000000000000000_u128,
    ]));

    prop.set_uint128(340282366920938463463374607431768211455_u128)
        .unwrap();
    let values = prop.get_uint128s().unwrap();
    assert_eq!(values.len(), 1);
}

// ============================================================================
// Deref Delegation Method Tests - Float32 Type
// ============================================================================

#[test]
fn test_property_float32_get() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Float32(vec![1.1, 2.2, 3.3]));

    let values = prop.get_float32s().unwrap();
    assert_eq!(values, &[1.1, 2.2, 3.3]);
}

#[test]
fn test_property_float32_get_first() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Float32(vec![3.5, 2.72]));

    let first = prop.get_first_float32().unwrap();
    assert_eq!(first, 3.5);
}

#[test]
fn test_property_float32_add() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Float32(vec![1.1]));

    prop.add_float32(2.2).unwrap();
    prop.add_float32(3.3).unwrap();

    let values = prop.get_float32s().unwrap();
    assert_eq!(values, &[1.1, 2.2, 3.3]);
}

#[test]
fn test_property_float32_set() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Float32(vec![1.1, 2.2]));

    prop.set_float32(99.99).unwrap();
    let values = prop.get_float32s().unwrap();
    assert_eq!(values, &[99.99]);
}

// ============================================================================
// Deref Delegation Method Tests - Float64 Type
// ============================================================================

#[test]
fn test_property_float64_get() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Float64(vec![1.111111, 2.222222, 3.333333]));

    let values = prop.get_float64s().unwrap();
    assert_eq!(values, &[1.111111, 2.222222, 3.333333]);
}

#[test]
fn test_property_float64_get_first() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Float64(vec![3.5, 2.8]));

    let first = prop.get_first_float64().unwrap();
    assert_eq!(first, 3.5);
}

#[test]
fn test_property_float64_add() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Float64(vec![1.111111]));

    prop.add_float64(2.222222).unwrap();
    prop.add_float64(3.333333).unwrap();

    let values = prop.get_float64s().unwrap();
    assert_eq!(values, &[1.111111, 2.222222, 3.333333]);
}

#[test]
fn test_property_float64_set() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Float64(vec![1.111111, 2.222222]));

    prop.set_float64(99.999999).unwrap();
    let values = prop.get_float64s().unwrap();
    assert_eq!(values, &[99.999999]);
}

// ============================================================================
// Deref Delegation Method Tests - String Type
// ============================================================================

#[test]
fn test_property_string_get() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::String(vec![
        "hello".to_string(),
        "world".to_string(),
        "rust".to_string(),
    ]));

    let values = prop.get_strings().unwrap();
    assert_eq!(values, &["hello", "world", "rust"]);
}

#[test]
fn test_property_string_get_first() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::String(vec![
        "first".to_string(),
        "second".to_string(),
    ]));

    let first = prop.get_first_string().unwrap();
    assert_eq!(first, "first");
}

#[test]
fn test_property_string_add() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::String(vec!["hello".to_string()]));

    prop.add_string("world".to_string()).unwrap();
    prop.add_string("rust".to_string()).unwrap();

    let values = prop.get_strings().unwrap();
    assert_eq!(values, &["hello", "world", "rust"]);
}

#[test]
fn test_property_string_set() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::String(vec![
        "hello".to_string(),
        "world".to_string(),
    ]));

    prop.set_string("rust".to_string()).unwrap();
    let values = prop.get_strings().unwrap();
    assert_eq!(values, &["rust"]);
}

// ============================================================================
// Deref Delegation Method Tests - ByteArray Type
// ============================================================================

// ByteArray related tests have been removed

// ============================================================================
// Deref Delegation Method Tests - Date Type
// ============================================================================

#[test]
fn test_property_date_get() {
    let mut prop = Property::new("test");
    let date1 = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
    let date2 = NaiveDate::from_ymd_opt(2023, 2, 1).unwrap();
    let date3 = NaiveDate::from_ymd_opt(2023, 3, 1).unwrap();
    prop.set_value(MultiValues::Date(vec![date1, date2, date3]));

    let values = prop.get_dates().unwrap();
    assert_eq!(values.len(), 3);
}

#[test]
fn test_property_date_get_first() {
    let mut prop = Property::new("test");
    let date1 = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
    let date2 = NaiveDate::from_ymd_opt(2023, 2, 1).unwrap();
    prop.set_value(MultiValues::Date(vec![date1, date2]));

    let first = prop.get_first_date().unwrap();
    assert_eq!(first, NaiveDate::from_ymd_opt(2023, 1, 1).unwrap());
}

#[test]
fn test_property_date_add() {
    let mut prop = Property::new("test");
    let date1 = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
    prop.set_value(MultiValues::Date(vec![date1]));

    let date2 = NaiveDate::from_ymd_opt(2023, 2, 1).unwrap();
    let date3 = NaiveDate::from_ymd_opt(2023, 3, 1).unwrap();
    prop.add_date(date2).unwrap();
    prop.add_date(date3).unwrap();

    let values = prop.get_dates().unwrap();
    assert_eq!(values.len(), 3);
}

#[test]
fn test_property_date_set() {
    let mut prop = Property::new("test");
    let date1 = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
    let date2 = NaiveDate::from_ymd_opt(2023, 2, 1).unwrap();
    prop.set_value(MultiValues::Date(vec![date1, date2]));

    let new_date = NaiveDate::from_ymd_opt(2023, 12, 31).unwrap();
    prop.set_date(new_date).unwrap();
    let values = prop.get_dates().unwrap();
    assert_eq!(values.len(), 1);
}

// ============================================================================
// Deref Delegation Method Tests - Time Type
// ============================================================================

#[test]
fn test_property_time_get() {
    let mut prop = Property::new("test");
    let time1 = NaiveTime::from_hms_opt(10, 30, 0).unwrap();
    let time2 = NaiveTime::from_hms_opt(14, 45, 30).unwrap();
    let time3 = NaiveTime::from_hms_opt(18, 0, 0).unwrap();
    prop.set_value(MultiValues::Time(vec![time1, time2, time3]));

    let values = prop.get_times().unwrap();
    assert_eq!(values.len(), 3);
}

#[test]
fn test_property_time_get_first() {
    let mut prop = Property::new("test");
    let time1 = NaiveTime::from_hms_opt(10, 30, 0).unwrap();
    let time2 = NaiveTime::from_hms_opt(14, 45, 30).unwrap();
    prop.set_value(MultiValues::Time(vec![time1, time2]));

    let first = prop.get_first_time().unwrap();
    assert_eq!(first, NaiveTime::from_hms_opt(10, 30, 0).unwrap());
}

#[test]
fn test_property_time_add() {
    let mut prop = Property::new("test");
    let time1 = NaiveTime::from_hms_opt(10, 30, 0).unwrap();
    prop.set_value(MultiValues::Time(vec![time1]));

    let time2 = NaiveTime::from_hms_opt(14, 45, 30).unwrap();
    let time3 = NaiveTime::from_hms_opt(18, 0, 0).unwrap();
    prop.add_time(time2).unwrap();
    prop.add_time(time3).unwrap();

    let values = prop.get_times().unwrap();
    assert_eq!(values.len(), 3);
}

#[test]
fn test_property_time_set() {
    let mut prop = Property::new("test");
    let time1 = NaiveTime::from_hms_opt(10, 30, 0).unwrap();
    let time2 = NaiveTime::from_hms_opt(14, 45, 30).unwrap();
    prop.set_value(MultiValues::Time(vec![time1, time2]));

    let new_time = NaiveTime::from_hms_opt(23, 59, 59).unwrap();
    prop.set_time(new_time).unwrap();
    let values = prop.get_times().unwrap();
    assert_eq!(values.len(), 1);
}

// ============================================================================
// Deref Delegation Method Tests - Instant Type
// ============================================================================

#[test]
fn test_property_instant_get() {
    let mut prop = Property::new("test");
    let instant1 = DateTime::from_timestamp(1672531200, 0).unwrap();
    let instant2 = DateTime::from_timestamp(1672617600, 0).unwrap();
    let instant3 = DateTime::from_timestamp(1672704000, 0).unwrap();
    prop.set_value(MultiValues::Instant(vec![instant1, instant2, instant3]));

    let values = prop.get_instants().unwrap();
    assert_eq!(values.len(), 3);
}

#[test]
fn test_property_instant_get_first() {
    let mut prop = Property::new("test");
    let instant1 = DateTime::from_timestamp(1672531200, 0).unwrap();
    let instant2 = DateTime::from_timestamp(1672617600, 0).unwrap();
    prop.set_value(MultiValues::Instant(vec![instant1, instant2]));

    let first = prop.get_first_instant().unwrap();
    assert_eq!(first, DateTime::from_timestamp(1672531200, 0).unwrap());
}

#[test]
fn test_property_instant_add() {
    let mut prop = Property::new("test");
    let instant1 = DateTime::from_timestamp(1672531200, 0).unwrap();
    prop.set_value(MultiValues::Instant(vec![instant1]));

    let instant2 = DateTime::from_timestamp(1672617600, 0).unwrap();
    let instant3 = DateTime::from_timestamp(1672704000, 0).unwrap();
    prop.add_instant(instant2).unwrap();
    prop.add_instant(instant3).unwrap();

    let values = prop.get_instants().unwrap();
    assert_eq!(values.len(), 3);
}

#[test]
fn test_property_instant_set() {
    let mut prop = Property::new("test");
    let instant1 = DateTime::from_timestamp(1672531200, 0).unwrap();
    let instant2 = DateTime::from_timestamp(1672617600, 0).unwrap();
    prop.set_value(MultiValues::Instant(vec![instant1, instant2]));

    let new_instant = DateTime::from_timestamp(1672790400, 0).unwrap();
    prop.set_instant(new_instant).unwrap();
    let values = prop.get_instants().unwrap();
    assert_eq!(values.len(), 1);
}

// ============================================================================
// Deref Delegation Method Tests - BigInteger Type
// ============================================================================

#[test]
fn test_property_bigint_get() {
    let mut prop = Property::new("test");
    let bigint1 = BigInt::from(12345678901234567890_i128);
    let bigint2 = BigInt::from(98765432109876543210_i128);
    let bigint3 = BigInt::from(11111111111111111111_i128);
    prop.set_value(MultiValues::BigInteger(vec![
        bigint1.clone(),
        bigint2.clone(),
        bigint3.clone(),
    ]));

    let values = prop.get_bigintegers().unwrap();
    assert_eq!(values, &[bigint1, bigint2, bigint3]);
}

#[test]
fn test_property_bigint_get_first() {
    let mut prop = Property::new("test");
    let bigint1 = BigInt::from(12345678901234567890_i128);
    let bigint2 = BigInt::from(98765432109876543210_i128);
    prop.set_value(MultiValues::BigInteger(vec![bigint1.clone(), bigint2]));

    let first = prop.get_first_biginteger().unwrap();
    assert_eq!(first, bigint1);
}

#[test]
fn test_property_bigint_add() {
    let mut prop = Property::new("test");
    let bigint1 = BigInt::from(12345678901234567890_i128);
    prop.set_value(MultiValues::BigInteger(vec![bigint1.clone()]));

    let bigint2 = BigInt::from(98765432109876543210_i128);
    let bigint3 = BigInt::from(11111111111111111111_i128);
    prop.add_biginteger(bigint2.clone()).unwrap();
    prop.add_biginteger(bigint3.clone()).unwrap();

    let values = prop.get_bigintegers().unwrap();
    assert_eq!(values, &[bigint1, bigint2, bigint3]);
}

#[test]
fn test_property_bigint_set() {
    let mut prop = Property::new("test");
    let bigint1 = BigInt::from(12345678901234567890_i128);
    let bigint2 = BigInt::from(98765432109876543210_i128);
    prop.set_value(MultiValues::BigInteger(vec![bigint1, bigint2]));

    let new_bigint = BigInt::from(99999999999999999999_i128);
    prop.set_biginteger(new_bigint.clone()).unwrap();
    let values = prop.get_bigintegers().unwrap();
    assert_eq!(values, &[new_bigint]);
}

// ============================================================================
// Deref Delegation Method Tests - BigDecimal Type
// ============================================================================

#[test]
fn test_property_bigdecimal_get() {
    let mut prop = Property::new("test");
    let bd1 = BigDecimal::from_str("123.456789").unwrap();
    let bd2 = BigDecimal::from_str("987.654321").unwrap();
    let bd3 = BigDecimal::from_str("111.222333").unwrap();
    prop.set_value(MultiValues::BigDecimal(vec![
        bd1.clone(),
        bd2.clone(),
        bd3.clone(),
    ]));

    let values = prop.get_bigdecimals().unwrap();
    assert_eq!(values, &[bd1, bd2, bd3]);
}

#[test]
fn test_property_bigdecimal_get_first() {
    let mut prop = Property::new("test");
    let bd1 = BigDecimal::from_str("123.456789").unwrap();
    let bd2 = BigDecimal::from_str("987.654321").unwrap();
    prop.set_value(MultiValues::BigDecimal(vec![bd1.clone(), bd2]));

    let first = prop.get_first_bigdecimal().unwrap();
    assert_eq!(first, bd1);
}

#[test]
fn test_property_bigdecimal_add() {
    let mut prop = Property::new("test");
    let bd1 = BigDecimal::from_str("123.456789").unwrap();
    prop.set_value(MultiValues::BigDecimal(vec![bd1.clone()]));

    let bd2 = BigDecimal::from_str("987.654321").unwrap();
    let bd3 = BigDecimal::from_str("111.222333").unwrap();
    prop.add_bigdecimal(bd2.clone()).unwrap();
    prop.add_bigdecimal(bd3.clone()).unwrap();

    let values = prop.get_bigdecimals().unwrap();
    assert_eq!(values, &[bd1, bd2, bd3]);
}

#[test]
fn test_property_bigdecimal_set() {
    let mut prop = Property::new("test");
    let bd1 = BigDecimal::from_str("123.456789").unwrap();
    let bd2 = BigDecimal::from_str("987.654321").unwrap();
    prop.set_value(MultiValues::BigDecimal(vec![bd1, bd2]));

    let new_bd = BigDecimal::from_str("999.999999").unwrap();
    prop.set_bigdecimal(new_bd.clone()).unwrap();
    let values = prop.get_bigdecimals().unwrap();
    assert_eq!(values, &[new_bd]);
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[test]
fn test_property_type_mismatch_error() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Int32(vec![42]));

    // Try to get wrong type
    let result = prop.get_strings();
    assert!(result.is_err());

    // Try to add wrong type
    let result = prop.add_string("hello".to_string());
    assert!(result.is_err());
}

#[test]
fn test_property_empty_value_error() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Empty(DataType::Int32));

    // Try to get first value
    let result = prop.get_first_int32();
    assert!(result.is_err());
}

#[test]
fn test_property_empty_get_returns_empty_slice() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Empty(DataType::Int32));

    // Getting all values should return empty slice
    let values = prop.get_int32s().unwrap();
    assert!(values.is_empty());
}

// ============================================================================
// Generic Method Tests - get<T>()
// ============================================================================

#[test]
fn test_property_generic_get_bool() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Bool(vec![true, false, true]));

    let values: Vec<bool> = prop.get().unwrap();
    assert_eq!(values, vec![true, false, true]);
}

#[test]
fn test_property_generic_get_char() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Char(vec!['a', 'b', 'c']));

    let values: Vec<char> = prop.get().unwrap();
    assert_eq!(values, vec!['a', 'b', 'c']);
}

#[test]
fn test_property_generic_get_int32() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Int32(vec![1, 2, 3]));

    let values: Vec<i32> = prop.get().unwrap();
    assert_eq!(values, vec![1, 2, 3]);
}

#[test]
fn test_property_generic_get_int64() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Int64(vec![1000000000, 2000000000]));

    let values: Vec<i64> = prop.get().unwrap();
    assert_eq!(values, vec![1000000000, 2000000000]);
}

#[test]
fn test_property_generic_get_uint32() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::UInt32(vec![100000, 200000]));

    let values: Vec<u32> = prop.get().unwrap();
    assert_eq!(values, vec![100000, 200000]);
}

#[test]
fn test_property_generic_get_float64() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Float64(vec![1.1, 2.2, 3.3]));

    let values: Vec<f64> = prop.get().unwrap();
    assert_eq!(values, vec![1.1, 2.2, 3.3]);
}

#[test]
fn test_property_generic_get_string() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::String(vec![
        "hello".to_string(),
        "world".to_string(),
    ]));

    let values: Vec<String> = prop.get().unwrap();
    assert_eq!(values, vec!["hello", "world"]);
}

// ByteArray related tests have been removed

#[test]
fn test_property_generic_get_bigint() {
    let mut prop = Property::new("test");
    let bigint1 = BigInt::from(12345678901234567890_i128);
    let bigint2 = BigInt::from(98765432109876543210_i128);
    prop.set_value(MultiValues::BigInteger(vec![
        bigint1.clone(),
        bigint2.clone(),
    ]));

    let values: Vec<BigInt> = prop.get().unwrap();
    assert_eq!(values, vec![bigint1, bigint2]);
}

#[test]
fn test_property_generic_get_bigdecimal() {
    let mut prop = Property::new("test");
    let bd1 = BigDecimal::from_str("123.456789").unwrap();
    let bd2 = BigDecimal::from_str("987.654321").unwrap();
    prop.set_value(MultiValues::BigDecimal(vec![bd1.clone(), bd2.clone()]));

    let values: Vec<BigDecimal> = prop.get().unwrap();
    assert_eq!(values, vec![bd1, bd2]);
}

// ============================================================================
// Generic Method Tests - get_first<T>()
// ============================================================================

#[test]
fn test_property_generic_get_first_bool() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Bool(vec![false, true]));

    let first: bool = prop.get_first().unwrap();
    assert!(!first);
}

#[test]
fn test_property_generic_get_first_char() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Char(vec!['x', 'y', 'z']));

    let first: char = prop.get_first().unwrap();
    assert_eq!(first, 'x');
}

#[test]
fn test_property_generic_get_first_int32() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Int32(vec![42, 43, 44]));

    let first: i32 = prop.get_first().unwrap();
    assert_eq!(first, 42);
}

#[test]
fn test_property_generic_get_first_string() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::String(vec![
        "first".to_string(),
        "second".to_string(),
    ]));

    let first: String = prop.get_first().unwrap();
    assert_eq!(first, "first");
}

#[test]
fn test_property_generic_get_first_float64() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Float64(vec![3.5, 2.72]));

    let first: f64 = prop.get_first().unwrap();
    assert_eq!(first, 3.5);
}

// ============================================================================
// Generic Method Tests - set<T>() - Single Value
// ============================================================================

#[test]
fn test_property_generic_set_single_bool() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Bool(vec![true, false]));

    prop.set(true).unwrap();
    let values: Vec<bool> = prop.get().unwrap();
    assert_eq!(values, vec![true]);
}

#[test]
fn test_property_generic_set_single_int32() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Int32(vec![1, 2, 3]));

    prop.set(42).unwrap();
    let values: Vec<i32> = prop.get().unwrap();
    assert_eq!(values, vec![42]);
}

#[test]
fn test_property_generic_set_single_string() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::String(vec!["old".to_string()]));

    prop.set("new".to_string()).unwrap();
    let values: Vec<String> = prop.get().unwrap();
    assert_eq!(values, vec!["new"]);
}

#[test]
fn test_property_generic_set_single_float64() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Float64(vec![1.1, 2.2]));

    prop.set(99.99).unwrap();
    let values: Vec<f64> = prop.get().unwrap();
    assert_eq!(values, vec![99.99]);
}

// ============================================================================
// Generic Method Tests - set<T>() - Multiple Values Vec
// ============================================================================

#[test]
fn test_property_generic_set_vec_bool() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Bool(vec![true]));

    prop.set(vec![false, true, false]).unwrap();
    let values: Vec<bool> = prop.get().unwrap();
    assert_eq!(values, vec![false, true, false]);
}

#[test]
fn test_property_generic_set_vec_int32() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Int32(vec![1, 2, 3]));

    prop.set(vec![4, 5, 6]).unwrap();
    let values: Vec<i32> = prop.get().unwrap();
    assert_eq!(values, vec![4, 5, 6]);
}

#[test]
fn test_property_generic_set_vec_string() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::String(vec!["old".to_string()]));

    prop.set(vec!["new1".to_string(), "new2".to_string()])
        .unwrap();
    let values: Vec<String> = prop.get().unwrap();
    assert_eq!(values, vec!["new1", "new2"]);
}

#[test]
fn test_property_generic_set_vec_float64() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Float64(vec![1.1]));

    prop.set(vec![2.2, 3.3, 4.4]).unwrap();
    let values: Vec<f64> = prop.get().unwrap();
    assert_eq!(values, vec![2.2, 3.3, 4.4]);
}

// ============================================================================
// Generic Method Tests - set<T>() - Slice
// ============================================================================

#[test]
fn test_property_generic_set_slice_int32() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Int32(vec![1, 2, 3]));

    let array = [4, 5, 6, 7];
    let slice = &array[..];
    prop.set(slice).unwrap();
    let values: Vec<i32> = prop.get().unwrap();
    assert_eq!(values, vec![4, 5, 6, 7]);
}

#[test]
fn test_property_generic_set_slice_string() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::String(vec!["old".to_string()]));

    let array = ["new1".to_string(), "new2".to_string(), "new3".to_string()];
    let slice = &array[..];
    prop.set(slice).unwrap();
    let values: Vec<String> = prop.get().unwrap();
    assert_eq!(values, vec!["new1", "new2", "new3"]);
}

#[test]
fn test_property_generic_set_slice_float64() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Float64(vec![1.1, 2.2]));

    let array = [3.3, 4.4];
    let slice = &array[..];
    prop.set(slice).unwrap();
    let values: Vec<f64> = prop.get().unwrap();
    assert_eq!(values, vec![3.3, 4.4]);
}

// ============================================================================
// Generic Method Tests - add<T>() - Single Value
// ============================================================================

#[test]
fn test_property_generic_add_single_bool() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Bool(vec![true]));

    prop.add(false).unwrap();
    let values: Vec<bool> = prop.get().unwrap();
    assert_eq!(values, vec![true, false]);
}

#[test]
fn test_property_generic_add_single_int32() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Int32(vec![1, 2]));

    prop.add(3).unwrap();
    let values: Vec<i32> = prop.get().unwrap();
    assert_eq!(values, vec![1, 2, 3]);
}

#[test]
fn test_property_generic_add_single_string() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::String(vec!["hello".to_string()]));

    prop.add("world".to_string()).unwrap();
    let values: Vec<String> = prop.get().unwrap();
    assert_eq!(values, vec!["hello", "world"]);
}

#[test]
fn test_property_generic_add_single_float64() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Float64(vec![1.1]));

    prop.add(2.2).unwrap();
    let values: Vec<f64> = prop.get().unwrap();
    assert_eq!(values, vec![1.1, 2.2]);
}

// ============================================================================
// Generic Method Tests - add<T>() - Multiple Values Vec
// ============================================================================

#[test]
fn test_property_generic_add_vec_bool() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Bool(vec![true]));

    prop.add(vec![false, true]).unwrap();
    let values: Vec<bool> = prop.get().unwrap();
    assert_eq!(values, vec![true, false, true]);
}

#[test]
fn test_property_generic_add_vec_int32() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Int32(vec![1, 2]));

    prop.add(vec![3, 4]).unwrap();
    let values: Vec<i32> = prop.get().unwrap();
    assert_eq!(values, vec![1, 2, 3, 4]);
}

#[test]
fn test_property_generic_add_vec_string() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::String(vec!["hello".to_string()]));

    prop.add(vec!["world".to_string(), "rust".to_string()])
        .unwrap();
    let values: Vec<String> = prop.get().unwrap();
    assert_eq!(values, vec!["hello", "world", "rust"]);
}

#[test]
fn test_property_generic_add_vec_float64() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Float64(vec![1.1]));

    prop.add(vec![2.2, 3.3]).unwrap();
    let values: Vec<f64> = prop.get().unwrap();
    assert_eq!(values, vec![1.1, 2.2, 3.3]);
}

// ============================================================================
// Generic Method Tests - add<T>() - Slice
// ============================================================================

#[test]
fn test_property_generic_add_slice_int32() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Int32(vec![1, 2]));

    let array = [3, 4, 5];
    let slice = &array[..];
    prop.add(slice).unwrap();
    let values: Vec<i32> = prop.get().unwrap();
    assert_eq!(values, vec![1, 2, 3, 4, 5]);
}

#[test]
fn test_property_generic_add_slice_string() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::String(vec!["hello".to_string()]));

    let array = ["world".to_string(), "rust".to_string()];
    let slice = &array[..];
    prop.add(slice).unwrap();
    let values: Vec<String> = prop.get().unwrap();
    assert_eq!(values, vec!["hello", "world", "rust"]);
}

#[test]
fn test_property_generic_add_slice_float64() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Float64(vec![1.1]));

    let array = [2.2, 3.3];
    let slice = &array[..];
    prop.add(slice).unwrap();
    let values: Vec<f64> = prop.get().unwrap();
    assert_eq!(values, vec![1.1, 2.2, 3.3]);
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_property_empty_after_clear() {
    let mut prop = Property::new("test");
    prop.set_value(MultiValues::Int32(vec![1, 2, 3, 4, 5]));

    assert_eq!(prop.count(), 5);
    prop.clear();
    assert_eq!(prop.count(), 0);
    assert!(prop.is_empty());
    assert_eq!(prop.data_type(), DataType::Int32); // Type remains unchanged
}

#[test]
fn test_property_large_collection() {
    let mut prop = Property::new("test");
    let large_vec: Vec<i32> = (1..=1000).collect();
    prop.set_value(MultiValues::Int32(large_vec.clone()));

    assert_eq!(prop.count(), 1000);
    let values = prop.get_int32s().unwrap();
    assert_eq!(values.len(), 1000);
    assert_eq!(values[0], 1);
    assert_eq!(values[999], 1000);
}

#[test]
fn test_property_clone() {
    let mut prop1 = Property::new("test");
    prop1.set_value(MultiValues::String(vec![
        "hello".to_string(),
        "world".to_string(),
    ]));
    prop1.set_description(Some("Test property".to_string()));
    prop1.set_final(true);

    let prop2 = prop1.clone();

    assert_eq!(prop1.name(), prop2.name());
    assert_eq!(prop1.count(), prop2.count());
    assert_eq!(prop1.data_type(), prop2.data_type());
    assert_eq!(prop1.description(), prop2.description());
    assert_eq!(prop1.is_final(), prop2.is_final());
}

#[test]
fn test_property_debug_format() {
    let mut prop = Property::new("test.property");
    prop.set_value(MultiValues::Int32(vec![42, 43]));
    prop.set_description(Some("Test property".to_string()));

    let debug_str = format!("{:?}", prop);
    assert!(debug_str.contains("test.property"));
    assert!(debug_str.contains("Int32"));
    assert!(debug_str.contains("42"));
    assert!(debug_str.contains("43"));
}

#[test]
fn test_property_partial_eq() {
    let mut prop1 = Property::new("test");
    prop1.set_value(MultiValues::Int32(vec![42, 43]));
    prop1.set_description(Some("Test".to_string()));

    let mut prop2 = Property::new("test");
    prop2.set_value(MultiValues::Int32(vec![42, 43]));
    prop2.set_description(Some("Test".to_string()));

    assert_eq!(prop1, prop2);

    // Modify one of them
    prop2.set_value(MultiValues::Int32(vec![44, 45]));
    assert_ne!(prop1, prop2);
}
