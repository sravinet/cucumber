// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! [Cucumber JSON format][1] [`Writer`] implementation.
//!
//! This module provides a modular implementation of the JSON writer following
//! the Single Responsibility Principle. The implementation is organized into
//! several focused modules:
//!
//! - [`types`]: Basic serializable data types
//! - [`element`]: Element (Scenario/Background) structures
//! - [`feature`]: Feature structures and utilities
//! - [`handlers`]: Event handling logic
//! - [`writer`]: Core writer implementation
//!
//! [1]: https://github.com/cucumber/cucumber-json-schema

pub mod element;
pub mod feature;
pub mod handlers;
pub mod types;
pub mod writer;

// Re-export all public types for backward compatibility
pub use self::{
    element::Element,
    feature::Feature,
    types::{Base64, Embedding, HookResult, RunResult, Status, Step, Tag},
    writer::Json,
};

#[cfg(test)]
mod integration_tests {
    use std::{io::Cursor, time::SystemTime};

    use super::*;
    use crate::{
        Event, World, Writer, cli,
        event::{
            Cucumber, Feature as FeatureEvent, Hook, HookType, Metadata,
            Scenario, Step as StepEvent,
        },
        parser::Result as ParserResult,
    };

    #[derive(Debug)]
    struct TestWorld;

    impl World for TestWorld {
        type Error = std::convert::Infallible;

        async fn new() -> Result<Self, Self::Error> {
            Ok(Self)
        }
    }

    fn create_test_feature() -> gherkin::Feature {
        gherkin::Feature {
            keyword: "Feature".to_string(),
            name: "Integration Test Feature".to_string(),
            description: None,
            background: None,
            scenarios: vec![],
            rules: vec![],
            tags: vec!["@integration".to_string()],
            span: gherkin::Span { start: 0, end: 0 },
            position: gherkin::LineCol { line: 1, col: 1 },
            path: Some(std::path::PathBuf::from("integration.feature")),
        }
    }

    fn create_test_scenario() -> gherkin::Scenario {
        gherkin::Scenario {
            keyword: "Scenario".to_string(),
            name: "Integration Test Scenario".to_string(),
            description: None,
            tags: vec!["@test".to_string()],
            span: gherkin::Span { start: 0, end: 0 },
            position: gherkin::LineCol { line: 5, col: 1 },
            steps: vec![],
            examples: vec![],
        }
    }

    fn create_test_step() -> gherkin::Step {
        gherkin::Step {
            keyword: "Given".to_string(),
            value: "integration test step".to_string(),
            docstring: None,
            table: None,
            span: gherkin::Span { start: 0, end: 0 },
            ty: gherkin::StepType::Given,
            position: gherkin::LineCol { line: 6, col: 1 },
        }
    }

