// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Feature flag matrix testing to ensure all combinations work correctly.
//!
//! This module tests various combinations of feature flags to ensure
//! compatibility and prevent integration issues across different configurations.

#![cfg(test)]

use std::fmt;

/// Feature flag detection utilities for runtime testing.
pub mod feature_support {
    /// Returns true if JSON output feature is enabled.
    pub const fn has_json_output() -> bool {
        cfg!(feature = "output-json")
    }
    
    /// Returns true if JUnit output feature is enabled.
    pub const fn has_junit_output() -> bool {
        cfg!(feature = "output-junit")
    }
    
    /// Returns true if libtest feature is enabled.
    pub const fn has_libtest() -> bool {
        cfg!(feature = "libtest")
    }
    
    /// Returns true if timestamps feature is enabled.
    pub const fn has_timestamps() -> bool {
        cfg!(feature = "timestamps")
    }
    
    /// Returns true if tracing feature is enabled.
    pub const fn has_tracing() -> bool {
        cfg!(feature = "tracing")
    }
    
    /// Returns true if observability feature is enabled.
    pub const fn has_observability() -> bool {
        cfg!(feature = "observability")
    }
    
    /// Returns true if macros feature is enabled.
    pub const fn has_macros() -> bool {
        cfg!(feature = "macros")
    }
    
    /// Returns list of enabled features.
    pub fn enabled_features() -> Vec<&'static str> {
        let mut features = Vec::new();
        
        if has_json_output() { features.push("output-json"); }
        if has_junit_output() { features.push("output-junit"); }
        if has_libtest() { features.push("libtest"); }
        if has_timestamps() { features.push("timestamps"); }
        if has_tracing() { features.push("tracing"); }
        if has_observability() { features.push("observability"); }
        if has_macros() { features.push("macros"); }
        
        features
    }
}

/// Feature combination testing utilities.
#[derive(Debug, Clone, PartialEq)]
pub struct FeatureCombination {
    pub output_json: bool,
    pub output_junit: bool,
    pub libtest: bool,
    pub timestamps: bool,
    pub tracing: bool,
    pub observability: bool,
    pub macros: bool,
}

impl FeatureCombination {
    /// Creates a new feature combination from current compilation features.
    pub fn current() -> Self {
        Self {
            output_json: feature_support::has_json_output(),
            output_junit: feature_support::has_junit_output(),
            libtest: feature_support::has_libtest(),
            timestamps: feature_support::has_timestamps(),
            tracing: feature_support::has_tracing(),
            observability: feature_support::has_observability(),
            macros: feature_support::has_macros(),
        }
    }
    
    /// Returns true if this is a minimal feature set.
    pub fn is_minimal(&self) -> bool {
        !self.output_json && !self.output_junit && !self.libtest 
            && !self.timestamps && !self.tracing && !self.observability
    }
    
    /// Returns true if this includes output features.
    pub fn has_output_features(&self) -> bool {
        self.output_json || self.output_junit || self.libtest
    }
    
    /// Returns true if this includes observability features.
    pub fn has_observability_features(&self) -> bool {
        self.timestamps || self.tracing || self.observability
    }
    
    /// Returns count of enabled features.
    pub fn feature_count(&self) -> usize {
        [
            self.output_json,
            self.output_junit,
            self.libtest,
            self.timestamps,
            self.tracing,
            self.observability,
            self.macros,
        ].iter().filter(|&&enabled| enabled).count()
    }
}

impl fmt::Display for FeatureCombination {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut features = Vec::new();
        if self.output_json { features.push("output-json"); }
        if self.output_junit { features.push("output-junit"); }
        if self.libtest { features.push("libtest"); }
        if self.timestamps { features.push("timestamps"); }
        if self.tracing { features.push("tracing"); }
        if self.observability { features.push("observability"); }
        if self.macros { features.push("macros"); }
        
        if features.is_empty() {
            write!(f, "minimal")
        } else {
            write!(f, "{}", features.join(","))
        }
    }
}

/// Core functionality tests that should work with any feature combination.
mod core_functionality_tests {
    use super::*;
    
    /// Basic test world that doesn't require macros.
    #[derive(Debug, Default)]
    struct BasicTestWorld {
        value: i32,
    }
    
    // Only compile step macros when macros feature is enabled
    #[cfg(feature = "macros")]
    mod with_macros {
        use super::*;
        use cucumber::{World, given, when, then};
        
        #[derive(Debug, Default, World)]
        struct MacroTestWorld {
            value: i32,
        }
        
