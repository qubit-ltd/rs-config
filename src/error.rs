/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! # Configuration Error Types
//!
//! Defines all possible error types in the configuration system.
//!
//! # Author
//!
//! Haixing Hu

pub use crate::config_error::ConfigError;

/// Result type for configuration operations
///
/// Used for all operations in the configuration system that may return errors.
pub type ConfigResult<T> = Result<T, ConfigError>;
