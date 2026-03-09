//! Event collector for gathering tracing events and managing scenario spans.

use std::collections::HashMap;

use futures::channel::mpsc;
use itertools::Either;
use tracing::span;

use super::{
    types::{
        Callback, LogReceiver, Scenarios, SpanCloseReceiver,
        SpanEventsCallbacks,
    },
    waiter::SpanCloseWaiter,
};
use crate::{
    ScenarioType,
    event::{self, Source},
    runner::basic::{RetryOptions, ScenarioId},
};

/// Collector of [`tracing::Event`]s.
#[derive(Debug)]
pub struct Collector {
    /// [`Scenarios`] with their IDs.
    scenarios: Scenarios,

    /// Receiver of [`tracing::Event`]s messages with optional corresponding
    /// [`ScenarioId`].
    logs_receiver: LogReceiver,

    /// All [`Callback`]s for [`Span`]s closing events with their completion
    /// status.
    span_events: SpanEventsCallbacks,

    /// Receiver of a [`Span`] closing event.
    span_close_receiver: SpanCloseReceiver,

    /// Sender for subscribing to a [`Span`] closing event.
    wait_span_event_sender: mpsc::UnboundedSender<(span::Id, Callback)>,

    /// Receiver for subscribing to a [`Span`] closing event.
    wait_span_event_receiver: mpsc::UnboundedReceiver<(span::Id, Callback)>,
}

impl Collector {
    /// Creates a new [`tracing::Event`]s [`Collector`].
    pub(crate) fn new(
        logs_receiver: LogReceiver,
        span_close_receiver: SpanCloseReceiver,
    ) -> Self {
        let (sender, receiver) = mpsc::unbounded();
        Self {
            scenarios: HashMap::new(),
            logs_receiver,
            span_events: HashMap::new(),
            span_close_receiver,
            wait_span_event_sender: sender,
            wait_span_event_receiver: receiver,
        }
    }

    /// Creates a new [`SpanCloseWaiter`].
    pub(crate) fn scenario_span_event_waiter(&self) -> SpanCloseWaiter {
        SpanCloseWaiter::new(self.wait_span_event_sender.clone())
    }

    /// Starts [`Scenario`]s from the provided `runnable`.
    ///
    /// [`Scenario`]: gherkin::Scenario
    pub(crate) fn start_scenarios(
        &mut self,
        runnable: impl AsRef<
            [(
                ScenarioId,
                Source<gherkin::Feature>,
                Option<Source<gherkin::Rule>>,
                Source<gherkin::Scenario>,
                ScenarioType,
                Option<RetryOptions>,
            )],
        >,
    ) {
        for (id, f, r, s, _, ret) in runnable.as_ref() {
            drop(
                self.scenarios
                    .insert(*id, (f.clone(), r.clone(), s.clone(), *ret)),
            );
        }
    }

    /// Marks a [`Scenario`] as finished, by its ID.
    ///
    /// [`Scenario`]: gherkin::Scenario
    pub(crate) fn finish_scenario(&mut self, id: ScenarioId) {
        drop(self.scenarios.remove(&id));
    }

    /// Returns all the emitted [`event::Scenario::Log`]s since this method was
    /// last called.
    ///
    /// In case a received [`tracing::Event`] doesn't contain a [`Scenario`]'s
    /// [`Span`], such [`tracing::Event`] will be forwarded to all active
    /// [`Scenario`]s.
    ///
    /// [`Scenario`]: gherkin::Scenario
    pub(crate) fn emitted_logs<W>(
        &mut self,
    ) -> Option<Vec<event::Cucumber<W>>> {
        self.notify_about_closing_spans();

        self.logs_receiver.try_next().ok().flatten().map(|(id, msg)| {
            id.and_then(|k| self.scenarios.get(&k))
                .map_or_else(
                    || Either::Left(self.scenarios.values()),
                    |p| Either::Right(std::iter::once(p)),
                )
                .map(|(f, r, s, opt)| {
                    event::Cucumber::scenario(
                        f.clone(),
                        r.clone(),
                        s.clone(),
                        event::RetryableScenario {
                            event: event::Scenario::Log(msg.clone()),
                            retries: opt.map(|o| o.retries),
                        },
                    )
                })
                .collect()
        })
    }