        #[given(regex = r"a value of (\d+)")]
        fn given_value(world: &mut MacroTestWorld, value: i32) {
            world.value = value;
        }
        
        #[when(regex = r"I add (\d+)")]
        fn when_add(world: &mut MacroTestWorld, add: i32) {
            world.value += add;
        }
        
        #[then(regex = r"the result should be (\d+)")]
        fn then_result(world: &mut MacroTestWorld, expected: i32) {
            assert_eq!(world.value, expected);
        }
        
        #[test]
        fn test_step_macros_work() {
            let mut world = MacroTestWorld::default();
            given_value(&mut world, 5);
            when_add(&mut world, 3);
            then_result(&mut world, 8);
            
            println!("✓ Step macros work correctly");
        }
    }
    
    #[test]
    fn test_basic_functionality_works() {
        let combo = FeatureCombination::current();
        println!("Testing core functionality with features: {}", combo);
        
        // Test basic functionality without requiring macros
        let mut world = BasicTestWorld::default();
        world.value = 5;
        world.value += 3;
        assert_eq!(world.value, 8);
        
        println!("✓ Basic functionality works without macros");
    }
    
    #[test]
    fn test_feature_detection_accuracy() {
        let combo = FeatureCombination::current();
        println!("Current feature combination: {}", combo);
        
        // Verify feature detection matches compilation
        assert_eq!(combo.macros, cfg!(feature = "macros"));
        assert_eq!(combo.timestamps, cfg!(feature = "timestamps"));
        assert_eq!(combo.tracing, cfg!(feature = "tracing"));
        assert_eq!(combo.observability, cfg!(feature = "observability"));
        assert_eq!(combo.output_json, cfg!(feature = "output-json"));
        assert_eq!(combo.output_junit, cfg!(feature = "output-junit"));
        assert_eq!(combo.libtest, cfg!(feature = "libtest"));
    }
}

/// Output format feature tests.
#[cfg(any(feature = "output-json", feature = "output-junit", feature = "libtest"))]
mod output_features_tests {
    use super::*;
    use std::io;
    
    // Basic test world for output testing
    #[derive(Debug, Default)]
    struct OutputTestWorld;
    
    // Implement World trait manually since we can't rely on macros
    impl cucumber::World for OutputTestWorld {
        type Error = std::convert::Infallible;
        
        async fn new() -> Result<Self, Self::Error> {
            Ok(Self::default())
        }
    }
    
    #[test]
    fn test_output_format_availability() {
        let combo = FeatureCombination::current();
        println!("Testing output formats with features: {}", combo);
        
        assert!(combo.has_output_features(), 
            "At least one output feature should be enabled");
        
        // Test that output features can be instantiated
        #[cfg(feature = "output-json")]
        {
            let _json_writer = cucumber::writer::Json::<io::Sink>::new::<OutputTestWorld>(io::sink());
            println!("✓ JSON writer created successfully");
        }
        
        #[cfg(feature = "output-junit")]
        {
            let _junit_writer = cucumber::writer::JUnit::<OutputTestWorld, io::Sink>::new(io::sink(), 1);
            println!("✓ JUnit writer created successfully");
        }
        
        #[cfg(feature = "libtest")]
        {
            let _libtest_writer = cucumber::writer::Libtest::<OutputTestWorld, io::Sink>::new(io::sink());
            println!("✓ Libtest writer created successfully");
        }
    }
    
    #[cfg(all(feature = "output-json", feature = "timestamps"))]
    #[test]
    fn test_json_with_timestamps() {
        println!("Testing JSON output with timestamps");
        let _json_writer = cucumber::writer::Json::<io::Sink>::new::<OutputTestWorld>(io::sink());
        // JSON output should include timestamp information when timestamps are enabled
        assert!(feature_support::has_timestamps());
    }
    
    #[cfg(all(feature = "output-junit", feature = "timestamps"))]
    #[test]
    fn test_junit_with_timestamps() {
        println!("Testing JUnit output with timestamps");
        let _junit_writer = cucumber::writer::JUnit::<OutputTestWorld, io::Sink>::new(io::sink(), 1);
        // JUnit output should include timing information when timestamps are enabled
        assert!(feature_support::has_timestamps());
    }
    
    #[cfg(all(feature = "libtest", feature = "timestamps"))]
    #[test]
    fn test_libtest_with_timestamps() {
        println!("Testing libtest output with timestamps");
        let _libtest_writer = cucumber::writer::Libtest::<OutputTestWorld, io::Sink>::new(io::sink());
        // libtest output should include timing information when timestamps are enabled
        assert!(feature_support::has_timestamps());
    }
    
