/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! # Configuration Property
//!
//! Defines the property structure for configuration items, including name,
//! value, description, and other information.
//!
//! # Author
//!
//! Haixing Hu

use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut};

use qubit_common::DataType;
use qubit_value::MultiValues;

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
/// let mut port = Property::new("port");
/// port.set(8080).unwrap();  // Generic method, type auto-inferred
/// port.set_description(Some("Server port".to_string()));
/// assert_eq!(port.name(), "port");
/// assert_eq!(port.count(), 1);
///
/// let mut code = Property::new("code");
/// code.set(42u8).unwrap();  // Generic set, inferred as u8
/// code.add(1u8).unwrap();
/// assert_eq!(code.count(), 2);
/// ```
///
/// # Author
///
/// Haixing Hu
///
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Property {
    /// Property name
    name: String,
    /// Property value
    value: MultiValues,
    /// Property description
    description: Option<String>,
    /// Whether this is a final value (cannot be overridden)
    is_final: bool,
}

impl Property {
    /// Creates a new property
    ///
    /// Creates an empty property with an initial value of an empty i32 list.
    ///
    /// # Parameters
    ///
    /// * `name` - Property name
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
    /// let prop = Property::new("server.port");
    /// assert_eq!(prop.name(), "server.port");
    /// assert!(prop.is_empty());
    /// ```
    #[inline]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: MultiValues::Empty(DataType::Int32),
            description: None,
            is_final: false,
        }
    }

    /// Creates a property with a value
    ///
    /// # Parameters
    ///
    /// * `name` - Property name
    /// * `value` - Property value
    ///
    /// # Returns
    ///
    /// Returns a new property instance
    ///
    /// # Examples
    ///
    /// ```rust
    /// use qubit_config::Property;
    /// use qubit_value::MultiValues;
    ///
    /// let prop = Property::with_value("port", MultiValues::Int32(vec![8080]));
    /// assert_eq!(prop.name(), "port");
    /// assert_eq!(prop.count(), 1);
    /// ```
    #[inline]
    pub fn with_value(name: impl Into<String>, value: MultiValues) -> Self {
        Self {
            name: name.into(),
            value,
            description: None,
            is_final: false,
        }
    }

    /// Gets the property name
    ///
    /// # Returns
    ///
    /// Returns the property name as a string slice
    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Gets a reference to the property value
    ///
    /// # Returns
    ///
    /// Returns a reference to the property value
    #[inline]
    pub fn value(&self) -> &MultiValues {
        &self.value
    }

    /// Gets a mutable reference to the property value
    ///
    /// # Returns
    ///
    /// Returns a mutable reference to the property value
    #[inline]
    pub fn value_mut(&mut self) -> &mut MultiValues {
        &mut self.value
    }

    /// Sets the property value
    ///
    /// # Parameters
    ///
    /// * `value` - New property value
    #[inline]
    pub fn set_value(&mut self, value: MultiValues) {
        self.value = value;
    }

    /// Gets the property description
    ///
    /// # Returns
    ///
    /// Returns the property description as Option
    #[inline]
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Sets the property description
    ///
    /// # Parameters
    ///
    /// * `description` - Property description
    #[inline]
    pub fn set_description(&mut self, description: Option<String>) {
        self.description = description;
    }

    /// Checks if this is a final value
    ///
    /// # Returns
    ///
    /// Returns `true` if the property is final
    #[inline]
    pub fn is_final(&self) -> bool {
        self.is_final
    }

    /// Sets whether this is a final value
    ///
    /// # Parameters
    ///
    /// * `is_final` - Whether this is final
    #[inline]
    pub fn set_final(&mut self, is_final: bool) {
        self.is_final = is_final;
    }

    /// Gets the data type
    ///
    /// # Returns
    ///
    /// Returns the data type of the property value
    #[inline]
    pub fn data_type(&self) -> DataType {
        self.value.data_type()
    }

    /// Gets the number of values
    ///
    /// # Returns
    ///
    /// Returns the number of values in the property
    #[inline]
    pub fn count(&self) -> usize {
        self.value.count()
    }

    /// Checks if the property is empty
    ///
    /// # Returns
    ///
    /// Returns `true` if the property contains no values
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    /// Clears the property value
    ///
    /// Clears all values in the property but keeps type information
    #[inline]
    pub fn clear(&mut self) {
        self.value.clear();
    }
}

impl Deref for Property {
    type Target = MultiValues;

    /// Dereferences to MultiValues
    ///
    /// Allows direct access to all MultiValues methods
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl DerefMut for Property {
    /// Mutably dereferences to MultiValues
    ///
    /// Allows direct mutable access to all MultiValues methods
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}
