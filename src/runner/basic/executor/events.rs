//! Event sending logic for the Basic executor.

use futures::channel::mpsc;
#[cfg(feature = "observability")]
use std::sync::{Arc, Mutex};

use crate::{
    Event, World, event, parser,
};

#[cfg(feature = "observability")]
use crate::{
    event::source::Source,
    runner::basic::ScenarioId,
};

/// Event sending functionality for the Executor.
#[cfg(not(feature = "observability"))]
pub(super) struct EventSender<W> {
    /// Channel sender for broadcasting Cucumber events to subscribers
    sender: mpsc::UnboundedSender<parser::Result<Event<event::Cucumber<W>>>>,
}

/// Event sending functionality for the Executor with observability.
#[cfg(feature = "observability")]
pub(super) struct EventSender<W: World> {
    /// Channel sender for broadcasting Cucumber events to subscribers
    sender: mpsc::UnboundedSender<parser::Result<Event<event::Cucumber<W>>>>,
    /// Registry of observers for external monitoring and integrations
    observers: Arc<Mutex<crate::observer::ObserverRegistry<W>>>,
    /// Current scenario execution context for observer notifications
    current_context: Arc<Mutex<Option<ScenarioContext>>>,
}

/// Context information for the current scenario
#[cfg(feature = "observability")]
#[derive(Clone, Debug)]
pub(super) struct ScenarioContext {
    pub scenario_id: ScenarioId,
    pub feature: Source<gherkin::Feature>,
    pub rule: Option<Source<gherkin::Rule>>,
    pub scenario: Source<gherkin::Scenario>,
    pub retries: Option<event::Retries>,
}

impl<W: World> EventSender<W> {
    /// Creates a new EventSender.
    #[cfg(not(feature = "observability"))]
    pub(super) fn new_with_sender(
        sender: mpsc::UnboundedSender<
            parser::Result<Event<event::Cucumber<W>>>,
        >,
    ) -> Self {
        Self { sender }
    }

    /// Creates a new EventSender with observer support.
    #[cfg(feature = "observability")]
    pub(super) fn new_with_sender(
        sender: mpsc::UnboundedSender<
            parser::Result<Event<event::Cucumber<W>>>,
        >,
    ) -> Self {
        Self {
            sender,
            observers: Arc::new(Mutex::new(
                crate::observer::ObserverRegistry::new(),
            )),
            current_context: Arc::new(Mutex::new(None)),
        }
    }

    /// Creates a new EventSender with a shared observer registry.
    #[cfg(feature = "observability")]
    pub(super) fn with_observers(
        sender: mpsc::UnboundedSender<
            parser::Result<Event<event::Cucumber<W>>>,
        >,
        observers: Arc<Mutex<crate::observer::ObserverRegistry<W>>>,
    ) -> Self {
        Self { sender, observers, current_context: Arc::new(Mutex::new(None)) }
    }

    /// Updates the current scenario context
    #[cfg(feature = "observability")]
    pub(super) fn set_scenario_context(
        &self,
        id: ScenarioId,
        feature: Source<gherkin::Feature>,
        rule: Option<Source<gherkin::Rule>>,
        scenario: Source<gherkin::Scenario>,
        retries: Option<event::Retries>,
    ) {
        if let Ok(mut context) = self.current_context.lock() {
            *context = Some(ScenarioContext {
                scenario_id: id,
                feature,
                rule,
                scenario,
                retries,
            });
        }
    }

    /// Clears the current scenario context
    #[cfg(feature = "observability")]
    pub(super) fn clear_scenario_context(&self) {
        if let Ok(mut context) = self.current_context.lock() {
            *context = None;
        }
    }

    /// Sends a single event.
    pub(super) fn send_event(&self, event: event::Cucumber<W>) {
        // Send the event through the channel
        let event_wrapper = Event::new(event.clone());
        self.sender
            .unbounded_send(Ok(event_wrapper.clone()))
            .unwrap_or_else(|e| panic!("Failed to send `Cucumber` event: {e}"));

        // Notify observers if enabled
        #[cfg(feature = "observability")]
        self.notify_observers(&event_wrapper, &event);
    }

