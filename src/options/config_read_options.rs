/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/

use qubit_datatype::{
    BlankStringPolicy, BooleanConversionOptions, CollectionConversionOptions,
    DataConversionOptions, DurationConversionOptions, DurationUnit, EmptyItemPolicy,
    StringConversionOptions,
};
use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Runtime options that control how configuration values are read and parsed.
///
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ConfigReadOptions {
    /// Common scalar, collection, boolean, and duration conversion options.
    conversion: DataConversionOptions,
    /// Whether unresolved `${...}` placeholders may be read from process
    /// environment variables.
    env_variable_substitution_enabled: bool,
}

impl Serialize for ConfigReadOptions {
    /// Serializes all runtime read options.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        ConfigReadOptionsSerde::from(self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ConfigReadOptions {
    /// Deserializes runtime read options.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        ConfigReadOptionsSerde::deserialize(deserializer)?
            .try_into()
            .map_err(D::Error::custom)
    }
}

impl ConfigReadOptions {
    /// Creates options suitable for environment-variable style values.
    ///
    /// # Returns
    ///
    /// Options that trim strings, treat blank scalar strings as missing, accept
    /// common boolean aliases, and split scalar strings on commas while
    /// skipping empty collection items. Environment-variable substitution is
    /// still disabled; enable it explicitly with
    /// [`Self::with_env_variable_substitution_enabled`].
    #[must_use]
    pub fn env_friendly() -> Self {
        Self {
            conversion: DataConversionOptions::env_friendly(),
            env_variable_substitution_enabled: false,
        }
    }

    /// Gets the underlying data conversion options.
    ///
    /// # Returns
    ///
    /// Options used by the shared `qubit-datatype` conversion layer.
    #[inline]
    pub fn conversion_options(&self) -> &DataConversionOptions {
        &self.conversion
    }

    /// Returns whether `${...}` substitution may read process environment
    /// variables when a value is missing from config.
    ///
    /// # Returns
    ///
    /// `true` when environment fallback is enabled.
    #[inline]
    pub fn is_env_variable_substitution_enabled(&self) -> bool {
        self.env_variable_substitution_enabled
    }

    /// Returns a copy with environment-variable substitution enabled or
    /// disabled.
    ///
    /// # Parameters
    ///
    /// * `enabled` - Whether unresolved config placeholders may fall back to
    ///   process environment variables.
    ///
    /// # Returns
    ///
    /// Updated options.
    #[must_use]
    pub fn with_env_variable_substitution_enabled(mut self, enabled: bool) -> Self {
        self.env_variable_substitution_enabled = enabled;
        self
    }

    /// Returns a copy with a different blank string policy.
    ///
    /// # Parameters
    ///
    /// * `policy` - New blank string policy.
    ///
    /// # Returns
    ///
    /// Updated options.
    #[must_use]
    pub fn with_blank_string_policy(mut self, policy: BlankStringPolicy) -> Self {
        self.conversion = self.conversion.with_blank_string_policy(policy);
        self
    }

    /// Returns a copy with a different empty collection item policy.
    ///
    /// # Parameters
    ///
    /// * `policy` - New empty item policy.
    ///
    /// # Returns
    ///
    /// Updated options.
    #[must_use]
    pub fn with_empty_item_policy(mut self, policy: EmptyItemPolicy) -> Self {
        self.conversion = self.conversion.with_empty_item_policy(policy);
        self
    }

    /// Returns a copy with different string conversion options.
    ///
    /// # Parameters
    ///
    /// * `string` - New string conversion options.
    ///
    /// # Returns
    ///
    /// Updated options.
    #[must_use]
    pub fn with_string_options(mut self, string: StringConversionOptions) -> Self {
        self.conversion = self.conversion.with_string_options(string);
        self
    }

    /// Returns a copy with different boolean conversion options.
    ///
    /// # Parameters
    ///
    /// * `boolean` - New boolean conversion options.
    ///
    /// # Returns
    ///
    /// Updated options.
    #[must_use]
    pub fn with_boolean_options(mut self, boolean: BooleanConversionOptions) -> Self {
        self.conversion = self.conversion.with_boolean_options(boolean);
        self
    }

