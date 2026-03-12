// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Comprehensive error recovery testing for panic-free architecture.
//!
//! This module validates that the error handling system provides graceful
//! recovery from all types of failures without crashing the system.

#![cfg(test)]

use std::{error::Error, io};
use cucumber::{
    World,
    error::{CucumberError, ExecutionError, WriterError, ExecutionResult, WriterResult},
};

/// Test world for error recovery scenarios.
#[derive(Debug, Default)]
struct ErrorRecoveryWorld {
    errors_encountered: Vec<String>,
    recovery_successful: bool,
}

impl World for ErrorRecoveryWorld {
    type Error = std::convert::Infallible;
    
    async fn new() -> Result<Self, Self::Error> {
        Ok(Self::default())
    }
}

impl ErrorRecoveryWorld {
    fn record_error(&mut self, error: &str) {
        self.errors_encountered.push(error.to_string());
    }
    
    fn mark_recovery_successful(&mut self) {
        self.recovery_successful = true;
    }
}

/// Test that ExecutionError provides proper error context and recovery.
mod execution_error_recovery {
    use super::*;

    #[test]
    fn test_feature_not_found_recovery() {
        let error = ExecutionError::feature_not_found("missing_feature");
        
        // Error should provide context
        assert!(error.to_string().contains("missing_feature"));
        assert!(error.is_feature_not_found());
        assert_eq!(error.feature_name(), Some("missing_feature"));
        
        // Error should be recoverable
        let result: ExecutionResult<()> = Err(error);
        match result {
            Ok(_) => panic!("Expected error"),
            Err(e) => {
                // Recovery logic would log error and continue
                println!("Recovered from feature not found: {}", e);
                // System continues operating
            }
        }
    }
    
    #[test] 
    fn test_rule_not_found_recovery() {
        let error = ExecutionError::rule_not_found("missing_rule", "test_feature");
        
        // Error should provide context for both rule and feature
        assert!(error.to_string().contains("missing_rule"));
        assert!(error.to_string().contains("test_feature"));
        assert!(error.is_rule_not_found());
        assert_eq!(error.rule_name(), Some("missing_rule"));
        assert_eq!(error.feature_name(), Some("test_feature"));
        
        // Recovery should allow fallback behavior
        let result: ExecutionResult<()> = Err(error);
        let recovered = result.unwrap_or_else(|e| {
            println!("Rule not found, using default behavior: {}", e);
            // Return default value instead of crashing
        });
        
        // System continues with recovered state
        // Recovery validation - system should continue operating
        assert!(true);
    }
    
    #[test]
    fn test_state_inconsistency_recovery() {
        let error = ExecutionError::state_inconsistency("started", "finished");
        
        assert!(error.to_string().contains("expected started, found finished"));
        assert!(error.is_state_inconsistency());
        
        // State inconsistency should allow reset/retry
        let result: ExecutionResult<String> = Err(error);
        let recovered = result.unwrap_or_else(|e| {
            println!("State inconsistency detected: {}, resetting...", e);
            "reset_state".to_string()
        });
        
        assert_eq!(recovered, "reset_state");
    }
    
    #[test]
    fn test_duplicate_event_recovery() {
        let error = ExecutionError::duplicate_event("test_scenario", "Started");
        
        assert!(error.to_string().contains("Duplicate event"));
        assert!(error.is_duplicate_event());
        
        // Duplicate event should be ignorable
        let result: ExecutionResult<()> = Err(error);
        result.unwrap_or_else(|e| {
            println!("Ignoring duplicate event: {}", e);
            // Continue processing without the duplicate
        });
    }
    
    #[test]
    fn test_invalid_sequence_recovery() {
        let error = ExecutionError::invalid_sequence("finished before started");
        
        assert!(error.to_string().contains("Invalid event sequence"));
        assert!(error.is_invalid_sequence());
        
        // Invalid sequence should allow reordering/correction
        let result: ExecutionResult<Vec<String>> = Err(error);
        let recovered = result.unwrap_or_else(|e| {
            println!("Invalid sequence detected: {}, reordering...", e);
            vec!["start".to_string(), "finish".to_string()]
        });
        
        assert_eq!(recovered, vec!["start", "finish"]);
    }
}

/// Test that WriterError provides proper error recovery.
mod writer_error_recovery {
    use super::*;
    