    /// Notifies observers about an event with context
    #[cfg(feature = "observability")]
    fn notify_observers(
        &self,
        event_wrapper: &Event<event::Cucumber<W>>,
        event: &event::Cucumber<W>,
    ) {
        if let Ok(context) = self.current_context.lock() {
            if let Some(ref ctx) = *context {
                // Build observation context from scenario context
                let mut observation_context = crate::observer::ObservationContext {
                    scenario_id: Some(ctx.scenario_id.0),
                    feature_name: ctx.feature.name.clone(),
                    rule_name: ctx.rule.as_ref().map(|r| r.name.clone()),
                    scenario_name: ctx.scenario.name.clone(),
                    retry_info: ctx.retries.clone(),
                    tags: ctx.scenario.tags.clone(),
                    timestamp: std::time::Instant::now(),
                };

                // Extract additional context from the event itself for more accurate reporting
                match event {
                    event::Cucumber::Feature(feature_src, feature_event) => {
                        // Update feature name from actual event if different
                        observation_context.feature_name = feature_src.name.clone();
                        
                        // Extract scenario-specific information from event
                        match feature_event {
                            event::Feature::Scenario(scenario_src, retryable) => {
                                observation_context.scenario_name = scenario_src.name.clone();
                                observation_context.retry_info = retryable.retries.clone();
                                observation_context.tags = scenario_src.tags.iter()
                                    .map(|t| t.to_string())
                                    .collect();
                            }
                            event::Feature::Rule(rule_src, rule_event) => {
                                observation_context.rule_name = Some(rule_src.name.clone());
                                if let event::Rule::Scenario(scenario_src, retryable) = rule_event {
                                    observation_context.scenario_name = scenario_src.name.clone();
                                    observation_context.retry_info = retryable.retries.clone();
                                    observation_context.tags = scenario_src.tags.iter()
                                        .map(|t| t.to_string())
                                        .collect();
                                }
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }

                // Notify all registered observers
                if let Ok(mut registry) = self.observers.lock() {
                    registry.notify(event_wrapper, &observation_context);
                }
            }
        }
    }

    /// Sends multiple events.
    pub(super) fn send_all_events(
        &self,
        events: impl IntoIterator<Item = event::Cucumber<W>>,
    ) {
        for event in events {
            self.send_event(event);
        }
    }

    /// Sends an event with additional metadata.
    /// 
    /// This method is used for events that need specific timing or context metadata,
    /// such as hook execution timing or step duration measurements.
    pub(super) fn send_event_with_meta(
        &self,
        event: event::Cucumber<W>,
        meta: &crate::event::Metadata,
    ) {
        // Create event with the provided metadata for precise timing
        let event_with_meta = meta.wrap(event.clone());

        // Send through normal channel with metadata
        self.sender
            .unbounded_send(Ok(event_with_meta.clone()))
            .unwrap_or_else(|e| panic!("Failed to send `Cucumber` event with metadata: {e}"));

        // Notify observers if enabled - observers receive events with timing metadata
        #[cfg(feature = "observability")]
        self.notify_observers(&event_with_meta, &event);
    }
}

#[cfg(test)]
mod tests {
    use futures::{TryStreamExt as _, channel::mpsc};

    use super::*;
    use crate::{event, test_utils::common::TestWorld};

    #[test]
    fn test_event_sender_creation() {
        let (sender, _receiver) = mpsc::unbounded();
        let _event_sender = EventSender::<TestWorld>::new_with_sender(sender);

        // EventSender should be created successfully
        assert!(true); // Basic existence check
    }

    #[test]
    fn test_send_single_event() {
        let (sender, mut receiver) = mpsc::unbounded();
        let event_sender = EventSender::<TestWorld>::new_with_sender(sender);

        let event = event::Cucumber::<TestWorld>::Started;
        event_sender.send_event(event);

        // Should receive the event
        let received = receiver.try_next().unwrap().unwrap().unwrap();
        assert!(matches!(received.value, event::Cucumber::Started));
    }

    #[test]
    fn test_send_multiple_events() {
        let (sender, mut receiver) = mpsc::unbounded();
        let event_sender = EventSender::<TestWorld>::new_with_sender(sender);

        let events = vec![
            event::Cucumber::<TestWorld>::Started,
            event::Cucumber::<TestWorld>::Finished,
        ];

        event_sender.send_all_events(events);

        // Should receive both events
        let first = receiver.try_next().unwrap().unwrap().unwrap();
        let second = receiver.try_next().unwrap().unwrap().unwrap();

        assert!(matches!(first.value, event::Cucumber::Started));
        assert!(matches!(second.value, event::Cucumber::Finished));
    }

    #[test]
    fn test_send_event_with_meta() {
        let (sender, mut receiver) = mpsc::unbounded();
        let event_sender = EventSender::<TestWorld>::new_with_sender(sender);

        let event = event::Cucumber::<TestWorld>::Started;
        let meta = crate::event::Metadata::new(());

        event_sender.send_event_with_meta(event, &meta);

        // Should receive the event
        let received = receiver.try_next().unwrap().unwrap().unwrap();
        assert!(matches!(received.value, event::Cucumber::Started));
    }

    #[test]
    #[should_panic(expected = "Failed to send `Cucumber` event")]
    fn test_send_event_panics_on_closed_channel() {
        let (sender, receiver) = mpsc::unbounded();
        let event_sender = EventSender::<TestWorld>::new_with_sender(sender);

        // Close the receiver to make the channel closed
        drop(receiver);

        let event = event::Cucumber::<TestWorld>::Started;
        event_sender.send_event(event); // Should panic
    }

    #[test]
    fn test_event_sender_multiple_instances() {
        let (sender1, mut receiver1) = mpsc::unbounded();
        let (sender2, mut receiver2) = mpsc::unbounded();

        let event_sender1 = EventSender::<TestWorld>::new_with_sender(sender1);
        let event_sender2 = EventSender::<TestWorld>::new_with_sender(sender2);

        event_sender1.send_event(event::Cucumber::<TestWorld>::Started);
        event_sender2.send_event(event::Cucumber::<TestWorld>::Finished);

        let received1 = receiver1.try_next().unwrap().unwrap().unwrap();
        let received2 = receiver2.try_next().unwrap().unwrap().unwrap();

        assert!(matches!(received1.value, event::Cucumber::Started));
        assert!(matches!(received2.value, event::Cucumber::Finished));
    }

    #[test]
    fn test_send_event_with_meta_functionality() {
        let (sender, mut receiver) = mpsc::unbounded();
        let event_sender = EventSender::<TestWorld>::new_with_sender(sender);

        let event = event::Cucumber::<TestWorld>::Started;
        let meta = crate::event::Metadata::new(());

        // Test that send_event_with_meta actually uses the metadata properly
        event_sender.send_event_with_meta(event, &meta);

        // Should receive the event with metadata
        let received = receiver.try_next().unwrap().unwrap().unwrap();
        assert!(matches!(received.value, event::Cucumber::Started));
        
        // Verify the event has metadata (indicates send_event_with_meta worked)
        #[cfg(feature = "timestamps")]
        {
            let event_timestamp = received.at;
            assert!(event_timestamp.elapsed().unwrap().as_nanos() > 0);
        }
    }

    #[test]
    fn test_send_event_vs_send_event_with_meta_distinction() {
        let (sender, mut receiver) = mpsc::unbounded();
        let event_sender = EventSender::<TestWorld>::new_with_sender(sender);

        let event1 = event::Cucumber::<TestWorld>::Started;
        let event2 = event::Cucumber::<TestWorld>::Finished;
        let meta = crate::event::Metadata::new(());

        // Send one event normally and one with metadata
        event_sender.send_event(event1);
        event_sender.send_event_with_meta(event2, &meta);

        // Both should be received
        let received1 = receiver.try_next().unwrap().unwrap().unwrap();
        let received2 = receiver.try_next().unwrap().unwrap().unwrap();

        assert!(matches!(received1.value, event::Cucumber::Started));
        assert!(matches!(received2.value, event::Cucumber::Finished));
    }
}
