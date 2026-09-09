// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Changed string leaves are owned separately from the borrowed input arena.

use std::collections::BTreeMap;

use qubit_datatype::DataConversionError;
use qubit_datatype::DataType;
use qubit_value::ValueError;
use qubit_value::ValueRef;

use super::node::ReadNode;
use super::node::SourceLocation;
use super::prepared_read::PreparedConfigRead;
use super::traversal;
use crate::ConfigError;
use crate::ConfigReader;
use crate::ConfigResult;
use crate::ReadPolicy;
use crate::config_reader::root_config;

pub(super) fn interpolate<R: ConfigReader + ?Sized>(
    prepared: &PreparedConfigRead<'_>,
    primary: &R,
) -> ConfigResult<BTreeMap<SourceLocation, String>> {
    let mut overlays = BTreeMap::new();
    let options = primary.read_policy();
    let fallback = root_config(primary);
    traversal::walk(prepared, true, |visit| {
        let Some(text) = visit.value.string() else {
            return Ok(());
        };
        if !text.contains("${") {
            // Preserve the resolver's output bound even when no replacement is
            // necessary, without allocating a duplicate of an unchanged string.
            if text.len() > options.max_interpolation_output_bytes() {
                return Err(ConfigError::SubstitutionOutputTooLarge {
                    path: visit.path.to_string(),
                    max_output_bytes: options.max_interpolation_output_bytes(),
                });
            }
            return Ok(());
        }
        let changed = crate::utils::substitute_variables_with_fallback(text, primary, fallback, options, &visit.path)?;
        if changed != text {
            let location = visit
                .location
                .clone()
                .expect("a source string always has a storage location");
            overlays.insert(location, changed);
        }
        Ok(())
    })?;
    Ok(overlays)
}

/// Missing scalar properties are classified after complete source admission.
/// Strings nested in JSON, maps, or collections keep their original positions.
pub(super) fn classify_missing(
    prepared: &mut PreparedConfigRead<'_>,
    options: &ReadPolicy,
    exact: bool,
) -> ConfigResult<()> {
    let node_count = prepared.nodes.len();
    let mut omitted: Option<Vec<bool>> = None;
    for (id, node) in prepared.nodes.iter_mut().enumerate() {
        let ReadNode::Scalar(ValueRef::String(original)) = node else {
            continue;
        };
        let Some(origin) = prepared.sources.origin(id) else {
            continue;
        };
        if !origin.segments.is_empty() {
            continue;
        }
        let text = prepared.overlays.get(origin).map_or(*original, String::as_str);
        let path = prepared
            .sources
            .property(origin.property_index)
            .expect("registered property")
            .name();
        match options.conversion_policy().string().normalize_optional(text) {
            Ok(Some(_)) => {}
            Ok(None) => {
                let error = ValueError::from(DataConversionError::missing(DataType::String, DataType::String));
                *node = ReadNode::Missing(error.missing().expect("missing conversion").clone());
                if !exact {
                    omitted.get_or_insert_with(|| vec![false; node_count])[id] = true;
                }
            }
            Err(error) => {
                return Err(ConfigError::from_data_conversion_error(
                    path,
                    error.into_data_conversion_error(DataType::String),
                ));
            }
        }
    }
    // Arena children are allocated after their parents, including lazily
    // expanded borrowed objects. Reverse order removes empty synthetic parents.
    let Some(mut omitted) = omitted else {
        return Ok(());
    };
    for id in (0..prepared.nodes.len()).rev() {
        if let ReadNode::Object(children) = &mut prepared.nodes[id] {
            children.retain(|_, child| !omitted[*child]);
            if children.is_empty() && prepared.sources.origin(id).is_none() && id != prepared.root {
                omitted[id] = true;
            }
        }
    }
    Ok(())
}