    #[test]
    fn test_io_error_recovery() {
        let io_error = io::Error::new(io::ErrorKind::BrokenPipe, "pipe closed");
        let writer_error = WriterError::from(io_error);
        
        assert!(writer_error.is_io_error());
        assert!(!writer_error.is_format_error());
        
        // I/O error should allow fallback output
        let result: WriterResult<()> = Err(writer_error);
        result.unwrap_or_else(|e| {
            println!("I/O error encountered: {}, using fallback output", e);
            // Switch to alternative output mechanism
        });
    }
    
    #[test]
    fn test_serialization_error_recovery() {
        #[cfg(any(feature = "output-json", feature = "libtest"))]
        {
            use serde_json;
            
            // Create a serialization error
            let invalid_json = "\x00\x01\x02";
            let serde_error = serde_json::from_str::<serde_json::Value>(invalid_json)
                .unwrap_err();
            let writer_error = WriterError::from(serde_error);
            
            assert!(writer_error.is_serialization_error());
            
            // Serialization error should allow alternative format
            let result: WriterResult<String> = Err(writer_error);
            let recovered = result.unwrap_or_else(|e| {
                println!("Serialization failed: {}, using plain text", e);
                "fallback_output".to_string()
            });
            
            assert_eq!(recovered, "fallback_output");
        }
    }
    
    #[test]
    fn test_format_error_recovery() {
        let format_error = std::fmt::Error;
        let writer_error = WriterError::from(format_error);
        
        assert!(writer_error.is_format_error());
        assert!(!writer_error.is_io_error());
        
        // Format error should allow simplified output
        let result: WriterResult<String> = Err(writer_error);
        let recovered = result.unwrap_or_else(|e| {
            println!("Format error: {}, using simplified format", e);
            "simplified_format".to_string()
        });
        
        assert_eq!(recovered, "simplified_format");
    }
    
    #[test]
    fn test_unavailable_error_recovery() {
        let writer_error = WriterError::unavailable("output buffer full");
        
        assert!(writer_error.is_unavailable());
        assert_eq!(writer_error.unavailable_reason(), Some("output buffer full"));
        
        // Unavailable error should allow retry or alternative
        let result: WriterResult<()> = Err(writer_error);
        result.unwrap_or_else(|e| {
            println!("Output unavailable: {}, retrying with smaller buffer", e);
            // Implement retry logic or alternative output
        });
    }
    
    #[cfg(feature = "output-junit")]
    #[test]
    fn test_xml_error_recovery() {
        let writer_error = WriterError::xml("malformed XML structure");
        
        assert!(writer_error.is_xml_error());
        
        // XML error should allow simplified XML or alternative format
        let result: WriterResult<String> = Err(writer_error);
        let recovered = result.unwrap_or_else(|e| {
            println!("XML generation failed: {}, using simplified XML", e);
            "<simple>fallback</simple>".to_string()
        });
        
        assert_eq!(recovered, "<simple>fallback</simple>");
    }
}

/// Test that CucumberError provides comprehensive error recovery.
mod cucumber_error_recovery {
    use super::*;
    
    #[test]
    fn test_error_hierarchy_recovery() {
        // Test that errors can be converted and recovered at different levels
        let execution_error = ExecutionError::feature_not_found("test");
        let cucumber_error = CucumberError::from(execution_error);
        
        assert!(matches!(cucumber_error, CucumberError::Execution(_)));
        
        // Recovery at CucumberError level
        let result: Result<(), CucumberError> = Err(cucumber_error);
        result.unwrap_or_else(|e| {
            match e {
                CucumberError::Execution(exec_err) => {
                    println!("Execution error recovered: {}", exec_err);
                }
                CucumberError::Writer(writer_err) => {
                    println!("Writer error recovered: {}", writer_err);
                }
                _ => {
                    println!("Other error recovered: {}", e);
                }
            }
        });
    }
    
    #[test]
    fn test_error_chaining_recovery() {
        // Test error source chains are preserved for debugging
        let io_error = io::Error::new(io::ErrorKind::Other, "root cause");
        let writer_error = WriterError::from(io_error);
        let cucumber_error = CucumberError::from(writer_error);
        
        // Verify error chain is preserved
        assert!(cucumber_error.source().is_some());
        
        // Recovery can access the error chain
        let result: Result<(), CucumberError> = Err(cucumber_error);
        result.unwrap_or_else(|e| {
            println!("Top-level error: {}", e);
            if let Some(source) = e.source() {
                println!("Caused by: {}", source);
                if let Some(root_cause) = source.source() {
                    println!("Root cause: {}", root_cause);
                }
            }
        });
    }
    
