// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::cell::OnceCell;

use crate::ConfigResult;

/// Lazily stores one scalar's successful interpolation result.
pub(crate) struct PreparedScalar<'a> {
    raw: &'a str,
    expanded: OnceCell<String>,
}

impl<'a> PreparedScalar<'a> {
    pub(crate) fn new(raw: &'a str) -> Self {
        Self {
            raw,
            expanded: OnceCell::new(),
        }
    }

    pub(crate) fn text(
        &self,
        interpolate: bool,
        substitute: &dyn Fn(&str) -> ConfigResult<String>,
    ) -> ConfigResult<&str> {
        if !interpolate {
            return Ok(self.raw);
        }
        if let Some(value) = self.expanded.get() {
            return Ok(value);
        }
        let value = substitute(self.raw)?;
        let _ = self.expanded.set(value);
        Ok(self.expanded.get().expect("the expanded scalar was just set"))
    }

    pub(crate) fn is_raw(&self, value: &str) -> bool {
        self.raw == value
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::PreparedScalar;
    use crate::ConfigError;
    use crate::ConfigResult;

    #[test]
    fn successful_scalar_preparation_runs_once() {
        let scalar = PreparedScalar::new("${value}");
        let calls = Cell::new(0);
        let substitute = |_: &str| -> ConfigResult<String> {
            calls.set(calls.get() + 1);
            Ok("prepared".to_owned())
        };

        assert_eq!(scalar.text(true, &substitute).unwrap(), "prepared");
        assert_eq!(scalar.text(true, &substitute).unwrap(), "prepared");
        assert_eq!(calls.get(), 1);
    }

    #[test]
    fn failed_scalar_preparation_is_not_cached() {
        let scalar = PreparedScalar::new("${value}");
        let calls = Cell::new(0);
        let substitute = |_: &str| -> ConfigResult<String> {
            calls.set(calls.get() + 1);
            if calls.get() == 1 {
                Err(ConfigError::SubstitutionError {
                    path: "key".to_owned(),
                    message: "unavailable".to_owned(),
                })
            } else {
                Ok("prepared".to_owned())
            }
        };

        assert!(scalar.text(true, &substitute).is_err());
        assert_eq!(scalar.text(true, &substitute).unwrap(), "prepared");
        assert_eq!(scalar.text(true, &substitute).unwrap(), "prepared");
        assert_eq!(calls.get(), 2);
    }
}
