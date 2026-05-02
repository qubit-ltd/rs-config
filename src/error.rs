/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! # Configuration Error Types
//!
//! Defines all possible error types in the configuration system.
//!

pub use crate::config_error::ConfigError;

/// Result type for configuration operations
///
/// Used for all operations in the configuration system that may return errors.
pub type ConfigResult<T> = Result<T, ConfigError>;
