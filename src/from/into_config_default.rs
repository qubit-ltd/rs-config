/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/

/// Converts a caller-provided fallback value into the typed configuration result.
///
/// This trait keeps typed reads focused on [`super::FromConfig`] while allowing
/// ergonomic default literals, such as `&str` for `String` and `&[&str]` for
/// `Vec<String>`.
pub trait IntoConfigDefault<T> {
    /// Converts this fallback value into `T`.
    ///
    /// # Returns
    ///
    /// The owned default value used when a configuration key is missing or
    /// empty.
    fn into_config_default(self) -> T;
}

impl<T> IntoConfigDefault<T> for T {
    #[inline]
    fn into_config_default(self) -> T {
        self
    }
}

impl IntoConfigDefault<String> for &str {
    #[inline]
    fn into_config_default(self) -> String {
        self.to_string()
    }
}

impl IntoConfigDefault<String> for &String {
    #[inline]
    fn into_config_default(self) -> String {
        self.clone()
    }
}

impl<T> IntoConfigDefault<Vec<T>> for &[T]
where
    T: Clone,
{
    #[inline]
    fn into_config_default(self) -> Vec<T> {
        self.to_vec()
    }
}

impl<T> IntoConfigDefault<Vec<T>> for &Vec<T>
where
    T: Clone,
{
    #[inline]
    fn into_config_default(self) -> Vec<T> {
        self.as_slice().to_vec()
    }
}

impl<T, const N: usize> IntoConfigDefault<Vec<T>> for [T; N] {
    #[inline]
    fn into_config_default(self) -> Vec<T> {
        Vec::from(self)
    }
}

impl<T, const N: usize> IntoConfigDefault<Vec<T>> for &[T; N]
where
    T: Clone,
{
    #[inline]
    fn into_config_default(self) -> Vec<T> {
        self.to_vec()
    }
}

impl IntoConfigDefault<Vec<String>> for &[&str] {
    #[inline]
    fn into_config_default(self) -> Vec<String> {
        self.iter().map(|value| value.to_string()).collect()
    }
}

impl IntoConfigDefault<Vec<String>> for &Vec<&str> {
    #[inline]
    fn into_config_default(self) -> Vec<String> {
        self.iter().map(|value| value.to_string()).collect()
    }
}

impl IntoConfigDefault<Vec<String>> for Vec<&str> {
    #[inline]
    fn into_config_default(self) -> Vec<String> {
        self.into_iter().map(|value| value.to_string()).collect()
    }
}

impl<const N: usize> IntoConfigDefault<Vec<String>> for [&str; N] {
    #[inline]
    fn into_config_default(self) -> Vec<String> {
        self.into_iter().map(|value| value.to_string()).collect()
    }
}

impl<const N: usize> IntoConfigDefault<Vec<String>> for &[&str; N] {
    #[inline]
    fn into_config_default(self) -> Vec<String> {
        self.iter().map(|value| value.to_string()).collect()
    }
}
