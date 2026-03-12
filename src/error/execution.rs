// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Execution-specific error types and utilities.
//!
//! This module defines errors that can occur during test execution,
//! including feature/rule/scenario state management and event processing.

use derive_more::with_trait::{Display, Error};

/// Execution-specific errors.
#[derive(Debug, Display, Error)]
pub enum ExecutionError {
    /// Feature not found in execution context.
    #[display("Feature '{feature_name}' not found in execution context")]
    FeatureNotFound {
        /// Name of the feature that was not found.
        #[error(not(source))]
        feature_name: String,
    },

    /// Rule not found in feature context.
    #[display("Rule '{rule_name}' not found in feature '{feature_name}'")]
    RuleNotFound {
        /// Name of the rule that was not found.
        #[error(not(source))]
        rule_name: String,
        /// Name of the feature containing the rule.
        #[error(not(source))]
        feature_name: String,
    },

    /// Scenario state inconsistency detected.
    #[display("Scenario state inconsistency: expected {expected}, found {actual}")]
    StateInconsistency {
        /// Expected state.
        #[error(not(source))]
        expected: String,
        /// Actual state found.
        #[error(not(source))]
        actual: String,
    },

    /// Duplicate event for scenario.
    #[display("Duplicate event for scenario '{scenario}': {event_type}")]
    DuplicateEvent {
        /// Scenario identifier.
        #[error(not(source))]
        scenario: String,
        /// Type of the duplicate event.
        #[error(not(source))]
        event_type: String,
    },

    /// Invalid event sequence detected.
    #[display("Invalid event sequence: {description}")]
    InvalidSequence {
        /// Description of the invalid sequence.
        #[error(not(source))]
        description: String,
    },
}

/// Result type alias for execution operations.
pub type ExecutionResult<T> = std::result::Result<T, ExecutionError>;

impl ExecutionError {
    /// Creates a new feature not found error.
    #[must_use]
    pub fn feature_not_found(feature_name: impl Into<String>) -> Self {
        Self::FeatureNotFound {
            feature_name: feature_name.into(),
        }
    }

    /// Creates a new rule not found error.
    #[must_use]
    pub fn rule_not_found(
        rule_name: impl Into<String>,
        feature_name: impl Into<String>,
    ) -> Self {
        Self::RuleNotFound {
            rule_name: rule_name.into(),
            feature_name: feature_name.into(),
        }
    }

    /// Creates a new state inconsistency error.
    #[must_use]
    pub fn state_inconsistency(
        expected: impl Into<String>,
        actual: impl Into<String>,
    ) -> Self {
        Self::StateInconsistency {
            expected: expected.into(),
            actual: actual.into(),
        }
    }

    /// Creates a new duplicate event error.
    #[must_use]
    pub fn duplicate_event(
        scenario: impl Into<String>,
        event_type: impl Into<String>,
    ) -> Self {
        Self::DuplicateEvent {
            scenario: scenario.into(),
            event_type: event_type.into(),
        }
    }

    /// Creates a new invalid sequence error.
    #[must_use]
    pub fn invalid_sequence(description: impl Into<String>) -> Self {
        Self::InvalidSequence {
            description: description.into(),
        }
    }

    /// Returns true if this is a feature not found error.
    #[must_use]
    pub fn is_feature_not_found(&self) -> bool {
        matches!(self, Self::FeatureNotFound { .. })
    }

    /// Returns true if this is a rule not found error.
    #[must_use]
    pub fn is_rule_not_found(&self) -> bool {
        matches!(self, Self::RuleNotFound { .. })
    }

    /// Returns true if this is a state inconsistency error.
    #[must_use]
    pub fn is_state_inconsistency(&self) -> bool {
        matches!(self, Self::StateInconsistency { .. })
    }

    /// Returns true if this is a duplicate event error.
    #[must_use]
    pub fn is_duplicate_event(&self) -> bool {
        matches!(self, Self::DuplicateEvent { .. })
    }

    /// Returns true if this is an invalid sequence error.
    #[must_use]
    pub fn is_invalid_sequence(&self) -> bool {
        matches!(self, Self::InvalidSequence { .. })
    }