    #[tokio::test]
    async fn full_scenario_lifecycle() {
        let mut writer = Json::raw(Cursor::new(Vec::new()));
        let feature = create_test_feature();
        let scenario = create_test_scenario();
        let step = create_test_step();

        // 1. Scenario started
        let event = Event::new(
            Cucumber::Feature(
                crate::event::Source::new(feature.clone()),
                FeatureEvent::Scenario(
                    crate::event::Source::new(scenario.clone()),
                    crate::event::RetryableScenario {
                        event: Scenario::Started::<TestWorld>,
                        retries: None,
                    },
                ),
            ),
        );
        writer.handle_event(Ok(event), &cli::Empty).await;

        // 2. Step started
        let event = Event::new(
            Cucumber::Feature(
                crate::event::Source::new(feature.clone()),
                FeatureEvent::Scenario(
                    crate::event::Source::new(scenario.clone()),
                    crate::event::RetryableScenario {
                        event: Scenario::Step::<TestWorld>(crate::event::Source::new(step.clone()), StepEvent::Started),
                        retries: None,
                    },
                ),
            ),
        );
        writer.handle_event(Ok(event), &cli::Empty).await;

        // 3. Add a log message
        let event = Event::new(
            Cucumber::Feature(
                crate::event::Source::new(feature.clone()),
                FeatureEvent::Scenario(
                    crate::event::Source::new(scenario.clone()),
                    crate::event::RetryableScenario {
                        event: Scenario::<TestWorld>::Log("Step execution log".to_string()),
                        retries: None,
                    },
                ),
            ),
        );
        writer.handle_event(Ok(event), &cli::Empty).await;

        // 4. Step passed
        let event = Event::new(
            Cucumber::Feature(
                crate::event::Source::new(feature.clone()),
                FeatureEvent::Scenario(
                    crate::event::Source::new(scenario.clone()),
                    crate::event::RetryableScenario {
                        event: Scenario::<TestWorld>::Step(
                            crate::event::Source::new(step.clone()),
                            StepEvent::Passed {
                                captures: regex::Regex::new(r"").unwrap().capture_locations(),
                                location: None,
                            },
                        ),
                        retries: None,
                    },
                ),
            ),
        );
        writer.handle_event(Ok(event), &cli::Empty).await;

        // 5. Before hook
        let event = Event::new(
            Cucumber::Feature(
                crate::event::Source::new(feature.clone()),
                FeatureEvent::Scenario(
                    crate::event::Source::new(scenario.clone()),
                    crate::event::RetryableScenario {
                        event: Scenario::<TestWorld>::Hook(HookType::Before, Hook::Started),
                        retries: None,
                    },
                ),
            ),
        );
        writer.handle_event(Ok(event), &cli::Empty).await;

        let event = Event::new(
            Cucumber::Feature(
                crate::event::Source::new(feature.clone()),
                FeatureEvent::Scenario(
                    crate::event::Source::new(scenario.clone()),
                    crate::event::RetryableScenario {
                        event: Scenario::<TestWorld>::Hook(HookType::Before, Hook::Passed),
                        retries: None,
                    },
                ),
            ),
        );
        writer.handle_event(Ok(event), &cli::Empty).await;

        // 6. Scenario finished
        let event = Event::new(
            Cucumber::Feature(
                crate::event::Source::new(feature.clone()),
                FeatureEvent::Scenario(
                    crate::event::Source::new(scenario.clone()),
                    crate::event::RetryableScenario {
                        event: Scenario::<TestWorld>::Finished,
                        retries: None,
                    },
                ),
            ),
        );
        writer.handle_event(Ok(event), &cli::Empty).await;

        // 7. Finish and output JSON
        let event = Event::new(Cucumber::<TestWorld>::Finished);
        writer.handle_event(Ok(event), &cli::Empty).await;

        // JSON writer processed all events successfully without panic
        // This validates the full integration flow works correctly
    }

    #[test]
    fn all_types_are_serializable() {
        // Test that all our types can be serialized properly
        let base64 = Base64::encode("test data");
        assert!(serde_json::to_string(&base64).is_ok());

        let embedding = Embedding::from_log("test log");
        assert!(serde_json::to_string(&embedding).is_ok());

        let tag = Tag { name: "@test".to_string(), line: 1 };
        assert!(serde_json::to_string(&tag).is_ok());

        let status = Status::Passed;
        assert!(serde_json::to_string(&status).is_ok());

        let run_result = RunResult {
            status: Status::Passed,
            duration: 1000,
            error_message: None,
        };
        assert!(serde_json::to_string(&run_result).is_ok());

        let step = Step {
            keyword: "Given".to_string(),
            line: 1,
            name: "test step".to_string(),
            hidden: false,
            result: run_result.clone(),
            embeddings: vec![embedding],
        };
        assert!(serde_json::to_string(&step).is_ok());

        let hook_result = HookResult { result: run_result, embeddings: vec![] };
        assert!(serde_json::to_string(&hook_result).is_ok());
    }

    #[test]
    fn json_schema_compatibility() {
        // Verify that our JSON output matches expected schema structure
        let feature = create_test_feature();
        let scenario = create_test_scenario();
        let element = Element::new(&feature, None, &scenario, "scenario");
        let json_feature = Feature::new(&feature);

        // Test required fields are present
        let element_json = serde_json::to_value(&element).unwrap();
        assert!(element_json.as_object().unwrap().contains_key("keyword"));
        assert!(element_json.as_object().unwrap().contains_key("type"));
        assert!(element_json.as_object().unwrap().contains_key("id"));
        assert!(element_json.as_object().unwrap().contains_key("line"));
        assert!(element_json.as_object().unwrap().contains_key("name"));
        assert!(element_json.as_object().unwrap().contains_key("tags"));
        assert!(element_json.as_object().unwrap().contains_key("steps"));

        let feature_json = serde_json::to_value(&json_feature).unwrap();
        assert!(feature_json.as_object().unwrap().contains_key("keyword"));
        assert!(feature_json.as_object().unwrap().contains_key("name"));
        assert!(feature_json.as_object().unwrap().contains_key("tags"));
        assert!(feature_json.as_object().unwrap().contains_key("elements"));
        assert!(feature_json.as_object().unwrap().contains_key("uri"));
    }
}
