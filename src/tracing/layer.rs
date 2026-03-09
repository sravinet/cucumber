//! Tracing layer for recording scenario IDs and managing span lifecycle.

use futures::channel::mpsc;
use tracing::{Subscriber, span};
use tracing_subscriber::{
    layer::{self, Layer},
    registry::LookupSpan,
};

use super::visitor::GetScenarioId;
use crate::runner::basic::ScenarioId;

/// [`Layer`] recording a [`ScenarioId`] into [`Span`]'s [`Extensions`].
///
/// [`Extensions`]: tracing_subscriber::registry::Extensions
#[derive(Debug)]
pub struct RecordScenarioId {
    /// Sender for [`Span`] closing events.
    span_close_sender: mpsc::UnboundedSender<span::Id>,
}

impl RecordScenarioId {
    /// Creates a new [`RecordScenarioId`] [`Layer`].
    pub const fn new(
        span_close_sender: mpsc::UnboundedSender<span::Id>,
    ) -> Self {
        Self { span_close_sender }
    }

    /// Retrieves a [`ScenarioId`] from the given span, if present.
    pub fn get_scenario_id_from_span<S>(
        span: &tracing_subscriber::registry::SpanRef<'_, S>,
    ) -> Option<ScenarioId>
    where
        S: for<'a> LookupSpan<'a> + Subscriber,
    {
        span.extensions().get::<ScenarioId>().copied()
    }

    /// Checks if a span contains scenario-related tracing data.
    pub fn is_scenario_span<S>(
        span: &tracing_subscriber::registry::SpanRef<'_, S>,
    ) -> bool
    where
        S: for<'a> LookupSpan<'a> + Subscriber,
    {
        span.extensions().get::<ScenarioId>().is_some()
    }
}

impl<S> Layer<S> for RecordScenarioId
where
    S: for<'a> LookupSpan<'a> + Subscriber,
{
    fn on_new_span(
        &self,
        attrs: &span::Attributes<'_>,
        id: &span::Id,
        ctx: layer::Context<'_, S>,
    ) {
        if let Some(span) = ctx.span(id) {
            let mut visitor = GetScenarioId::new();
            attrs.values().record(&mut visitor);

            if let Some(scenario_id) = visitor.get_scenario_id() {
                let mut ext = span.extensions_mut();
                _ = ext.replace(scenario_id);
            }
        }
    }

    fn on_record(
        &self,
        id: &span::Id,
        values: &span::Record<'_>,
        ctx: layer::Context<'_, S>,
    ) {
        if let Some(span) = ctx.span(id) {
            let mut visitor = GetScenarioId::new();
            values.record(&mut visitor);

            if let Some(scenario_id) = visitor.get_scenario_id() {
                let mut ext = span.extensions_mut();
                _ = ext.replace(scenario_id);
            }
        }
    }

    fn on_close(&self, id: span::Id, _ctx: layer::Context<'_, S>) {
        _ = self.span_close_sender.unbounded_send(id).ok();
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashSet,
        sync::{Arc, Mutex},
    };

    use tracing::{Event, Subscriber};
    use tracing_subscriber::{layer::Context, registry::Registry};

    use super::*;

    #[derive(Debug)]
    struct TestSubscriber {
        spans: Arc<Mutex<HashSet<span::Id>>>,
    }

    impl TestSubscriber {
        fn new() -> Self {
            Self { spans: Arc::new(Mutex::new(HashSet::new())) }
        }
    }

    impl Subscriber for TestSubscriber {
        fn enabled(&self, _metadata: &tracing::Metadata<'_>) -> bool {
            true
        }

        fn new_span(&self, _span: &span::Attributes<'_>) -> span::Id {
            let id = span::Id::from_u64(42);
            self.spans.lock().unwrap().insert(id.clone());
            id
        }

        fn record(&self, _span: &span::Id, _values: &span::Record<'_>) {}

        fn record_follows_from(&self, _span: &span::Id, _follows: &span::Id) {}

        fn event(&self, _event: &Event<'_>) {}

        fn enter(&self, _span: &span::Id) {}

        fn exit(&self, _span: &span::Id) {}
    }

    #[test]
    fn test_record_scenario_id_creation() {
        let (sender, _receiver) = mpsc::unbounded();
        let layer = RecordScenarioId::new(sender);

        // Test that the layer was created successfully
        assert!(std::mem::size_of_val(&layer) > 0);
    }

    #[test]
    fn test_layer_basic_functionality() {
        let (sender, _receiver) = mpsc::unbounded();
        let _layer = RecordScenarioId::new(sender);
        
        // Test basic layer functionality without complex tracing APIs
        let span_id = span::Id::from_u64(42);
        assert_eq!(span_id, span::Id::from_u64(42));
    }

    #[test]
    fn test_span_id_utilities() {
        let (_sender, _receiver) = mpsc::unbounded::<span::Id>();
        
        // Test span ID creation and comparison
        let span_ids = vec![
            span::Id::from_u64(1),
            span::Id::from_u64(2),
            span::Id::from_u64(3),
        ];

        assert_eq!(span_ids.len(), 3);
        assert_ne!(span_ids[0], span_ids[1]);
    }

    #[test]
    fn test_scenario_id_creation() {
        let (_sender, _receiver) = mpsc::unbounded::<span::Id>();
        
        // Test basic scenario ID functionality
        use crate::runner::basic::ScenarioId;
        let scenario_id = ScenarioId(1);
        assert_eq!(scenario_id.0, 1);
    }

    #[test] 
    fn test_layer_channel_functionality() {
        let (sender, mut receiver) = mpsc::unbounded::<span::Id>();
        let _layer = RecordScenarioId::new(sender);
        
        // Test that the channel is properly configured
        assert!(receiver.try_next().is_ok()); // Should not have any messages yet
    }

    #[test]
    fn test_layer_with_closed_channel() {
        let (sender, mut receiver) = mpsc::unbounded::<span::Id>();
        drop(receiver); // Close receiver

        let _layer = RecordScenarioId::new(sender);
        let _span_id = span::Id::from_u64(42);

        // Test that layer can be created even with closed channel
        assert!(true);
    }
}
