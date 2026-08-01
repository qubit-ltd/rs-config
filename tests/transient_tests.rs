use std::collections::hash_map::DefaultHasher;
use std::hash::{
    Hash,
    Hasher,
};

use qubit_config::Transient;

#[test]
fn transient_ignores_inner_value_for_equality_and_hash() {
    let first = Transient::new(1_u32);
    let second = Transient::new(2_u32);

    assert_eq!(first, second);

    let mut first_hasher = DefaultHasher::new();
    first.hash(&mut first_hasher);
    let mut second_hasher = DefaultHasher::new();
    second.hash(&mut second_hasher);
    assert_eq!(first_hasher.finish(), second_hasher.finish());
}

#[test]
fn transient_preserves_inner_value_through_access_and_clone() {
    let mut value = Transient::new(String::from("runtime"));
    assert_eq!(value.get(), "runtime");
    value.get_mut().push_str(" state");

    let cloned = value.clone();
    assert_eq!(value.into_inner(), "runtime state");
    assert_eq!(cloned.into_inner(), "runtime state");
}
