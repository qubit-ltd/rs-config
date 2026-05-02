/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! # Mutable Configuration Property Guard
//!
//! Provides guarded mutable access to non-final configuration properties.
//!

use std::ops::Deref;

use qubit_value::MultiValues;
use qubit_value::multi_values::{
    MultiValuesAddArg, MultiValuesAdder, MultiValuesMultiAdder, MultiValuesSetArg,
    MultiValuesSetter, MultiValuesSetterSlice, MultiValuesSingleSetter,
};

use crate::{ConfigError, ConfigResult, Property};

/// Guarded mutable access to a non-final [`Property`] stored in a
/// [`crate::Config`].
///
/// This wrapper deliberately exposes read-only deref to [`Property`], but not
/// `DerefMut`. Value-changing operations re-check the property's final flag on
/// every call, so setting a property final through the guard immediately blocks
/// subsequent mutation through the same guard.
///
pub struct ConfigPropertyMut<'a> {
    property: &'a mut Property,
}

impl<'a> ConfigPropertyMut<'a> {
    /// Creates a guarded mutable property reference.
    ///
    /// # Parameters
    ///
    /// * `property` - The property to guard.
    ///
    /// # Returns
    ///
    /// A mutable guard for `property`.
    #[inline]
    pub(crate) fn new(property: &'a mut Property) -> Self {
        Self { property }
    }

    /// Returns the underlying property as a read-only reference.
    ///
    /// # Returns
    ///
    /// The guarded property.
    #[inline]
    pub fn as_property(&self) -> &Property {
        self.property
    }

    /// Sets the property description when the property is not final.
    ///
    /// # Parameters
    ///
    /// * `description` - New property description.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::PropertyIsFinal`] if the property has already
    /// been marked final.
    #[inline]
    pub fn set_description(&mut self, description: Option<String>) -> ConfigResult<()> {
        self.ensure_not_final()?;
        self.property.set_description(description);
        Ok(())
    }

    /// Sets whether the property is final.
    ///
    /// Marking a non-final property as final succeeds. A property that is
    /// already final may be marked final again, but cannot be unset through
    /// this guard.
    ///
    /// # Parameters
    ///
    /// * `is_final` - Whether the property should be final.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::PropertyIsFinal`] when trying to unset an
    /// already-final property.
    #[inline]
    pub fn set_final(&mut self, is_final: bool) -> ConfigResult<()> {
        if self.property.is_final() && !is_final {
            return Err(ConfigError::PropertyIsFinal(
                self.property.name().to_string(),
            ));
        }
        self.property.set_final(is_final);
        Ok(())
    }

    /// Replaces the property value when the property is not final.
    ///
    /// # Parameters
    ///
    /// * `value` - New property value.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::PropertyIsFinal`] if the property has already
    /// been marked final.
    #[inline]
    pub fn set_value(&mut self, value: MultiValues) -> ConfigResult<()> {
        self.ensure_not_final()?;
        self.property.set_value(value);
        Ok(())
    }

    /// Replaces the property value using the generic [`MultiValues`] setter.
    ///
    /// # Type Parameters
    ///
    /// * `S` - Input accepted by [`MultiValues`] setter traits.
    ///
    /// # Parameters
    ///
    /// * `values` - New value or values.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::PropertyIsFinal`] if the property has already
    /// been marked final, or a converted value error if setting fails.
    pub fn set<S>(&mut self, values: S) -> ConfigResult<()>
    where
        S: for<'b> MultiValuesSetArg<'b>,
        <S as MultiValuesSetArg<'static>>::Item: Clone,
        MultiValues: MultiValuesSetter<<S as MultiValuesSetArg<'static>>::Item>
            + MultiValuesSetterSlice<<S as MultiValuesSetArg<'static>>::Item>
            + MultiValuesSingleSetter<<S as MultiValuesSetArg<'static>>::Item>,
    {
        self.ensure_not_final()?;
        self.property.set(values).map_err(ConfigError::from)
    }

    /// Appends values using the generic [`MultiValues`] adder.
    ///
    /// # Type Parameters
    ///
    /// * `S` - Input accepted by [`MultiValues`] adder traits.
    ///
    /// # Parameters
    ///
    /// * `values` - Value or values to append.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::PropertyIsFinal`] if the property has already
    /// been marked final, or a converted value error if appending fails.
    pub fn add<S>(&mut self, values: S) -> ConfigResult<()>
    where
        S: for<'b> MultiValuesAddArg<'b, Item = <S as MultiValuesSetArg<'static>>::Item>
            + for<'b> MultiValuesSetArg<'b>,
        <S as MultiValuesSetArg<'static>>::Item: Clone,
        MultiValues: MultiValuesAdder<<S as MultiValuesSetArg<'static>>::Item>
            + MultiValuesMultiAdder<<S as MultiValuesSetArg<'static>>::Item>
            + MultiValuesSetter<<S as MultiValuesSetArg<'static>>::Item>
            + MultiValuesSetterSlice<<S as MultiValuesSetArg<'static>>::Item>
            + MultiValuesSingleSetter<<S as MultiValuesSetArg<'static>>::Item>,
    {
        self.ensure_not_final()?;
        self.property.add(values).map_err(ConfigError::from)
    }

    /// Clears the property value when the property is not final.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::PropertyIsFinal`] if the property has already
    /// been marked final.
    #[inline]
    pub fn clear(&mut self) -> ConfigResult<()> {
        self.ensure_not_final()?;
        self.property.clear();
        Ok(())
    }

    /// Ensures the guarded property has not been marked final.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the property is mutable.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::PropertyIsFinal`] if the property is final.
    #[inline]
    fn ensure_not_final(&self) -> ConfigResult<()> {
        if self.property.is_final() {
            return Err(ConfigError::PropertyIsFinal(
                self.property.name().to_string(),
            ));
        }
        Ok(())
    }
}

impl Deref for ConfigPropertyMut<'_> {
    type Target = Property;

    /// Dereferences to the guarded property for read-only access.
    #[inline]
    fn deref(&self) -> &Self::Target {
        self.property
    }
}
