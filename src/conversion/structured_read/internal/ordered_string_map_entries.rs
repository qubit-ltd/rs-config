// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Borrowed entries in deterministic raw-key order for native string maps.

use std::collections::HashMap;

/// Iterates over native string-map entries in ascending raw-key order.
///
/// This collects and sorts only borrowed key/value pairs, so payload strings
/// are never cloned. Construction takes O(n log n) time and O(n) reference
/// storage; iteration then takes O(1) per entry.
pub(crate) struct OrderedStringMapEntries<'a> {
    /// Sorted borrowed entries not yet yielded to the caller.
    entries: std::vec::IntoIter<(&'a String, &'a String)>,
}

impl<'a> OrderedStringMapEntries<'a> {
    /// Creates a sorted borrowed iterator over a native string map.
    ///
    /// # Parameters
    ///
    /// * `values` - String map whose keys and values are borrowed.
    pub(crate) fn new(values: &'a HashMap<String, String>) -> Self {
        let mut entries: Vec<_> = values.iter().collect();
        entries.sort_unstable_by(|left, right| left.0.cmp(right.0));
        Self {
            entries: entries.into_iter(),
        }
    }
}

impl<'a> Iterator for OrderedStringMapEntries<'a> {
    /// Borrowed key and value from the source map.
    type Item = (&'a String, &'a String);

    /// Returns the next entry in ascending raw-key order.
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.entries.next()
    }

    /// Returns lower and upper bounds for the number of remaining entries.
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.entries.size_hint()
    }
}

impl ExactSizeIterator for OrderedStringMapEntries<'_> {
    /// Returns the exact number of entries not yet yielded.
    #[inline]
    fn len(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::OrderedStringMapEntries;

    #[test]
    fn test_yields_sorted_borrowed_entries_and_exact_remaining_length() {
        let values = HashMap::from([
            ("z".to_owned(), "last".to_owned()),
            ("a.b".to_owned(), "dotted".to_owned()),
            ("界".to_owned(), "unicode".to_owned()),
            ("a[0]".to_owned(), "bracketed".to_owned()),
            ("".to_owned(), "empty".to_owned()),
        ]);
        let mut entries = OrderedStringMapEntries::new(&values);
        let collected = entries.by_ref().map(|(key, _)| key.as_str()).collect::<Vec<_>>();
        assert_eq!(collected, ["", "a.b", "a[0]", "z", "界"]);

        let mut entries = OrderedStringMapEntries::new(&values);
        assert_eq!(entries.len(), values.len());
        for (index, expected) in ["", "a.b", "a[0]", "z", "界"].into_iter().enumerate() {
            let (key, value) = entries.next().unwrap();
            assert_eq!(key, expected);
            let (source_key, source_value) = values.get_key_value(key).unwrap();
            assert!(std::ptr::eq(key, source_key));
            assert!(std::ptr::eq(value, source_value));
            assert_eq!(entries.len(), values.len() - index - 1);
        }
        assert!(entries.next().is_none());
        assert_eq!(entries.len(), 0);
    }

    #[test]
    fn test_handles_empty_and_single_entry_maps() {
        let empty = HashMap::new();
        assert_eq!(OrderedStringMapEntries::new(&empty).len(), 0);
        let one = HashMap::from([("only".to_owned(), "value".to_owned())]);
        let mut entries = OrderedStringMapEntries::new(&one);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries.next().unwrap().0, "only");
        assert_eq!(entries.len(), 0);
    }
}