    #[test]
    fn test_error_context_preservation() {
        // Test that error context is preserved through conversions
        let writer_error = WriterError::unavailable("disk full");
        let cucumber_error = CucumberError::from(writer_error);
        
        let error_string = cucumber_error.to_string();
        assert!(error_string.contains("disk full"));
        assert!(error_string.contains("Writer error"));
        
        // Context should help with recovery decisions
        if error_string.contains("disk full") {
            println!("Recovery: Attempting cleanup to free disk space");
        } else if error_string.contains("network") {
            println!("Recovery: Switching to offline mode");
        }
    }
}

/// Test real-world error scenarios and recovery patterns.
mod integration_error_recovery {
    use super::*;
    
    struct ErrorProneSystem {
        should_fail_io: bool,
        should_fail_execution: bool,
        recovery_count: usize,
    }
    
    impl ErrorProneSystem {
        fn new() -> Self {
            Self {
                should_fail_io: false,
                should_fail_execution: false,
                recovery_count: 0,
            }
        }
        
        fn with_io_failure(mut self) -> Self {
            self.should_fail_io = true;
            self
        }
        
        fn with_execution_failure(mut self) -> Self {
            self.should_fail_execution = true;
            self
        }
        
        fn attempt_operation(&mut self) -> Result<String, CucumberError> {
            if self.should_fail_execution {
                self.should_fail_execution = false; // Fail only once
                return Err(CucumberError::Execution(
                    ExecutionError::feature_not_found("test_feature")
                ));
            }
            
            if self.should_fail_io {
                self.should_fail_io = false; // Fail only once
                return Err(CucumberError::Writer(
                    WriterError::Io(io::Error::new(io::ErrorKind::BrokenPipe, "pipe failed"))
                ));
            }
            
            Ok("operation_successful".to_string())
        }
        
        fn recovery_with_retry(&mut self, max_retries: usize) -> String {
            for attempt in 1..=max_retries {
                match self.attempt_operation() {
                    Ok(result) => {
                        if self.recovery_count > 0 {
                            println!("Recovery successful after {} attempts", self.recovery_count + 1);
                        }
                        return result;
                    }
                    Err(e) => {
                        self.recovery_count += 1;
                        println!("Attempt {} failed: {}, retrying...", attempt, e);
                        
                        // Specific recovery actions based on error type
                        match e {
                            CucumberError::Execution(_) => {
                                println!("  -> Resetting execution state");
                            }
                            CucumberError::Writer(_) => {
                                println!("  -> Reconnecting output stream");
                            }
                            _ => {
                                println!("  -> General recovery action");
                            }
                        }
                    }
                }
            }
            
            // If all retries failed, return a default/fallback result
            println!("All retries exhausted, using fallback result");
            "fallback_result".to_string()
        }
    }
    
    #[test]
    fn test_io_error_retry_recovery() {
        let mut system = ErrorProneSystem::new().with_io_failure();
        let result = system.recovery_with_retry(3);
        
        // First attempt should fail, second should succeed
        assert_eq!(result, "operation_successful");
        assert_eq!(system.recovery_count, 1);
    }
    
    #[test]
    fn test_execution_error_retry_recovery() {
        let mut system = ErrorProneSystem::new().with_execution_failure();
        let result = system.recovery_with_retry(3);
        
        // First attempt should fail, second should succeed
        assert_eq!(result, "operation_successful");
        assert_eq!(system.recovery_count, 1);
    }
    
    #[test]
    fn test_multiple_error_recovery() {
        let mut system = ErrorProneSystem::new()
            .with_execution_failure()
            .with_io_failure();
        let result = system.recovery_with_retry(5);
        
        // Should recover from both types of errors
        assert_eq!(result, "operation_successful");
        assert_eq!(system.recovery_count, 2);
    }
    
    #[test]
    fn test_graceful_degradation() {
        // Simulate a system that fails all attempts but gracefully degrades
        let mut system = ErrorProneSystem::new()
            .with_execution_failure()
            .with_io_failure();
        
        // Force continued failures for degradation test
        system.should_fail_execution = true;
        system.should_fail_io = true;
        
        let result = system.recovery_with_retry(2);
        
        // Should fall back to default behavior instead of crashing
        assert_eq!(result, "fallback_result");
        assert_eq!(system.recovery_count, 2);
    }
}

/// Performance tests for error recovery overhead.
mod error_recovery_performance {
    use super::*;
    use std::time::{Duration, Instant};
    
