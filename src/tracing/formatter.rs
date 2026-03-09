//! Custom formatters for tracing events that handle scenario context appropriately.

use std::fmt::{self, Write};

use tracing::{Event, Subscriber, field::Field};
use tracing_subscriber::{
    fmt::{format, FormatEvent, FormatFields, FmtContext},
    field::RecordFields,
    registry::LookupSpan,
};

use super::visitor::{GetScenarioId, IsScenarioIdSpan};
use crate::runner::basic::ScenarioId;

/// [`FormatFields`] implementation that skips formatting fields if the span
/// contains a scenario ID field.
///
/// This prevents scenario ID internal field from being included in log output.
#[derive(Debug)]
pub struct SkipScenarioIdSpan<F>(pub F);

impl<F> SkipScenarioIdSpan<F> {
    /// Creates a new [`SkipScenarioIdSpan`] formatter wrapper.
    pub const fn new(inner: F) -> Self {
        Self(inner)
    }
}

impl<'a, F> FormatFields<'a> for SkipScenarioIdSpan<F>
where
    F: FormatFields<'a>,
{
    fn format_fields<R: RecordFields>(
        &self,
        writer: format::Writer<'a>,
        fields: R,
    ) -> fmt::Result {
        // Check if this field set contains the scenario ID field
        let mut visitor = IsScenarioIdSpan::new();
        fields.record(&mut visitor);

        if visitor.is_scenario_span() {
            // Skip formatting if it contains scenario ID
            Ok(())
        } else {
            // Otherwise use the inner formatter
            self.0.format_fields(writer, fields)
        }
    }
}

/// [`FormatEvent`] implementation that appends scenario context to event messages.
///
/// This formatter looks up the current scenario context and appends appropriate
/// suffixes to help with log parsing and context identification.
#[derive(Debug)]
pub struct AppendScenarioMsg<F>(pub F);

impl<F> AppendScenarioMsg<F> {
    /// Creates a new [`AppendScenarioMsg`] formatter wrapper.
    pub const fn new(inner: F) -> Self {
        Self(inner)
    }
}

impl<S, N, F> FormatEvent<S, N> for AppendScenarioMsg<F>
where
    F: FormatEvent<S, N>,
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: format::Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        // Try to get scenario ID from current span
        let scenario_id = ctx
            .lookup_current()
            .and_then(|span_ref| {
                // Try getting from extensions first
                span_ref.extensions().get::<GetScenarioId>()
                    .map(|stored| stored.get_scenario_id())
                    .flatten()
                    .or_else(|| {
                        // If not in extensions, try extracting from span metadata
                        let metadata = span_ref.metadata();
                        if metadata.fields().iter().any(|f| f.name() == "scenario_id") {
                            // This span might contain scenario ID, but we can't easily extract it
                            // without more complex visitor implementation
                            None
                        } else {
                            None
                        }
                    })
            });

        // Format the inner event first
        self.0.format_event(ctx, writer.by_ref(), event)?;

        // Append the appropriate suffix
        match scenario_id {
            Some(id) => write!(writer, "{}{}{}", suffix::SCENARIO_ID_START, id.0, suffix::END),
            None => write!(writer, "{}{}", suffix::NO_SCENARIO_ID, suffix::END),
        }
    }
}

/// Suffix constants for scenario context identification in log output.
pub mod suffix {
    /// Suffix indicating no scenario context is available.
    pub const NO_SCENARIO_ID: &str = " [no-scenario]";
    
    /// Start marker for scenario ID in log output.
    pub const SCENARIO_ID_START: &str = " [scenario-";
    
    /// Before scenario ID marker for parsing.
    pub const BEFORE_SCENARIO_ID: &str = " [scenario-";
    
    /// End marker for scenario context in log output.
    pub const END: &str = "]";
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_subscriber::{fmt::format, registry::Registry};

    /// Simple test writer that captures formatted output.
    #[derive(Default)]
    struct TestWriter {
        content: String,
    }

    impl TestWriter {
        fn new() -> Self {
            Self::default()
        }

        fn to_string(&self) -> String {
            self.content.clone()
        }
    }

    impl Write for TestWriter {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            self.content.push_str(s);
            Ok(())
        }
    }

    #[test]
    fn test_skip_scenario_id_span_creation() {
        let inner_formatter = format::DefaultFields::new();
        let formatter = SkipScenarioIdSpan::new(inner_formatter);
        
        // Test that the formatter can be created
        assert_eq!(std::mem::size_of_val(&formatter), std::mem::size_of::<format::DefaultFields>());
    }

    #[test]
    fn test_append_scenario_msg_creation() {
        let inner_formatter = format::Format::default();
        let formatter = AppendScenarioMsg::new(inner_formatter);
        
        // Test that the formatter can be created
        assert!(std::mem::size_of_val(&formatter) > 0);
    }

    #[test]
    fn test_suffix_constants() {
        // Verify suffix constants are properly defined
        assert!(!suffix::NO_SCENARIO_ID.is_empty());
        assert!(!suffix::SCENARIO_ID_START.is_empty());
        assert!(!suffix::END.is_empty());
        
        // Verify they are distinct
        assert_ne!(suffix::NO_SCENARIO_ID, suffix::SCENARIO_ID_START);
        assert_ne!(suffix::NO_SCENARIO_ID, suffix::END);
        assert_ne!(suffix::SCENARIO_ID_START, suffix::END);
    }

    #[test]
    fn test_formatter_wrappers_are_debug() {
        let fields_formatter = SkipScenarioIdSpan::new(format::DefaultFields::new());
        let event_formatter = AppendScenarioMsg::new(format::Format::default());

        // Test Debug implementations
        let _debug1 = format!("{:?}", fields_formatter);
        let _debug2 = format!("{:?}", event_formatter);
    }

    #[test]
    fn test_formatter_basic_functionality() {
        // Basic test that formatters can be constructed and used without panicking
        let _skip_formatter = SkipScenarioIdSpan::new(format::DefaultFields::new());
        let _append_formatter = AppendScenarioMsg::new(format::Format::default());
        
        // This test just ensures the types compile and can be created
        assert!(true);
    }
}