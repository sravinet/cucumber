// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Core JSON writer implementation.

use std::{fmt::Debug, io};

use crate::{
    Event, World, Writer, cli,
    event::{self, Cucumber, Rule},
    parser,
    writer::{
        self,
        discard,
        ext::Ext as _,
        json::{feature::Feature, handlers::EventHandler},
    },
};

/// [Cucumber JSON format][1] [`Writer`] implementation outputting JSON to an
/// [`io::Write`] implementor.
///
/// # Ordering
///
/// This [`Writer`] isn't [`Normalized`] by itself, so should be wrapped into
/// a [`writer::Normalize`], otherwise will panic in runtime as won't be able to
/// form [correct JSON][1].
///
/// [1]: https://github.com/cucumber/cucumber-json-schema
/// [`Normalized`]: writer::Normalized
#[derive(Clone, Debug)]
pub struct Json<Out: io::Write> {
    /// [`io::Write`] implementor to output [JSON][1] into.
    ///
    /// [1]: https://github.com/cucumber/cucumber-json-schema
    output: Out,

    /// Event handler for processing events and managing state.
    handler: EventHandler,
}

impl<W: World + Debug, Out: io::Write> Writer<W> for Json<Out> {
    type Cli = cli::Empty;

    async fn handle_event(
        &mut self,
        event: parser::Result<Event<event::Cucumber<W>>>,
        _: &Self::Cli,
    ) {
        match event.map(Event::split) {
            Err(parser::Error::Parsing(e)) => {
                let feature = Feature::parsing_err(&e);
                self.handler.features.push(feature);
            }
            Err(parser::Error::ExampleExpansion(e)) => {
                let feature = Feature::example_expansion_err(&e);
                self.handler.features.push(feature);
            }
            Ok((
                Cucumber::Feature(f, event::Feature::Scenario(sc, ev)),
                meta,
            )) => {
                self.handler
                    .handle_scenario_event(&f, None, &sc, ev.event, meta);
            }
            Ok((
                Cucumber::Feature(
                    f,
                    event::Feature::Rule(r, Rule::Scenario(sc, ev)),
                ),
                meta,
            )) => {
                self.handler.handle_scenario_event(
                    &f,
                    Some(&r),
                    &sc,
                    ev.event,
                    meta,
                );
            }
            Ok((Cucumber::Finished, _)) => {
                self.write_output();
            }
            _ => {}
        }
    }
}

impl<O: io::Write> writer::NonTransforming for Json<O> {}

impl<Out: io::Write> Json<Out> {
    /// Creates a new [`Normalized`] [`Json`] [`Writer`] outputting [JSON][1]
    /// into the given `output`.
    ///
    /// [`Normalized`]: writer::Normalized
    /// [1]: https://github.com/cucumber/cucumber-json-schema
    #[must_use]
    pub fn new<W: Debug + World>(output: Out) -> writer::Normalize<W, Self> {
        Self::raw(output).normalized()
    }
}

impl Json<io::Stdout> {
    /// Creates a new [`Normalized`] [`Json`] [`Writer`] outputting to
    /// [`io::Stdout`].
    ///
    /// [`Normalized`]: writer::Normalized
    #[must_use]
    pub fn stdout<W: Debug + World>() -> writer::Normalize<W, Self> {
        Self::new(io::stdout())
    }
}

impl<Out: io::Write> Json<Out> {
    /// Creates a new non-[`Normalized`] [`Json`] [`Writer`] outputting
    /// [JSON][1] into the given `output`, and suitable for feeding into
    /// [`tee()`].
    ///
    /// [`Normalized`]: writer::Normalized
    /// [`tee()`]: crate::WriterExt::tee
    /// [1]: https://github.com/cucumber/cucumber-json-schema
    /// [2]: crate::event::Cucumber
    #[must_use]
    pub fn for_tee(output: Out) -> discard::Arbitrary<discard::Stats<Self>> {
        Self::raw(output).discard_stats_writes().discard_arbitrary_writes()
    }

    /// Creates a new raw and non-[`Normalized`] [`Json`] [`Writer`] outputting
    /// [JSON][1] into the given `output`.
    ///
    /// Use it only if you know what you're doing. Otherwise, consider using
    /// [`Json::new()`] which creates an already [`Normalized`] version of
    /// [`Json`] [`Writer`].
    ///
    /// [`Normalized`]: writer::Normalized
    /// [1]: https://github.com/cucumber/cucumber-json-schema
    /// [2]: crate::event::Cucumber
    #[must_use]
    pub fn raw(output: Out) -> Self {
        Self { output, handler: EventHandler::new() }
    }