    /// Notifies all its subscribers about closing [`Span`]s via [`Callback`]s.
    fn notify_about_closing_spans(&mut self) {
        if let Some(id) = self.span_close_receiver.try_next().ok().flatten() {
            self.span_events.entry(id).or_default().1 = true;
        }
        while let Some((id, callback)) =
            self.wait_span_event_receiver.try_next().ok().flatten()
        {
            self.span_events
                .entry(id)
                .or_default()
                .0
                .get_or_insert(Vec::new())
                .push(callback);
        }
        self.span_events.retain(|_, (callbacks, is_received)| {
            if callbacks.is_some() && *is_received {
                for callback in callbacks
                    .take()
                    .unwrap_or_else(|| unreachable!("`callbacks.is_some()`"))
                {
                    _ = callback.send(()).ok();
                }
                false
            } else {
                true
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use futures::TryStreamExt;

    use super::*;
    use crate::event::Source;

    #[test]
    fn test_collector_creation() {
        let (logs_sender, logs_receiver) = mpsc::unbounded();
        let (span_sender, span_receiver) = mpsc::unbounded();

        let collector = Collector::new(logs_receiver, span_receiver);
        assert_eq!(collector.scenarios.len(), 0);
        assert_eq!(collector.span_events.len(), 0);

        drop(logs_sender);
        drop(span_sender);
    }

    #[test]
    fn test_scenario_span_event_waiter() {
        let (logs_sender, logs_receiver) = mpsc::unbounded();
        let (span_sender, span_receiver) = mpsc::unbounded();

        let collector = Collector::new(logs_receiver, span_receiver);
        let _waiter = collector.scenario_span_event_waiter();

        drop(logs_sender);
        drop(span_sender);
    }

    #[test]
    fn test_start_scenarios() {
        let (logs_sender, logs_receiver) = mpsc::unbounded();
        let (span_sender, span_receiver) = mpsc::unbounded();

        let mut collector = Collector::new(logs_receiver, span_receiver);

        let feature = gherkin::Feature {
            keyword: "Feature".to_string(),
            name: "Test Feature".to_string(),
            description: None,
            background: None,
            scenarios: vec![],
            rules: vec![],
            tags: vec![],
            span: gherkin::Span { start: 0, end: 0 },
            position: gherkin::LineCol { line: 1, col: 1 },
            path: None,
        };
        let scenario = gherkin::Scenario {
            keyword: "Scenario".to_string(),
            name: "Test Scenario".to_string(),
            description: None,
            steps: vec![],
            tags: vec![],
            span: gherkin::Span { start: 0, end: 0 },
            position: gherkin::LineCol { line: 5, col: 1 },
            examples: vec![],
        };

        let runnable = vec![(
            ScenarioId(1),
            Source::new(feature, None),
            None,
            Source::new(scenario, None),
            ScenarioType::Normal,
            None,
        )];

        collector.start_scenarios(&runnable);
        assert_eq!(collector.scenarios.len(), 1);

        drop(logs_sender);
        drop(span_sender);
    }

    #[test]
    fn test_finish_scenario() {
        let (logs_sender, logs_receiver) = mpsc::unbounded();
        let (span_sender, span_receiver) = mpsc::unbounded();

        let mut collector = Collector::new(logs_receiver, span_receiver);

        let feature = gherkin::Feature {
            keyword: "Feature".to_string(),
            name: "Test Feature".to_string(),
            description: None,
            background: None,
            scenarios: vec![],
            rules: vec![],
            tags: vec![],
            span: gherkin::Span { start: 0, end: 0 },
            position: gherkin::LineCol { line: 1, col: 1 },
            path: None,
        };
        let scenario = gherkin::Scenario {
            keyword: "Scenario".to_string(),
            name: "Test Scenario".to_string(),
            description: None,
            steps: vec![],
            tags: vec![],
            span: gherkin::Span { start: 0, end: 0 },
            position: gherkin::LineCol { line: 5, col: 1 },
            examples: vec![],
        };

        let runnable = vec![(
            ScenarioId(1),
            Source::new(feature, None),
            None,
            Source::new(scenario, None),
            ScenarioType::Normal,
            None,
        )];

        collector.start_scenarios(&runnable);
        assert_eq!(collector.scenarios.len(), 1);

        collector.finish_scenario(ScenarioId(1));
        assert_eq!(collector.scenarios.len(), 0);

        drop(logs_sender);
        drop(span_sender);
    }

    #[test]
    fn test_notify_about_closing_spans() {
        let (logs_sender, logs_receiver) = mpsc::unbounded();
        let (span_sender, span_receiver) = mpsc::unbounded();

        let mut collector = Collector::new(logs_receiver, span_receiver);

        // Send a span close event
        let span_id = span::Id::from_u64(42);
        span_sender.unbounded_send(span_id.clone()).unwrap();

        // This should process the span close event
        collector.notify_about_closing_spans();

        // Verify the span event was recorded
        assert!(collector.span_events.contains_key(&span_id));

        drop(logs_sender);
        drop(span_sender);
    }

    #[tokio::test]
    async fn test_stream_processing() {
        use futures::stream;
        
        let (logs_sender, logs_receiver) = mpsc::unbounded();
        let (span_sender, span_receiver) = mpsc::unbounded();

        // Create a test stream and use TryStreamExt functionality
        let test_stream = stream::iter(vec![
            Ok("log1".to_string()),
            Ok("log2".to_string()),
            Err("error".to_string()),
        ]);

        // Use TryStreamExt to filter and collect results
        let results: Result<Vec<_>, _> = test_stream.try_collect().await;
        
        // Verify stream processing works (expecting error)
        assert!(results.is_err());

        drop(logs_sender);
        drop(span_sender);
        drop(logs_receiver);
        drop(span_receiver);
    }
}