    /// Returns a copy with different collection conversion options.
    ///
    /// # Parameters
    ///
    /// * `collection` - New collection conversion options.
    ///
    /// # Returns
    ///
    /// Updated options.
    #[must_use]
    pub fn with_collection_options(mut self, collection: CollectionConversionOptions) -> Self {
        self.conversion = self.conversion.with_collection_options(collection);
        self
    }

    /// Returns a copy with different duration conversion options.
    ///
    /// # Parameters
    ///
    /// * `duration` - New duration conversion options.
    ///
    /// # Returns
    ///
    /// Updated options.
    #[must_use]
    pub fn with_duration_options(mut self, duration: DurationConversionOptions) -> Self {
        self.conversion = self.conversion.with_duration_options(duration);
        self
    }
}

impl AsRef<DataConversionOptions> for ConfigReadOptions {
    /// Borrows the underlying data conversion options.
    #[inline]
    fn as_ref(&self) -> &DataConversionOptions {
        &self.conversion
    }
}

impl From<DataConversionOptions> for ConfigReadOptions {
    /// Creates config read options from data conversion options.
    ///
    /// Environment-variable fallback for `${...}` substitution remains disabled.
    #[inline]
    fn from(conversion: DataConversionOptions) -> Self {
        Self {
            conversion,
            env_variable_substitution_enabled: false,
        }
    }
}

/// Serde representation of [`ConfigReadOptions`].
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ConfigReadOptionsSerde {
    /// Common scalar, collection, boolean, and duration conversion options.
    #[serde(default)]
    conversion: DataConversionOptionsSerde,
    /// Whether unresolved `${...}` placeholders may fall back to environment variables.
    #[serde(default)]
    env_variable_substitution_enabled: bool,
}

impl Default for ConfigReadOptionsSerde {
    /// Creates the serde representation of default read options.
    fn default() -> Self {
        Self::from(&ConfigReadOptions::default())
    }
}

impl From<&ConfigReadOptions> for ConfigReadOptionsSerde {
    /// Converts read options to their serde representation.
    fn from(options: &ConfigReadOptions) -> Self {
        Self {
            conversion: DataConversionOptionsSerde::from(&options.conversion),
            env_variable_substitution_enabled: options.env_variable_substitution_enabled,
        }
    }
}

impl TryFrom<ConfigReadOptionsSerde> for ConfigReadOptions {
    type Error = String;

    /// Converts the serde representation back to read options.
    fn try_from(value: ConfigReadOptionsSerde) -> Result<Self, Self::Error> {
        Ok(Self {
            conversion: value.conversion.try_into()?,
            env_variable_substitution_enabled: value.env_variable_substitution_enabled,
        })
    }
}

/// Serde representation of [`DataConversionOptions`].
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DataConversionOptionsSerde {
    /// String source conversion behavior.
    #[serde(default)]
    string: StringConversionOptionsSerde,
    /// Boolean string literal conversion behavior.
    #[serde(default)]
    boolean: BooleanConversionOptionsSerde,
    /// Scalar string collection conversion behavior.
    #[serde(default)]
    collection: CollectionConversionOptionsSerde,
    /// Duration conversion behavior.
    #[serde(default)]
    duration: DurationConversionOptionsSerde,
}

impl Default for DataConversionOptionsSerde {
    /// Creates the serde representation of default conversion options.
    fn default() -> Self {
        Self::from(&DataConversionOptions::default())
    }
}

impl From<&DataConversionOptions> for DataConversionOptionsSerde {
    /// Converts conversion options to their serde representation.
    fn from(options: &DataConversionOptions) -> Self {
        Self {
            string: StringConversionOptionsSerde::from(&options.string),
            boolean: BooleanConversionOptionsSerde::from(&options.boolean),
            collection: CollectionConversionOptionsSerde::from(&options.collection),
            duration: DurationConversionOptionsSerde::from(&options.duration),
        }
    }
}

