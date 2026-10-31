// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Exact reads keep their single source inline; subtrees use indexed metadata.

use super::node::NodeId;
use super::node::SourceLocation;
use crate::Property;

/// Borrowed property table and per-node source origins for a prepared read.
///
/// `NodeId` values index the node arena, while property indexes address the
/// separate `properties` table; they are distinct index domains. In the
/// exact-property form, only node and property index zero are valid. In the
/// tree form, `origins` has one slot for each arena node, and `properties`
/// maps property indexes to the source properties that contributed leaves.
/// Synthetic path nodes can have no origin. Returned property
/// references borrow the original configuration for the prepared read's
/// lifetime.
#[derive(Debug)]
pub(super) enum SourceIndex<'a> {
    /// One exact property is the complete input.
    Exact {
        /// Exact property source.
        property: &'a Property,
        /// Location of the exact property.
        origin: SourceLocation,
    },
    /// Indexed subtree with property and origin tables.
    Tree {
        /// Properties contributing to indexed nodes.
        properties: Vec<&'a Property>,
        /// Optional source locations for indexed nodes.
        origins: Vec<Option<SourceLocation>>,
    },
}

impl<'a> SourceIndex<'a> {
    /// Creates an empty subtree index.
    pub(super) fn tree() -> Self {
        Self::Tree {
            properties: Vec::new(),
            origins: Vec::new(),
        }
    }

    /// Creates an index for one exact property.
    pub(super) fn exact(property: &'a Property) -> Self {
        Self::Exact {
            property,
            origin: SourceLocation {
                property_index: 0,
                segments: Vec::new(),
            },
        }
    }

    /// Returns whether this index represents one property read directly.
    ///
    /// Exact indexes address only node and property index zero; subtree reads
    /// use the tree variant instead.
    pub(super) fn is_exact(&self) -> bool {
        matches!(self, Self::Exact { .. })
    }

    /// Returns the node's origin, or `None` for synthetic/out-of-range nodes.
    pub(super) fn origin(&self, id: NodeId) -> Option<&SourceLocation> {
        match self {
            Self::Exact { origin, .. } => (id == 0).then_some(origin),
            Self::Tree { origins, .. } => origins.get(id).and_then(Option::as_ref),
        }
    }

    /// Returns the source property at `index`, or `None` when out of range.
    pub(super) fn property(&self, index: usize) -> Option<&'a Property> {
        match self {
            Self::Exact { property, .. } => (index == 0).then_some(*property),
            Self::Tree { properties, .. } => properties.get(index).copied(),
        }
    }

    /// Returns the number of source properties addressable by property index.
    ///
    /// This is not the number of arena nodes; use the prepared node arena for
    /// that count.
    pub(super) fn len(&self) -> usize {
        match self {
            Self::Exact { .. } => 1,
            Self::Tree { properties, .. } => properties.len(),
        }
    }

    /// Appends the origin slot for the next arena node in a subtree index.
    ///
    /// Call this once, in arena-node insertion order, so a node ID addresses
    /// the matching origin slot. `None` records a synthetic node. This is
    /// valid only for [`Self::Tree`]; calling it on [`Self::Exact`] panics.
    pub(super) fn push_origin(&mut self, origin: Option<SourceLocation>) {
        let Self::Tree { origins, .. } = self else {
            unreachable!("only subtree construction appends nodes")
        };
        origins.push(origin);
    }

    /// Adds a source property to the subtree property's table.
    ///
    /// Property indexes address this table and are distinct from arena node
    /// IDs. Append properties in the order used by their source locations;
    /// this is valid only for [`Self::Tree`], and panics on [`Self::Exact`].
    pub(super) fn push_property(&mut self, property: &'a Property) {
        let Self::Tree { properties, .. } = self else {
            unreachable!("only subtree construction appends properties")
        };
        properties.push(property);
    }
}
