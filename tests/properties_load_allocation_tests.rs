// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Isolated allocator evidence for properties parse versus source loading.

use std::alloc::GlobalAlloc;
use std::alloc::Layout;
use std::alloc::System;
use std::hint::black_box;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use qubit_config::Config;
use qubit_config::ConfigSource;
use qubit_config::source::PropertiesConfigSource;

struct CountingAllocator;

static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn account_alloc(size: usize) {
    ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
    let live = LIVE.fetch_add(size, Ordering::Relaxed) + size;
    PEAK.fetch_max(live, Ordering::Relaxed);
}

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            account_alloc(layout.size());
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_ptr = unsafe { System.realloc(ptr, layout, new_size) };
        if !new_ptr.is_null() {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            let live = LIVE.fetch_add(new_size, Ordering::Relaxed) - layout.size() + new_size;
            LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
            PEAK.fetch_max(live, Ordering::Relaxed);
        }
        new_ptr
    }
}

#[derive(Debug)]
struct Sample {
    allocations: usize,
    peak_live_bytes: usize,
}

fn measure(action: impl FnOnce()) -> Sample {
    ALLOCATIONS.store(0, Ordering::Relaxed);
    PEAK.store(LIVE.load(Ordering::Relaxed), Ordering::Relaxed);
    action();
    Sample {
        allocations: ALLOCATIONS.load(Ordering::Relaxed),
        peak_live_bytes: PEAK.load(Ordering::Relaxed),
    }
}

fn legacy_collect_then_load(content: &str) -> Config {
    let pairs = PropertiesConfigSource::parse_content(content).unwrap();
    let mut config = Config::new();
    for (key, value) in pairs {
        config.set(&key, value).unwrap();
    }
    config
}

#[test]
fn test_reports_isolated_repeated_and_distinct_key_allocation_samples() {
    for repeated in [true, false] {
        let content = (0..1_024)
            .map(|index| {
                let key = if repeated {
                    "service.shared".to_string()
                } else {
                    format!("service.key_{index}")
                };
                format!("{key}={index}\n")
            })
            .collect::<String>();
        let source = PropertiesConfigSource::from_content(content.clone());

        // Warm runtime paths before measuring. This binary contains only this
        // one test, so no other test thread can contaminate the interval.
        let _ = black_box(source.load().unwrap());
        let legacy = measure(|| {
            let _ = black_box(legacy_collect_then_load(&content));
        });
        let direct = measure(|| {
            let _ = black_box(source.load().unwrap());
        });
        eprintln!(
            "properties allocations keys={} count=1024 legacy={legacy:?} direct={direct:?}",
            if repeated { "repeated" } else { "distinct" },
        );
        assert!(legacy.allocations >= direct.allocations);
        assert!(legacy.peak_live_bytes >= direct.peak_live_bytes);
    }
}
