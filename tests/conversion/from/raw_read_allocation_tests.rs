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
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Mutex;

use qubit_config::Config;
use qubit_config::ConfigReader;

struct CountingAllocator;

static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static ALLOCATION_TEST_LOCK: Mutex<()> = Mutex::new(());

#[global_allocator]
static GLOBAL_ALLOCATOR: CountingAllocator = CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) }
    }
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
    let _: u32 = config
        .get("port")
        .expect("the warm-up conversion should succeed");

    ALLOCATIONS.store(0, Ordering::Relaxed);
    let value: u32 = config
        .get("port")
        .expect("the raw conversion should succeed");
    let allocations = ALLOCATIONS.load(Ordering::Relaxed);

    assert_eq!(value, 8080);
    assert_eq!(
        allocations, 0,
        "raw scalar conversion should not allocate a cloned source string",
    );
}

#[test]
fn test_root_reader_prefix_iteration_does_not_box() {
    let _guard = ALLOCATION_TEST_LOCK
        .lock()
        .expect("the allocation test lock should not be poisoned");
    let mut config = Config::new();
    config
        .set("http.host", "localhost")
        .expect("the test configuration should accept the value");
    assert_eq!(
        <Config as ConfigReader>::iter_prefix(&config, "http.").count(),
        1,
    );

    ALLOCATIONS.store(0, Ordering::Relaxed);
    let count =
        <Config as ConfigReader>::iter_prefix(&config, "http.").count();
    let allocations = ALLOCATIONS.load(Ordering::Relaxed);

    assert_eq!(count, 1);
    assert_eq!(
        allocations, 0,
        "root prefix iteration should not allocate a boxed iterator",
    );
}