    /// Returns the feature name if applicable.
    #[must_use]
    pub fn feature_name(&self) -> Option<&str> {
        match self {
            Self::FeatureNotFound { feature_name } => Some(feature_name),
            Self::RuleNotFound { feature_name, .. } => Some(feature_name),
            _ => None,
        }
    }

    /// Returns the rule name if applicable.
    #[must_use]
    pub fn rule_name(&self) -> Option<&str> {
        match self {
            Self::RuleNotFound { rule_name, .. } => Some(rule_name),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_error_constructors() {
        let feature_err = ExecutionError::feature_not_found("test_feature");
        assert!(feature_err.is_feature_not_found());
        assert_eq!(feature_err.feature_name(), Some("test_feature"));
        assert!(
            feature_err
                .to_string()
                .contains("Feature 'test_feature' not found")
        );

        let rule_err = ExecutionError::rule_not_found("test_rule", "test_feature");
        assert!(rule_err.is_rule_not_found());
        assert_eq!(rule_err.rule_name(), Some("test_rule"));
        assert_eq!(rule_err.feature_name(), Some("test_feature"));
        assert!(
            rule_err
                .to_string()
                .contains("Rule 'test_rule' not found in feature 'test_feature'")
        );

        let state_err = ExecutionError::state_inconsistency("started", "finished");
        assert!(state_err.is_state_inconsistency());
        assert!(
            state_err
                .to_string()
                .contains("expected started, found finished")
        );

        let duplicate_err =
            ExecutionError::duplicate_event("test_scenario", "Started");
        assert!(duplicate_err.is_duplicate_event());
        assert!(
            duplicate_err
                .to_string()
                .contains("Duplicate event for scenario 'test_scenario': Started")
        );

        let sequence_err =
            ExecutionError::invalid_sequence("finished before started");
        assert!(sequence_err.is_invalid_sequence());
        assert!(
            sequence_err
                .to_string()
                .contains("Invalid event sequence: finished before started")
        );
    }

    #[test]
    fn test_execution_error_type_checks() {
        let feature_err = ExecutionError::feature_not_found("test");
        assert!(feature_err.is_feature_not_found());
        assert!(!feature_err.is_rule_not_found());
        assert!(!feature_err.is_state_inconsistency());
        assert!(!feature_err.is_duplicate_event());
        assert!(!feature_err.is_invalid_sequence());

        let rule_err = ExecutionError::rule_not_found("rule", "feature");
        assert!(!rule_err.is_feature_not_found());
        assert!(rule_err.is_rule_not_found());
        assert!(!rule_err.is_state_inconsistency());
        assert!(!rule_err.is_duplicate_event());
        assert!(!rule_err.is_invalid_sequence());
    }

    #[test]
    fn test_execution_error_name_extraction() {
        let feature_err = ExecutionError::feature_not_found("my_feature");
        assert_eq!(feature_err.feature_name(), Some("my_feature"));
        assert_eq!(feature_err.rule_name(), None);

        let rule_err = ExecutionError::rule_not_found("my_rule", "my_feature");
        assert_eq!(rule_err.feature_name(), Some("my_feature"));
        assert_eq!(rule_err.rule_name(), Some("my_rule"));

        let state_err = ExecutionError::state_inconsistency("a", "b");
        assert_eq!(state_err.feature_name(), None);
        assert_eq!(state_err.rule_name(), None);
    }

    #[test]
    fn test_execution_result_type() {
        let ok_result: ExecutionResult<String> = Ok("success".to_string());
        assert!(ok_result.is_ok());
        assert_eq!(ok_result.unwrap(), "success");

        let err_result: ExecutionResult<String> =
            Err(ExecutionError::feature_not_found("test"));
        assert!(err_result.is_err());
        assert!(err_result.unwrap_err().is_feature_not_found());
    }

    #[test]
    fn test_execution_error_display() {
        let variants = [
            ExecutionError::feature_not_found("test_feature"),
            ExecutionError::rule_not_found("test_rule", "test_feature"),
            ExecutionError::state_inconsistency("expected", "actual"),
            ExecutionError::duplicate_event("scenario", "event"),
            ExecutionError::invalid_sequence("description"),
        ];

        for error in variants {
            // Ensure all variants have meaningful display messages
            let display = error.to_string();
            assert!(!display.is_empty());
            assert!(!display.contains("ExecutionError"));
        }
    }
}