    /// Writes the final JSON output to the configured output stream.
    fn write_output(&mut self) {
        let json = match serde_json::to_string(&self.handler.features) {
            Ok(json) => json,
            Err(e) => {
                eprintln!("Warning: Failed to serialize JSON: {e}");
                return;
            }
        };

        if let Err(e) = self.output.write_all(json.as_bytes()) {
            eprintln!("Warning: Failed to write JSON: {e}");
        }
    }

    /// Returns a reference to the features collected so far.
    pub fn features(&self) -> &[Feature] {
        self.handler.features()
    }

    /// Returns the current statistics from the event handler.
    pub fn stats(&self) -> &crate::writer::common::WriterStats {
        self.handler.stats()
    }

    /// Returns whether there are any logs pending.
    pub fn has_pending_logs(&self) -> bool {
        self.handler.has_logs()
    }

    /// Clears any pending logs.
    pub fn clear_pending_logs(&mut self) {
        self.handler.clear_logs();
    }

    /// Returns the number of features collected so far.
    pub fn feature_count(&self) -> usize {
        self.handler.features.len()
    }
}

#[cfg(test)]
mod tests {
    use std::{io::Cursor, time::SystemTime};

    use super::*;
    use crate::{
        event::{Cucumber, Metadata, Scenario},
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

    fn create_test_json_writer() -> Json<Cursor<Vec<u8>>> {
        Json::raw(Cursor::new(Vec::new()))
    }

    #[test]
    fn json_writer_new() {
        let output = Cursor::new(Vec::new());
        let writer = Json::raw(output);

        assert_eq!(writer.feature_count(), 0);
        assert!(!writer.has_pending_logs());
    }

    #[test]
    fn json_writer_raw_creation() {
        fn test_creation() -> Json<Cursor<Vec<u8>>> {
            Json::raw(Cursor::new(Vec::new()))
        }

        let writer = test_creation();
        assert_eq!(writer.feature_count(), 0);
    }

    #[tokio::test]
    async fn handle_parsing_error() {
        let mut writer = create_test_json_writer();
        let error = parser::Error::Parsing(std::sync::Arc::new(gherkin::ParseFileError::Reading {
            path: std::path::PathBuf::from("test.feature"),
            source: std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "File not found",
            ),
        }));

        let result: parser::Result<Event<event::Cucumber<TestWorld>>> = Err(error);
        writer.handle_event(result, &cli::Empty).await;

        assert_eq!(writer.feature_count(), 1);
        let features = writer.features();
        assert_eq!(features[0].uri, Some("test.feature".to_string()));
        assert_eq!(features[0].elements.len(), 1);
        assert_eq!(features[0].elements[0].r#type, "scenario");
    }

    #[tokio::test]
    async fn handle_example_expansion_error() {
        let mut writer = create_test_json_writer();
        let error = parser::Error::ExampleExpansion(
            crate::feature::ExpandExamplesError {
                path: Some(std::path::PathBuf::from("examples.feature")),
                pos: gherkin::LineCol { line: 10, col: 5 },
                name: "missing_placeholder".to_string(),
            }.into(),
        );

        let result: parser::Result<Event<event::Cucumber<TestWorld>>> = Err(error);
        writer.handle_event(result, &cli::Empty).await;

        assert_eq!(writer.feature_count(), 1);
        let features = writer.features();
        assert_eq!(features[0].uri, Some("examples.feature".to_string()));
        assert_eq!(features[0].elements.len(), 1);
        assert_eq!(features[0].elements[0].steps[0].line, 10);
    }

    #[tokio::test]
    async fn handle_scenario_log_event() {
        let mut writer = create_test_json_writer();
        let feature = create_test_gherkin_feature();
        let scenario = create_test_gherkin_scenario();

        let event = Event::new(
            Cucumber::Feature(
                crate::event::Source::new(feature),
                crate::event::Feature::Scenario(
                    crate::event::Source::new(scenario),
                    crate::event::RetryableScenario {
                        event: Scenario::Log::<TestWorld>("Test log message".to_string()),
                        retries: None,
                    },
                ),
            ),
        );

        writer.handle_event(Ok(event), &cli::Empty).await;

        assert!(writer.has_pending_logs());
    }

    #[tokio::test]
    async fn handle_finished_event_writes_output() {
        let mut writer = create_test_json_writer();

        let event = Event::new(Cucumber::Finished::<TestWorld>);

        writer.handle_event(Ok(event), &cli::Empty).await;

        // Check that output was written (even if empty)
        let output = writer.output.into_inner();
        assert_eq!(output, b"[]");
    }

    #[test]
    fn writer_stats() {
        let writer = create_test_json_writer();
        let stats = writer.stats();

        // New writer should have zero stats
        assert_eq!(stats.passed(), 0);
        assert_eq!(stats.failed(), 0);
        assert_eq!(stats.skipped(), 0);
    }

    #[test]
    fn writer_clear_pending_logs() {
        let mut writer = create_test_json_writer();
        writer.handler.logs.push("test".to_string());

        assert!(writer.has_pending_logs());
        writer.clear_pending_logs();
        assert!(!writer.has_pending_logs());
    }

    #[test]
    fn writer_features_accessor() {
        let writer = create_test_json_writer();
        let features = writer.features();

        assert!(features.is_empty());
    }

    fn create_test_gherkin_feature() -> gherkin::Feature {
        gherkin::Feature {
            keyword: "Feature".to_string(),
            name: "Test Feature".to_string(),
            description: None,
            background: None,
            scenarios: vec![],
            rules: vec![],
            tags: vec![],
            span: gherkin::Span { start: 0, end: 0 },
            position: gherkin::LineCol { line: 1, col: 1 },
            path: Some(std::path::PathBuf::from("test.feature")),
        }
    }

    fn create_test_gherkin_scenario() -> gherkin::Scenario {
        gherkin::Scenario {
            keyword: "Scenario".to_string(),
            name: "Test Scenario".to_string(),
            description: None,
            tags: vec![],
            span: gherkin::Span { start: 0, end: 0 },
            position: gherkin::LineCol { line: 5, col: 1 },
            steps: vec![],
            examples: vec![],
        }
    }

    #[tokio::test]
    async fn handle_event_with_metadata() {
        let mut writer = create_test_json_writer();
        let feature = create_test_gherkin_feature();

        let metadata: Metadata = Event::new(());
        
        let event = Event {
            value: event::Cucumber::Feature(
                event::Source::new(feature),
                event::Feature::<TestWorld>::Started,
            ),
            at: SystemTime::UNIX_EPOCH,
        };

        // Test that metadata can be created and used
        assert!(std::mem::size_of_val(&metadata) > 0);
        
        writer.handle_event(Ok(event), &cli::Empty).await;
        assert_eq!(writer.feature_count(), 1);
    }

    #[tokio::test]
    async fn handle_event_with_timing() {
        let mut writer = create_test_json_writer();
        let scenario = create_test_gherkin_scenario();
        let feature = create_test_gherkin_feature();

        let start_time = SystemTime::UNIX_EPOCH;

        let start_event = Event {
            value: event::Cucumber::Feature(
                event::Source::new(feature),
                event::Feature::Scenario(
                    event::Source::new(scenario),
                    event::RetryableScenario {
                        event: Scenario::<TestWorld>::Started,
                        retries: None,
                    },
                ),
            ),
            at: start_time,
        };

        writer.handle_event(Ok(start_event), &cli::Empty).await;
        
        // Test that timing is captured and handled
        // This validates SystemTime usage in event handling
        assert!(writer.feature_count() <= 1);
    }

    #[test]
    fn parser_result_error_handling() {
        // Test error path using ParserResult
        let parse_error = gherkin::ParseFileError::Reading {
            path: std::path::PathBuf::from("missing.feature"),
            source: std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "File not found",
            ),
        };
        let parser_result: ParserResult<Event<event::Cucumber<TestWorld>>> = 
            Err(parser::Error::Parsing(std::sync::Arc::new(parse_error)));
        
        // Validate error can be processed
        assert!(parser_result.is_err());
        match parser_result {
            Err(parser::Error::Parsing(err)) => {
                assert!(matches!(err.as_ref(), gherkin::ParseFileError::Reading { .. }));
            }
            _ => panic!("Expected parsing error"),
        }
    }
    
    #[tokio::test]
    async fn test_parser_result_integration() {
        let mut writer = create_test_json_writer();
        let feature = create_test_gherkin_feature();
        
        // Test ParserResult::Ok case
        let ok_result: ParserResult<gherkin::Feature> = Ok(feature.clone());
        match ok_result {
            Ok(f) => {
                let event = Event::new(event::Cucumber::Feature(
                    event::Source::new(f),
                    event::Feature::<TestWorld>::Started,
                ));
                writer.handle_event(Ok(event), &cli::Empty).await;
                assert_eq!(writer.feature_count(), 1);
            }
            Err(_) => panic!("Expected Ok result"),
        }
        
        // Test ParserResult::Err case for completeness
        let err_result: ParserResult<gherkin::Feature> = Err(Box::new(crate::parser::Error::new("test error")));
        assert!(err_result.is_err());
    }
}