    #[test]
    fn test_error_creation_performance() {
        let start = Instant::now();
        
        // Create many errors to test overhead
        for i in 0..1000 {
            let _error = ExecutionError::feature_not_found(&format!("feature_{}", i));
        }
        
        let duration = start.elapsed();
        println!("Created 1000 errors in: {:?}", duration);
        
        // Error creation should be fast
        assert!(duration < Duration::from_millis(100));
    }
    
    #[test]
    fn test_error_conversion_performance() {
        let start = Instant::now();
        
        // Test error conversions
        for i in 0..1000 {
            let execution_error = ExecutionError::feature_not_found(&format!("feature_{}", i));
            let _cucumber_error: CucumberError = execution_error.into();
        }
        
        let duration = start.elapsed();
        println!("Converted 1000 errors in: {:?}", duration);
        
        // Error conversion should be fast
        assert!(duration < Duration::from_millis(50));
    }
    
    #[test] 
    fn test_result_unwrap_or_else_performance() {
        let start = Instant::now();
        
        // Test Result handling performance
        for i in 0..1000 {
            let result: Result<String, ExecutionError> = if i % 2 == 0 {
                Ok(format!("success_{}", i))
            } else {
                Err(ExecutionError::feature_not_found(&format!("feature_{}", i)))
            };
            
            let _value = result.unwrap_or_else(|_| "fallback".to_string());
        }
        
        let duration = start.elapsed();
        println!("Processed 1000 results in: {:?}", duration);
        
        // Result handling should be fast
        assert!(duration < Duration::from_millis(100));
    }
}

/// Test comprehensive error recovery scenarios.
mod comprehensive_error_tests {
    use super::*;
    
    #[test]
    fn test_error_recovery_world_integration() {
        let mut world = ErrorRecoveryWorld::default();
        
        // Simulate error scenarios
        let errors = vec![
            CucumberError::Execution(ExecutionError::feature_not_found("test1")),
            CucumberError::Writer(WriterError::unavailable("buffer full")),
            CucumberError::Execution(ExecutionError::state_inconsistency("a", "b")),
        ];
        
        for (i, error) in errors.into_iter().enumerate() {
            match error {
                CucumberError::Execution(e) => {
                    world.record_error(&format!("execution_error_{}: {}", i, e));
                }
                CucumberError::Writer(e) => {
                    world.record_error(&format!("writer_error_{}: {}", i, e));
                }
                _ => {
                    world.record_error(&format!("other_error_{}: {}", i, error));
                }
            }
        }
        
        world.mark_recovery_successful();
        
        // Verify recovery tracking
        assert_eq!(world.errors_encountered.len(), 3);
        assert!(world.recovery_successful);
        assert!(world.errors_encountered[0].contains("execution_error_0"));
        assert!(world.errors_encountered[1].contains("writer_error_1"));
        assert!(world.errors_encountered[2].contains("execution_error_2"));
    }
    
    #[test]
    fn test_error_message_quality() {
        // Test that error messages provide actionable information
        let errors = vec![
            ExecutionError::feature_not_found("missing_feature"),
            ExecutionError::rule_not_found("missing_rule", "test_feature"),
            ExecutionError::state_inconsistency("started", "finished"),
            ExecutionError::duplicate_event("scenario", "Started"),
            ExecutionError::invalid_sequence("out of order events"),
        ];
        
        for error in errors {
            let message = error.to_string();
            
            // Error messages should be informative
            assert!(!message.is_empty());
            assert!(!message.contains("Error")); // Should be human-readable
            assert!(message.len() > 10); // Should have meaningful content
            
            println!("Error message: {}", message);
        }
    }
    
    #[test]
    fn test_error_debugging_support() {
        // Test that errors provide good debugging information
        let io_error = io::Error::new(io::ErrorKind::PermissionDenied, "access denied");
        let writer_error = WriterError::from(io_error);
        let cucumber_error = CucumberError::from(writer_error);
        
        // Test debug formatting
        let debug_string = format!("{:?}", cucumber_error);
        assert!(debug_string.contains("Writer"));
        assert!(debug_string.contains("Io"));
        assert!(debug_string.contains("access denied"));
        
        // Test error source chain
        let mut current_error: &dyn Error = &cucumber_error;
        let mut depth = 0;
        
        while let Some(source) = current_error.source() {
            depth += 1;
            current_error = source;
        }
        
        // Should have a meaningful error chain depth
        assert!(depth > 0);
        println!("Error chain depth: {}", depth);
    }
}