impl TryFrom<DataConversionOptionsSerde> for DataConversionOptions {
    type Error = String;

    /// Converts the serde representation back to conversion options.
    fn try_from(value: DataConversionOptionsSerde) -> Result<Self, Self::Error> {
        Ok(Self {
            string: value.string.into(),
            boolean: value.boolean.try_into()?,
            collection: value.collection.into(),
            duration: value.duration.into(),
        })
    }
}

/// Serde representation of [`StringConversionOptions`].
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StringConversionOptionsSerde {
    /// Whether strings are trimmed before conversion.
    #[serde(default)]
    trim: bool,
    /// How blank strings are interpreted after optional trimming.
    #[serde(default)]
    blank_string_policy: BlankStringPolicySerde,
}

impl Default for StringConversionOptionsSerde {
    /// Creates the serde representation of default string conversion options.
    fn default() -> Self {
        Self::from(&StringConversionOptions::default())
    }
}

impl From<&StringConversionOptions> for StringConversionOptionsSerde {
    /// Converts string conversion options to their serde representation.
    fn from(options: &StringConversionOptions) -> Self {
        Self {
            trim: options.trim,
            blank_string_policy: options.blank_string_policy.into(),
        }
    }
}

impl From<StringConversionOptionsSerde> for StringConversionOptions {
    /// Converts the serde representation back to string conversion options.
    fn from(value: StringConversionOptionsSerde) -> Self {
        Self {
            trim: value.trim,
            blank_string_policy: value.blank_string_policy.into(),
        }
    }
}

/// Serde representation of [`BooleanConversionOptions`].
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BooleanConversionOptionsSerde {
    /// String literals accepted as `true`.
    #[serde(default = "default_true_literals")]
    true_literals: Vec<String>,
    /// String literals accepted as `false`.
    #[serde(default = "default_false_literals")]
    false_literals: Vec<String>,
    /// Whether literal matching is case-sensitive.
    #[serde(default)]
    case_sensitive: bool,
}

impl Default for BooleanConversionOptionsSerde {
    /// Creates the serde representation of default boolean conversion options.
    fn default() -> Self {
        Self::from(&BooleanConversionOptions::default())
    }
}

impl From<&BooleanConversionOptions> for BooleanConversionOptionsSerde {
    /// Converts boolean conversion options to their serde representation.
    fn from(options: &BooleanConversionOptions) -> Self {
        Self {
            true_literals: options.true_literals().to_vec(),
            false_literals: options.false_literals().to_vec(),
            case_sensitive: options.case_sensitive,
        }
    }
}

impl TryFrom<BooleanConversionOptionsSerde> for BooleanConversionOptions {
    type Error = String;

    /// Converts the serde representation back to boolean conversion options.
    fn try_from(value: BooleanConversionOptionsSerde) -> Result<Self, Self::Error> {
        let mut options = BooleanConversionOptions::strict();
        let strict = BooleanConversionOptions::strict();
        ensure_literal_prefix(
            &value.true_literals,
            strict.true_literals(),
            "true_literals",
        )?;
        ensure_literal_prefix(
            &value.false_literals,
            strict.false_literals(),
            "false_literals",
        )?;
        for literal in value
            .true_literals
            .iter()
            .skip(strict.true_literals().len())
        {
            options = options.with_true_literal(literal);
        }
        for literal in value
            .false_literals
            .iter()
            .skip(strict.false_literals().len())
        {
            options = options.with_false_literal(literal);
        }
        Ok(options.with_case_sensitive(value.case_sensitive))
    }
}

/// Serde representation of [`CollectionConversionOptions`].
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CollectionConversionOptionsSerde {
    /// Whether a scalar string can be split into collection items.
    #[serde(default)]
    split_scalar_strings: bool,
    /// Delimiters used to split scalar strings.
    #[serde(default = "default_delimiters")]
    delimiters: Vec<char>,
    /// Whether split items are trimmed before element conversion.
    #[serde(default)]
    trim_items: bool,
    /// How empty split items are interpreted.
    #[serde(default)]
    empty_item_policy: EmptyItemPolicySerde,
}

