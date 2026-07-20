// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! # Configuration Property
//!
//! Defines the property structure for configuration items, including name,
//! value, description, and other information.

use serde::{
    Deserialize,
    Serialize,
};
use std::fmt::{
    self,
    Debug,
    Formatter,
};
use std::ops::{
    Deref,
    DerefMut,
};

use qubit_datatype::DataType;
use qubit_redact::redacted_debug;
use qubit_value::ValueContainer;

/// Configuration Property
///
/// Represents a configuration item: name, value, description, and whether it is
/// final.
///
/// # Features
///
/// - Supports multi-value configuration
/// - Supports description information
/// - Supports final value marking (final properties cannot be overridden)
/// - Supports serialization and deserialization
///
/// # Examples
///
/// ```rust
/// use qubit_config::Property;
///
/// let mut port = Property::new("port", 8080);
/// port.set_description(Some("Server port".to_string()));
/// assert_eq!(port.name(), "port");
/// assert_eq!(port.count(), 1);
///
/// let mut code = Property::new("code", 42u8);
/// code.add(1u8).unwrap();
/// assert_eq!(code.count(), 2);
/// ```
#[must_use]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Property {
    /// Property name
    name: String,
    /// Property value
    value: ValueContainer,
    /// Property description
    description: Option<String>,
    /// Whether this is a final value (cannot be overridden)
    is_final: bool,
}

impl Debug for Property {
    /// Formats property metadata while redacting the stored value.
    #[inline]
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Property")
            .field("name", &self.name)
            .field("value", &redacted_debug(&self.value))
            .field("description", &self.description)
            .field("is_final", &self.is_final)
            .finish()
    }
}

impl Property {
    /// Creates a new property
    ///
    /// Creates a property with its initial value and shape.
    ///
    /// # Parameters
    ///
    /// * `name` - Property name
    /// * `value` - Initial scalar or collection value
    ///
    /// # Returns
    ///
    /// Returns a new property instance
    ///
    /// # Examples
    ///
    /// ```rust
    /// use qubit_config::Property;
    ///
    /// let prop = Property::new("server.port", 8080);
    /// assert_eq!(prop.name(), "server.port");
    /// assert_eq!(prop.count(), 1);
    /// ```
    #[inline]
    pub fn new(
        name: impl Into<String>,
        value: impl Into<ValueContainer>,
    ) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            description: None,
            is_final: false,
        }
    }

    /// Gets the property name
    ///
    /// # Returns
    ///
    /// Returns the property name as a string slice
    #[inline(always)]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Gets a reference to the property value
    ///
    /// # Returns
    ///
    /// Returns a reference to the property value
    #[inline(always)]
    pub fn value(&self) -> &ValueContainer {
        &self.value
    }

    /// Gets a mutable reference to the property value
    ///
    /// # Returns
    ///
    /// Returns a mutable reference to the property value
    #[inline(always)]
    pub fn value_mut(&mut self) -> &mut ValueContainer {
        &mut self.value
    }

    /// Sets the property value
    ///
    /// # Parameters
    ///
    /// * `value` - New property value
    #[inline(always)]
    pub fn set_value(&mut self, value: impl Into<ValueContainer>) {
        self.value = value.into();
    }

    /// Gets the property description
    ///
    /// # Returns
    ///
    /// Returns the property description as Option
    #[inline(always)]
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Sets the property description
    ///
    /// # Parameters
    ///
    /// * `description` - Property description
    #[inline(always)]
    pub fn set_description(&mut self, description: Option<String>) {
        self.description = description;
    }

    /// Checks if this is a final value
    ///
    /// # Returns
    ///
    /// Returns `true` if the property is final
    #[inline(always)]
    pub fn is_final(&self) -> bool {
        self.is_final
    }

    /// Sets whether this is a final value
    ///
    /// # Parameters
    ///
    /// * `is_final` - Whether this is final
    #[inline(always)]
    pub fn set_final(&mut self, is_final: bool) {
        self.is_final = is_final;
    }

    /// Gets the data type
    ///
    /// # Returns
    ///
    /// Returns the data type of the property value
    #[inline(always)]
    pub fn data_type(&self) -> DataType {
        self.value.data_type()
    }

    /// Gets the number of values
    ///
    /// # Returns
    ///
    /// Returns the number of values in the property
    #[inline(always)]
    pub fn count(&self) -> usize {
        self.value.count()
    }

    /// Checks if the property is empty
    ///
    /// # Returns
    ///
    /// Returns `true` for both an unset value and a concrete empty collection.
    /// Use [`ValueContainer::is_unset`] when those states must be
    /// distinguished.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.value.count() == 0
    }

    /// Clears the property value
    ///
    /// Clears all values in the property but keeps type information
    #[inline(always)]
    pub fn clear(&mut self) {
        self.value.clear();
    }

    /// Clones this property with a replacement name.
    ///
    /// # Parameters
    ///
    /// * `name` - Name assigned to the cloned property.
    ///
    /// # Returns
    ///
    /// A clone that preserves the value and metadata under the new name.
    #[inline]
    pub(crate) fn renamed(&self, name: impl Into<String>) -> Self {
        let mut property = self.clone();
        property.name = name.into();
        property
    }
}

impl Deref for Property {
    type Target = ValueContainer;

    /// Dereferences to [`ValueContainer`].
    ///
    /// Allows direct access to scalar-or-collection operations.
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl DerefMut for Property {
    /// Mutably dereferences to [`ValueContainer`].
    ///
    /// Allows direct mutable access to scalar-or-collection operations.
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}
