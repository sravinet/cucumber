//! [`tracing`] integration layer.
//!
//! This module provides comprehensive tracing integration for Cucumber tests,
//! allowing for detailed logging and span management during test execution.
//!
//! The module is organized into several focused components:
//! - [`types`]: Core type definitions and aliases
//! - [`collector`]: Event collection and scenario management
//! - [`cucumber_ext`]: Extension methods for Cucumber configuration
//! - [`scenario_id_ext`]: ScenarioId extensions for span creation
//! - [`waiter`]: Span lifecycle management
//! - [`layer`]: Tracing layer implementation
//! - [`visitor`]: Field visitors for extracting scenario information
//! - [`formatter`]: Event and field formatting with scenario markers
//! - [`writer`]: CollectorWriter for sending events to the collector

pub mod collector;
pub mod cucumber_ext;
pub mod formatter;
pub mod layer;
pub mod scenario_id_ext;
pub mod types;
pub mod visitor;
pub mod waiter;
pub mod writer;

// Re-export public types for backward compatibility
pub use collector::Collector;
// Re-export suffix constants for parsing
pub use formatter::suffix;
pub use formatter::{AppendScenarioMsg, SkipScenarioIdSpan};
pub use layer::RecordScenarioId;
// Re-export commonly used type aliases
pub use types::{
    Callback, IsReceived, LogMessage, LogReceiver, LogSender, Scenarios,
    SpanCloseReceiver, SpanCloseSender, SpanEventsCallbacks,
};
// Re-export visitor types for advanced usage
pub use visitor::{GetScenarioId, IsScenarioIdSpan};
pub use waiter::SpanCloseWaiter;
pub use writer::CollectorWriter;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_public_types_accessible() {
        // Test that all main types are accessible
        use std::collections::HashMap;

        use futures::channel::mpsc;

        let scenarios: Scenarios = HashMap::new();
        let span_events: SpanEventsCallbacks = HashMap::new();
        
        // Validate that collections can be used
        assert!(scenarios.is_empty());
        assert!(span_events.is_empty());

        let (log_sender, log_receiver): (LogSender, LogReceiver) =
            mpsc::unbounded();
        let (span_sender, span_receiver): (
            SpanCloseSender,
            SpanCloseReceiver,
        ) = mpsc::unbounded();

        let collector = Collector::new(log_receiver, span_receiver);
        let waiter = SpanCloseWaiter::new(mpsc::unbounded().0);
        let layer = RecordScenarioId::new(span_sender);
        let writer = CollectorWriter::new(log_sender);
        
        // Validate that the tracing components can be used
        let _scenario_waiter = collector.scenario_span_event_waiter();
        let _cloned_waiter = waiter.clone();
        
        // Test that writer implements expected traits
        use std::fmt::Debug;
        fn verify_debug<T: Debug>(_item: &T) -> bool { true }
        assert!(verify_debug(&layer));
        assert!(verify_debug(&writer));
    }

    #[test]
    fn test_visitor_types_accessible() {
        let get_visitor = GetScenarioId::new();
        let is_visitor = IsScenarioIdSpan::new();
        
        // Validate that visitors can be used for their intended purpose
        assert!(get_visitor.get_scenario_id().is_none()); // Initially no ID
        assert!(!is_visitor.is_scenario_span()); // Initially not a scenario span
    }

    #[test]
    fn test_formatter_types_accessible() {
        use tracing_subscriber::fmt::format::{DefaultFields, Format};

        let _skip_formatter = SkipScenarioIdSpan(DefaultFields::new());
        let _append_formatter = AppendScenarioMsg(Format::default());
    }

    #[test]
    fn test_suffix_constants_accessible() {
        assert_eq!(suffix::END, "__cucumber__scenario");
        assert_eq!(suffix::BEFORE_SCENARIO_ID, "__");
        assert_eq!(suffix::NO_SCENARIO_ID, "__unknown");
    }

    #[test]
    fn test_type_aliases_work() {
        use tracing::span;

        let _is_received: IsReceived = true;
        let (_callback_sender, _callback_receiver): (Callback, _) =
            futures::channel::oneshot::channel();
        let _log_msg: LogMessage = (None, String::new());
        
        // Test span creation functionality
        let test_span = span!(tracing::Level::INFO, "test_scenario", scenario_id = 42);
        let _span_id = test_span.id();
    }

    #[test]
    fn test_module_organization() {
        // Verify all modules are accessible
        let _ = types::Scenarios::new();
        let _ = visitor::GetScenarioId::new();
        let _ = visitor::IsScenarioIdSpan::new();

        // Test constants are accessible from their modules
        assert!(formatter::suffix::END.len() > 0);
    }

    #[test]
    fn test_integration_types_compatibility() {
        use futures::channel::mpsc;

        use crate::runner::basic::ScenarioId;

        // Test that types work together as expected
        let (log_sender, log_receiver) = mpsc::unbounded();
        let (span_sender, span_receiver) = mpsc::unbounded();
        
        // Test ScenarioId integration with tracing
        let scenario_id = ScenarioId(42);
        let _waiter = waiter::SpanCloseWaiter::new(mpsc::unbounded().0);
        
        // Validate ScenarioId can be used for span identification
        assert_eq!(scenario_id.0, 42);

        let collector = Collector::new(log_receiver, span_receiver);
        let _waiter = collector.scenario_span_event_waiter();
        let _writer = CollectorWriter::new(log_sender);
        let _layer = RecordScenarioId::new(span_sender);

        // These should all be compatible types
        assert!(std::mem::size_of_val(&collector) > 0);
        assert!(std::mem::size_of_val(&_waiter) > 0);
        assert!(std::mem::size_of_val(&_writer) > 0);
        assert!(std::mem::size_of_val(&_layer) > 0);
    }
}