impl Default for CollectionConversionOptionsSerde {
    /// Creates the serde representation of default collection conversion options.
    fn default() -> Self {
        Self::from(&CollectionConversionOptions::default())
    }
}

impl From<&CollectionConversionOptions> for CollectionConversionOptionsSerde {
    /// Converts collection conversion options to their serde representation.
    fn from(options: &CollectionConversionOptions) -> Self {
        Self {
            split_scalar_strings: options.split_scalar_strings,
            delimiters: options.delimiters.clone(),
            trim_items: options.trim_items,
            empty_item_policy: options.empty_item_policy.into(),
        }
    }
}

impl From<CollectionConversionOptionsSerde> for CollectionConversionOptions {
    /// Converts the serde representation back to collection conversion options.
    fn from(value: CollectionConversionOptionsSerde) -> Self {
        Self {
            split_scalar_strings: value.split_scalar_strings,
            delimiters: value.delimiters,
            trim_items: value.trim_items,
            empty_item_policy: value.empty_item_policy.into(),
        }
    }
}

/// Serde representation of [`DurationConversionOptions`].
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DurationConversionOptionsSerde {
    /// Unit used for suffixless strings and integer conversions.
    #[serde(default)]
    unit: DurationUnitSerde,
    /// Whether formatted duration strings include the unit suffix.
    #[serde(default = "default_append_unit_suffix")]
    append_unit_suffix: bool,
}

impl Default for DurationConversionOptionsSerde {
    /// Creates the serde representation of default duration conversion options.
    fn default() -> Self {
        Self::from(&DurationConversionOptions::default())
    }
}

impl From<&DurationConversionOptions> for DurationConversionOptionsSerde {
    /// Converts duration conversion options to their serde representation.
    fn from(options: &DurationConversionOptions) -> Self {
        Self {
            unit: options.unit.into(),
            append_unit_suffix: options.append_unit_suffix,
        }
    }
}

impl From<DurationConversionOptionsSerde> for DurationConversionOptions {
    /// Converts the serde representation back to duration conversion options.
    fn from(value: DurationConversionOptionsSerde) -> Self {
        Self {
            unit: value.unit.into(),
            append_unit_suffix: value.append_unit_suffix,
        }
    }
}

/// Serde representation of [`BlankStringPolicy`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum BlankStringPolicySerde {
    /// Keep blank strings as real string values.
    Preserve,
    /// Treat blank strings as missing values.
    TreatAsMissing,
    /// Reject blank strings as invalid input.
    Reject,
}

impl Default for BlankStringPolicySerde {
    /// Creates the default blank string policy representation.
    fn default() -> Self {
        BlankStringPolicy::Preserve.into()
    }
}

impl From<BlankStringPolicy> for BlankStringPolicySerde {
    /// Converts a blank string policy to its serde representation.
    fn from(value: BlankStringPolicy) -> Self {
        match value {
            BlankStringPolicy::Preserve => Self::Preserve,
            BlankStringPolicy::TreatAsMissing => Self::TreatAsMissing,
            BlankStringPolicy::Reject => Self::Reject,
        }
    }
}

impl From<BlankStringPolicySerde> for BlankStringPolicy {
    /// Converts the serde representation back to a blank string policy.
    fn from(value: BlankStringPolicySerde) -> Self {
        match value {
            BlankStringPolicySerde::Preserve => Self::Preserve,
            BlankStringPolicySerde::TreatAsMissing => Self::TreatAsMissing,
            BlankStringPolicySerde::Reject => Self::Reject,
        }
    }
}

/// Serde representation of [`EmptyItemPolicy`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum EmptyItemPolicySerde {
    /// Keep empty items and pass them to the element converter.
    Keep,
    /// Drop empty items before element conversion.
    Skip,
    /// Reject empty items as invalid input.
    Reject,
}

impl Default for EmptyItemPolicySerde {
    /// Creates the default empty item policy representation.
    fn default() -> Self {
        EmptyItemPolicy::Keep.into()
    }
}

