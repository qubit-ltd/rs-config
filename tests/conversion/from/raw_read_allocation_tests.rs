// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Allocation regressions for raw configuration reads.

use std::alloc::GlobalAlloc;
use std::alloc::Layout;
use std::alloc::System;
use std::cell::Cell;
use std::sync::Mutex;

use qubit_config::Config;
use qubit_config::ConfigReader;

#[derive(Clone, Copy, Default)]
struct AllocationCounts {
    count: usize,
    bytes: usize,
}

struct CountingAllocator;

static ALLOCATION_TEST_LOCK: Mutex<()> = Mutex::new(());

thread_local! {
    static TEST_THREAD_ALLOCATIONS: Cell<AllocationCounts> = const { Cell::new(AllocationCounts { count: 0, bytes: 0 }) };
}

/// Resets the allocation counter for the current test thread.
fn reset_test_thread_allocations() {
    TEST_THREAD_ALLOCATIONS.with(|allocations| allocations.set(AllocationCounts::default()));
}

/// Returns the allocations observed on the current test thread.
fn test_thread_allocations() -> usize {
    TEST_THREAD_ALLOCATIONS.with(|allocations| allocations.get().count)
}

fn test_thread_allocation_bytes() -> usize {
    TEST_THREAD_ALLOCATIONS.with(|allocations| allocations.get().bytes)
}

#[global_allocator]
static GLOBAL_ALLOCATOR: CountingAllocator = CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        TEST_THREAD_ALLOCATIONS.with(|allocations| {
            let counts = allocations.get();
            allocations.set(AllocationCounts {
                count: counts.count.saturating_add(1),
                bytes: counts.bytes.saturating_add(layout.size()),
            });
        });
        unsafe { System.alloc(layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        TEST_THREAD_ALLOCATIONS.with(|allocations| {
            let counts = allocations.get();
            allocations.set(AllocationCounts {
                count: counts.count.saturating_add(1),
                bytes: counts.bytes.saturating_add(new_size),
            });
        });
        unsafe { System.realloc(pointer, layout, new_size) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) }
    }
}

#[test]
fn test_raw_string_read_avoids_an_extra_source_text_allocation() {
    let _guard = ALLOCATION_TEST_LOCK
        .lock()
        .expect("the allocation test lock should not be poisoned");
    let source = "x".repeat(1_024);
    let mut config = Config::new();
    config.set("text", source.as_str()).expect("set text");
    let property = config.get_property("text").unwrap().unwrap();
    let policy = config.read_policy();
    let _: String = config.get("text").expect("warm up raw read");
    let _: String = property
        .value()
        .to_first_with(policy.conversion_policy(), policy.conversion_limits())
        .expect("warm up direct conversion");

    reset_test_thread_allocations();
    let raw: String = config.get("text").expect("raw string read");
    let raw_bytes = test_thread_allocation_bytes();

    reset_test_thread_allocations();
    let direct: String = property
        .value()
        .to_first_with(policy.conversion_policy(), policy.conversion_limits())
        .expect("direct conversion");
    let direct_bytes = test_thread_allocation_bytes();

    assert_eq!(raw, direct);
    assert_eq!(
        raw_bytes, direct_bytes,
        "raw reads should allocate only the returned string"
    );
}

#[test]
fn test_raw_scalar_conversion_does_not_clone_source_text() {
    let _guard = ALLOCATION_TEST_LOCK
        .lock()
        .expect("the allocation test lock should not be poisoned");
    let mut config = Config::new();
    config
        .set("port", "8080")
        .expect("the test configuration should accept the value");
    let _: u32 = config.get("port").expect("the warm-up conversion should succeed");

    reset_test_thread_allocations();
    let value: u32 = config.get("port").expect("the raw conversion should succeed");
    let allocations = test_thread_allocations();

    assert_eq!(value, 8080);
    assert_eq!(
        allocations, 0,
        "raw scalar conversion should not allocate a cloned source string",
    );
}

/// Verifies root prefix iteration avoids boxing allocations.
#[test]
fn test_root_reader_prefix_iteration_does_not_box() {
    let _guard = ALLOCATION_TEST_LOCK
        .lock()
        .expect("the allocation test lock should not be poisoned");
    let mut config = Config::new();
    config
        .set("http.host", "localhost")
        .expect("the test configuration should accept the value");
    assert_eq!(<Config as ConfigReader>::iter_prefix(&config, "http.").count(), 1,);

    reset_test_thread_allocations();
    let count = <Config as ConfigReader>::iter_prefix(&config, "http.").count();
    let allocations = test_thread_allocations();

    assert_eq!(count, 1);
    assert_eq!(
        allocations, 0,
        "root prefix iteration should not allocate a boxed iterator",
    );
}