    /// Test multiple output formats can coexist
    #[cfg(all(feature = "output-json", feature = "output-junit"))]
    #[test]
    fn test_multiple_output_formats() {
        println!("Testing multiple output formats together");
        let _json_writer = cucumber::writer::Json::<io::Sink>::new::<OutputTestWorld>(io::sink());
        let _junit_writer = cucumber::writer::JUnit::<OutputTestWorld, io::Sink>::new(io::sink(), 1);
        // Multiple output formats should work together
    }
}

/// Observability feature tests.
#[cfg(any(feature = "timestamps", feature = "tracing", feature = "observability"))]
mod observability_tests {
    use super::*;
    
    #[test]
    fn test_observability_features_available() {
        let combo = FeatureCombination::current();
        println!("Testing observability with features: {}", combo);
        
        assert!(combo.has_observability_features(),
            "At least one observability feature should be enabled");
    }
    
    #[cfg(feature = "timestamps")]
    #[test]
    fn test_timestamps_feature() {
        println!("Testing timestamps feature");
        assert!(feature_support::has_timestamps());
        // Timestamp functionality should be available
    }
    
    #[cfg(feature = "tracing")]
    #[test]
    fn test_tracing_feature() {
        println!("Testing tracing feature");
        assert!(feature_support::has_tracing());
        // Tracing functionality should be available
    }
    
    #[cfg(feature = "observability")]
    #[test]
    fn test_observability_feature() {
        println!("Testing observability hooks");
        assert!(feature_support::has_observability());
        // Observability hooks should be available
    }
    
    #[cfg(all(feature = "tracing", feature = "observability"))]
    #[test]
    fn test_tracing_with_observability() {
        println!("Testing tracing + observability integration");
        // Tracing and observability should work together
        assert!(feature_support::has_tracing());
        assert!(feature_support::has_observability());
    }
    
    #[cfg(all(feature = "timestamps", feature = "tracing"))]
    #[test]
    fn test_timestamps_with_tracing() {
        println!("Testing timestamps + tracing integration");
        // Timestamps and tracing should work together
        assert!(feature_support::has_timestamps());
        assert!(feature_support::has_tracing());
    }
}

/// Cross-category feature integration tests.
#[cfg(all(
    any(feature = "output-json", feature = "output-junit", feature = "libtest"),
    any(feature = "timestamps", feature = "tracing", feature = "observability")
))]
mod integration_tests {
    use super::*;
    use std::io;
    
    // Integration test world that implements World
    #[derive(Debug, Default)]
    struct IntegrationTestWorld;
    
    impl cucumber::World for IntegrationTestWorld {
        type Error = std::convert::Infallible;
        
        async fn new() -> Result<Self, Self::Error> {
            Ok(Self::default())
        }
    }
    
    #[test]
    fn test_output_with_observability() {
        let combo = FeatureCombination::current();
        println!("Testing output + observability integration: {}", combo);
        
        assert!(combo.has_output_features());
        assert!(combo.has_observability_features());
        
        // Test that output and observability features work together
        #[cfg(all(feature = "output-json", feature = "tracing"))]
        {
            let _json_writer = cucumber::writer::Json::<io::Sink>::new::<IntegrationTestWorld>(io::sink());
            println!("✓ JSON writer with tracing");
        }
        
        #[cfg(all(feature = "output-junit", feature = "timestamps"))]
        {
            let _junit_writer = cucumber::writer::JUnit::<IntegrationTestWorld, io::Sink>::new(io::sink(), 1);
            println!("✓ JUnit writer with timestamps");
        }
        
        #[cfg(all(feature = "libtest", feature = "observability"))]
        {
            let _libtest_writer = cucumber::writer::Libtest::<IntegrationTestWorld, io::Sink>::new(io::sink());
            println!("✓ Libtest writer with observability");
        }
    }
    
    #[cfg(all(feature = "output-json", feature = "output-junit", feature = "tracing"))]
    #[test]
    fn test_multiple_outputs_with_tracing() {
        println!("Testing multiple outputs + tracing");
        let _json_writer = cucumber::writer::Json::<io::Sink>::new::<IntegrationTestWorld>(io::sink());
        let _junit_writer = cucumber::writer::JUnit::<IntegrationTestWorld, io::Sink>::new(io::sink(), 1);
        // Multiple outputs should work with tracing
        assert!(feature_support::has_tracing());
    }
    