impl From<EmptyItemPolicy> for EmptyItemPolicySerde {
    /// Converts an empty item policy to its serde representation.
    fn from(value: EmptyItemPolicy) -> Self {
        match value {
            EmptyItemPolicy::Keep => Self::Keep,
            EmptyItemPolicy::Skip => Self::Skip,
            EmptyItemPolicy::Reject => Self::Reject,
        }
    }
}

impl From<EmptyItemPolicySerde> for EmptyItemPolicy {
    /// Converts the serde representation back to an empty item policy.
    fn from(value: EmptyItemPolicySerde) -> Self {
        match value {
            EmptyItemPolicySerde::Keep => Self::Keep,
            EmptyItemPolicySerde::Skip => Self::Skip,
            EmptyItemPolicySerde::Reject => Self::Reject,
        }
    }
}

/// Serde representation of [`DurationUnit`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum DurationUnitSerde {
    /// Nanoseconds.
    Nanoseconds,
    /// Microseconds.
    Microseconds,
    /// Milliseconds.
    Milliseconds,
    /// Seconds.
    Seconds,
    /// Minutes.
    Minutes,
    /// Hours.
    Hours,
    /// Days.
    Days,
}

impl Default for DurationUnitSerde {
    /// Creates the default duration unit representation.
    fn default() -> Self {
        DurationUnit::default().into()
    }
}

impl From<DurationUnit> for DurationUnitSerde {
    /// Converts a duration unit to its serde representation.
    fn from(value: DurationUnit) -> Self {
        match value {
            DurationUnit::Nanoseconds => Self::Nanoseconds,
            DurationUnit::Microseconds => Self::Microseconds,
            DurationUnit::Milliseconds => Self::Milliseconds,
            DurationUnit::Seconds => Self::Seconds,
            DurationUnit::Minutes => Self::Minutes,
            DurationUnit::Hours => Self::Hours,
            DurationUnit::Days => Self::Days,
        }
    }
}

impl From<DurationUnitSerde> for DurationUnit {
    /// Converts the serde representation back to a duration unit.
    fn from(value: DurationUnitSerde) -> Self {
        match value {
            DurationUnitSerde::Nanoseconds => Self::Nanoseconds,
            DurationUnitSerde::Microseconds => Self::Microseconds,
            DurationUnitSerde::Milliseconds => Self::Milliseconds,
            DurationUnitSerde::Seconds => Self::Seconds,
            DurationUnitSerde::Minutes => Self::Minutes,
            DurationUnitSerde::Hours => Self::Hours,
            DurationUnitSerde::Days => Self::Days,
        }
    }
}

/// Returns the default true literals.
fn default_true_literals() -> Vec<String> {
    BooleanConversionOptions::default().true_literals().to_vec()
}

/// Returns the default false literals.
fn default_false_literals() -> Vec<String> {
    BooleanConversionOptions::default()
        .false_literals()
        .to_vec()
}

/// Returns the default scalar string collection delimiters.
fn default_delimiters() -> Vec<char> {
    CollectionConversionOptions::default().delimiters
}

/// Returns whether formatted durations include unit suffixes by default.
fn default_append_unit_suffix() -> bool {
    DurationConversionOptions::default().append_unit_suffix
}

/// Ensures serialized boolean literals came from a supported public constructor.
fn ensure_literal_prefix(
    actual: &[String],
    expected: &[String],
    field: &str,
) -> Result<(), String> {
    if actual.len() < expected.len()
        || !actual
            .iter()
            .zip(expected.iter())
            .all(|(left, right)| left == right)
    {
        return Err(format!(
            "{field} must start with the default literals: {:?}",
            expected
        ));
    }
    Ok(())
}

/// Exercises serde default implementations that are otherwise only present for
/// forward-compatible schema defaults.
#[cfg(coverage)]
#[doc(hidden)]
pub fn coverage_touch_config_read_option_serde_defaults() {
    let _ = ConfigReadOptionsSerde::default();
    let _ = BooleanConversionOptionsSerde::default();
}
