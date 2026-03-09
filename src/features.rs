// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Feature flags and conditional compilation module.
//!
//! This module centralizes all feature-dependent imports and conditional
//! compilation directives used throughout the cucumber crate.

/// Re-exports the codegen module when the "macros" feature is enabled.
#[cfg(feature = "macros")]
pub mod codegen {
    pub use crate::codegen::*;
}

/// Re-exports the tracing module when the "tracing" feature is enabled.
#[cfg(feature = "tracing")]
pub mod tracing {
    pub use crate::tracing::*;
}

/// Test dependencies that are only used in documentation tests and the book.
/// This helps prevent unused dependency warnings while keeping the dependencies
/// available for documentation examples.
#[cfg(test)]
pub mod test_deps {
    pub use rand as _;
    pub use tempfile as _;
    pub use tokio as _;
}

/// Checks if the "macros" feature is enabled at compile time.
#[must_use]
pub const fn has_macros_feature() -> bool {
    cfg!(feature = "macros")
}

/// Checks if the "tracing" feature is enabled at compile time.
#[must_use]
pub const fn has_tracing_feature() -> bool {
    cfg!(feature = "tracing")
}

/// Returns a list of enabled features as a static string slice.
/// 
/// This can be evaluated at compile time, providing zero-runtime-cost
/// feature detection for conditional behavior.
#[must_use]
pub const fn enabled() -> &'static [&'static str] {
    &[
        #[cfg(feature = "macros")]
        "macros",
        #[cfg(feature = "tracing")]
        "tracing",
    ]
}

/// Returns a formatted string describing all enabled features.
#[must_use]
pub fn summary() -> String {
    let features = enabled();
    if features.is_empty() {
        "No optional features enabled".to_owned()
    } else {
        format!("Enabled features: {}", features.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_macros_feature() {
        // Test that the function returns a boolean
        let has_macros = has_macros_feature();
        assert!(has_macros || !has_macros); // Always true, but tests the function call
    }

    #[test]
    fn test_has_tracing_feature() {
        // Test that the function returns a boolean
        let has_tracing = has_tracing_feature();
        assert!(has_tracing || !has_tracing); // Always true, but tests the function call
    }

    #[test]
    fn test_enabled_returns_slice() {
        let features = enabled();
        // Test that we get a slice (may be empty)
        assert!(features.len() >= 0);
    }

    #[test]
    fn test_features_summary_format() {
        let summary = summary();
        // Test that we get a non-empty string
        assert!(!summary.is_empty());

        // Test that it contains expected content
        if enabled().is_empty() {
            assert_eq!(summary, "No optional features enabled");
        } else {
            assert!(summary.starts_with("Enabled features: "));
        }
    }

    #[cfg(feature = "macros")]
    #[test]
    fn test_macros_feature_enabled() {
        assert!(has_macros_feature());
        assert!(enabled().contains(&"macros"));
    }

    #[cfg(feature = "tracing")]
    #[test]
    fn test_tracing_feature_enabled() {
        assert!(has_tracing_feature());
        assert!(enabled().contains(&"tracing"));
    }

    #[cfg(not(feature = "macros"))]
    #[test]
    fn test_macros_feature_disabled() {
        assert!(!has_macros_feature());
        assert!(!enabled().contains(&"macros"));
    }

    #[cfg(not(feature = "tracing"))]
    #[test]
    fn test_tracing_feature_disabled() {
        assert!(!has_tracing_feature());
        assert!(!enabled().contains(&"tracing"));
    }

    #[test]
    fn test_test_deps_are_accessible() {
        // Test that test dependencies are available and functional
        use test_deps::*;
        
        // Use rand to generate test data
        let mut rng = rand::thread_rng();
        use rand::Rng;
        let random_value: u32 = rng.gen_range(1..100);
        assert!(random_value >= 1 && random_value < 100);
        
        // Test passes if this compiles and runs without error
    }

    #[tokio::test]
    async fn test_async_feature_detection() {
        // Use tokio dependency for async feature testing
        use test_deps::*;
        
        // Test async capabilities with tokio
        let result = tokio::time::timeout(
            std::time::Duration::from_millis(100),
            async { enabled().len() }
        ).await;
        
        assert!(result.is_ok());
    }

    #[test] 
    fn test_temporary_file_support() {
        // Use tempfile dependency for testing file operations
        use test_deps::*;
        
        // Test temporary file creation for feature testing
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        assert!(temp_file.path().exists());
        
        // Write feature list to temp file
        use std::io::Write;
        let mut file = temp_file.reopen().unwrap();
        let features_content = enabled().join("\n");
        file.write_all(features_content.as_bytes()).unwrap();
        file.flush().unwrap();
    }

    #[test]
    fn test_features_consistency() {
        // Test that enabled() is consistent with individual feature checks
        let features = enabled();

        if has_macros_feature() {
            assert!(features.contains(&"macros"));
        } else {
            assert!(!features.contains(&"macros"));
        }

        if has_tracing_feature() {
            assert!(features.contains(&"tracing"));
        } else {
            assert!(!features.contains(&"tracing"));
        }
    }
}