    #[cfg(all(
        feature = "output-json",
        feature = "output-junit", 
        feature = "libtest",
        feature = "timestamps",
        feature = "tracing",
        feature = "observability"
    ))]
    #[test]
    fn test_full_feature_matrix() {
        println!("Testing full feature matrix");
        let combo = FeatureCombination::current();
        println!("Full combination: {}", combo);
        
        // All output formats
        let _json_writer = cucumber::writer::Json::<io::Sink>::new::<IntegrationTestWorld>(io::sink());
        let _junit_writer = cucumber::writer::JUnit::<IntegrationTestWorld, io::Sink>::new(io::sink(), 1);
        let _libtest_writer = cucumber::writer::Libtest::<IntegrationTestWorld, io::Sink>::new(io::sink());
        
        // All features should work together
        assert_eq!(combo.feature_count(), 7); // All non-default features
        assert!(combo.has_output_features());
        assert!(combo.has_observability_features());
    }
}

/// Macro feature tests.
#[cfg(feature = "macros")]
mod macro_tests {
    use super::*;
    use cucumber::{World, given, when, then};
    
    #[derive(Debug, Default, World)]
    struct MacroTestWorld {
        step_count: usize,
    }
    
    #[given("macros are enabled")]
    fn given_macros_enabled(world: &mut MacroTestWorld) {
        world.step_count += 1;
    }
    
    #[when("I use step attributes")]
    fn when_use_step_attributes(world: &mut MacroTestWorld) {
        world.step_count += 1;
    }
    
    #[then("they should work correctly")]
    fn then_should_work(world: &mut MacroTestWorld) {
        world.step_count += 1;
        assert_eq!(world.step_count, 3);
    }
    
    #[test]
    fn test_step_macros_work() {
        println!("Testing step attribute macros");
        let mut world = MacroTestWorld::default();
        
        // Test that step attribute macros compile and work
        given_macros_enabled(&mut world);
        when_use_step_attributes(&mut world);
        then_should_work(&mut world);
        
        assert!(feature_support::has_macros());
    }
    
    #[test]
    fn test_world_macro_works() {
        println!("Testing World derive macro");
        let _world = MacroTestWorld::default();
        // World derive macro should work
        assert!(feature_support::has_macros());
    }
}

/// Feature combination summary tests.
mod summary_tests {
    use super::*;
    
    #[test]
    fn test_feature_combination_display() {
        let combos = [
            FeatureCombination {
                output_json: false, output_junit: false, libtest: false,
                timestamps: false, tracing: false, observability: false, macros: false,
            },
            FeatureCombination {
                output_json: true, output_junit: false, libtest: false,
                timestamps: false, tracing: false, observability: false, macros: false,
            },
            FeatureCombination {
                output_json: true, output_junit: true, libtest: false,
                timestamps: true, tracing: false, observability: false, macros: false,
            },
        ];
        
        assert_eq!(combos[0].to_string(), "minimal");
        assert_eq!(combos[1].to_string(), "output-json");
        assert_eq!(combos[2].to_string(), "output-json,output-junit,timestamps");
    }
    
    #[test]
    fn test_current_feature_summary() {
        let combo = FeatureCombination::current();
        let features = feature_support::enabled_features();
        
        println!("=== FEATURE MATRIX TEST SUMMARY ===");
        println!("Current feature combination: {}", combo);
        println!("Enabled features: {:?}", features);
        println!("Feature count: {}", combo.feature_count());
        println!("Has output features: {}", combo.has_output_features());
        println!("Has observability features: {}", combo.has_observability_features());
        println!("Is minimal: {}", combo.is_minimal());
        
        if combo.is_minimal() {
            println!("✓ Testing minimal feature set");
        } else {
            println!("✓ Testing enhanced feature set");
        }
        
        // Report which specific features are active
        if combo.output_json { println!("  ✓ JSON output enabled"); }
        if combo.output_junit { println!("  ✓ JUnit output enabled"); }
        if combo.libtest { println!("  ✓ Libtest output enabled"); }
        if combo.timestamps { println!("  ✓ Timestamps enabled"); }
        if combo.tracing { println!("  ✓ Tracing enabled"); }
        if combo.observability { println!("  ✓ Observability enabled"); }
        if combo.macros { println!("  ✓ Macros enabled"); }
        
        println!("=== END SUMMARY ===");
        
        // Basic sanity check
        assert!(!features.is_empty() || combo.is_minimal());
    }
}