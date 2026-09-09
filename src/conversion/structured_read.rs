// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Borrowed input preparation and direct structured configuration reads.

use qubit_datatype::ConversionSession;

#[path = "structured_read/conversion_input.rs"]
mod conversion_input;
#[path = "structured_read/source_deserializer.rs"]
mod deserializer;
#[path = "structured_read/source_enum_access.rs"]
mod enum_access;
#[path = "structured_read/input_budget.rs"]
mod input_budget;
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

/// Prepares one borrowed input and shares a conversion session across all
/// leaves.
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
    let context = deserializer::ReadContext { prepared: &prepared };
    let deserializer = deserializer::SourceDeserializer::new(
        context,
        traversal::ReadView::from_node(&prepared.nodes[prepared.root]),
        prepared.sources.origin(prepared.root).cloned(),
        prepared.root_path.clone(),
        true,
        &mut session,
    );
    let mut ignored = Vec::new();
    let result = if reject_unknown {
        serde_ignored::deserialize(deserializer, |path| ignored.push(path.to_string()))
    } else {
        T::deserialize(deserializer)
    };
    let value = result.map_err(|error| error.into_config_error(&prepared.root_path))?;
    if ignored.is_empty() {
        return Ok(value);
    }
    ignored.sort();
    ignored.dedup();
    let paths = ignored
        .into_iter()
        .map(|path| {
            let path = path.trim_start_matches('.');
            if prepared.root_path.is_empty() {
                path.to_owned()
            } else if path.is_empty() {
                prepared.root_path.clone()
            } else {
                format!("{}.{path}", prepared.root_path)
            }
        })
        .collect();
    Err(crate::ConfigError::UnknownProperties { paths })
}

#[cfg(test)]
#[path = "../../tests/conversion/structured_read_preparation_tests.rs"]
mod preparation_tests;

#[cfg(test)]
#[path = "../../tests/conversion/structured_read_visitor_tests.rs"]
mod visitor_tests;
