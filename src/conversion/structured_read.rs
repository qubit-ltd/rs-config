// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Borrowed input preparation and direct structured configuration reads.

use std::cell::RefCell;

use qubit_datatype::ConversionSession;

#[path = "structured_read/conversion_input.rs"]
mod conversion_input;
#[path = "structured_read/source_deserializer.rs"]
mod deserializer;
#[path = "structured_read/source_enum_access.rs"]
mod enum_access;
#[path = "structured_read/internal/mod.rs"]
mod internal;
#[path = "structured_read/interpolation_overlay.rs"]
mod interpolation_overlay;
#[path = "structured_read/source_map_access.rs"]
mod map_access;
#[path = "structured_read/map_entries.rs"]
mod map_entries;
#[path = "structured_read/read_node.rs"]
mod node;
#[path = "structured_read/path.rs"]
mod path;
#[path = "structured_read/prepared_input_budget.rs"]
mod prepared_input_budget;
#[path = "structured_read/prepared_config_read.rs"]
mod prepared_read;
#[path = "structured_read/read_context.rs"]
mod read_context;
#[path = "structured_read/read_view.rs"]
mod read_view;
#[path = "structured_read/scalar_seq_access.rs"]
mod scalar_seq_access;
#[path = "structured_read/source_seq_access.rs"]
mod seq_access;
#[path = "structured_read/sequence_values.rs"]
mod sequence_values;
#[path = "structured_read/source_index.rs"]
mod source_index;
#[path = "structured_read/source_location.rs"]
mod source_location;
#[path = "structured_read/source_segment.rs"]
mod source_segment;
#[path = "structured_read/traversal.rs"]
mod traversal;
#[path = "structured_read/visit.rs"]
mod visit;

/// Prepares and deserializes the selected reader input with one shared
/// conversion budget.
///
/// The prefix selects a property or subtree in the reader's visible scope.
/// Preparation applies the requested interpolation policy and admits the
/// complete selected input before Serde visits it. All converted leaves share
/// one conversion session so collection limits apply across the full read.
/// When `reject_unknown` is enabled, paths reported through Serde's ignored
/// value callback are collected and returned as a sorted, deduplicated
/// [`crate::ConfigError::UnknownProperties`] error after deserialization.
/// Serde errors are mapped to configuration errors rooted at the selected
/// diagnostic path.
///
/// # Type Parameters
///
/// * `T` - Owned target type that implements Serde deserialization.
///
/// # Parameters
///
/// * `reader` - Configuration reader that supplies values and read policy.
/// * `prefix` - Reader-relative property or subtree path.
/// * `interpolate` - Whether selected string leaves are interpolated.
/// * `reject_unknown` - Whether ignored-field callbacks are reported.
///
/// # Returns
///
/// The deserialized target when preparation and conversion succeed.
///
/// # Errors
///
/// Returns preparation, conversion, Serde, or unknown-property errors.
pub(crate) fn deserialize_from_reader<R, T>(
    reader: &R,
    prefix: &str,
    interpolate: bool,
    reject_unknown: bool,
) -> crate::ConfigResult<T>
where
    R: crate::ConfigReader + ?Sized,
    T: serde::de::DeserializeOwned,
{
    let prepared = prepared_read::prepare_read(reader, prefix, interpolate)?;
    let options = reader.read_policy();
    let mut session = ConversionSession::new(options.conversion_policy(), options.conversion_limits());
    let ignored = RefCell::new(Vec::new());
    let context = deserializer::ReadContext {
        prepared: &prepared,
        ignored_paths: reject_unknown.then_some(&ignored),
    };
    let deserializer = deserializer::SourceDeserializer::new(
        context,
        traversal::ReadView::from_node(&prepared.nodes[prepared.root]),
        prepared.sources.origin(prepared.root).cloned(),
        prepared.diagnostic_root_path.clone(),
        true,
        &mut session,
    );
    let value =
        T::deserialize(deserializer).map_err(|error| error.into_config_error(&prepared.diagnostic_root_path))?;
    let mut ignored = ignored.into_inner();
    if ignored.is_empty() {
        return Ok(value);
    }
    ignored.sort();
    ignored.dedup();
    Err(crate::ConfigError::UnknownProperties { paths: ignored })
}

#[cfg(test)]
mod preparation_tests;

#[cfg(test)]
mod visitor_tests;
