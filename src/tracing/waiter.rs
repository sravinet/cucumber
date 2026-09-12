//! Span close waiter for managing asynchronous span lifecycle events.

use futures::channel::{mpsc, oneshot};
use tracing::span;

use super::types::Callback;

/// Waiter for a particular [`tracing::Span`] to be closed, which is required because a
/// [`CollectorWriter`] can notify about an [`crate::event::Scenario::Log`] after a
/// [`gherkin::Scenario`]/[`crate::step::Step`] is considered [`Finished`] already, due to
/// implementation details of a [`Subscriber`].
///
/// [`CollectorWriter`]: super::writer::CollectorWriter
/// [`Finished`]: crate::event::Scenario::Finished
/// [`gherkin::Scenario`]: gherkin::Scenario
/// [`crate::step::Step`]: gherkin::Step
/// [`Subscriber`]: tracing::Subscriber
#[derive(Clone, Debug)]
pub struct SpanCloseWaiter {
    /// Sender for subscribing to the [`tracing::Span`] closing.
    wait_span_event_sender: mpsc::UnboundedSender<(span::Id, Callback)>,
}

impl SpanCloseWaiter {
    /// Creates a new [`SpanCloseWaiter`].
    #[must_use]
    pub const fn new(
        wait_span_event_sender: mpsc::UnboundedSender<(span::Id, Callback)>,
    ) -> Self {
        Self { wait_span_event_sender }
    }

    /// Waits for the [`tracing::Span`] being closed.
    ///
    /// Subscribes to the closing of the [`tracing::Span`] and waits for the
    /// [`Collector`] to notify back, so that every
    /// [`crate::event::Scenario::Log`] emitted inside it is forwarded before
    /// its [`gherkin::Scenario`] is considered finished.
    ///
    /// [`Collector`]: super::collector::Collector
    pub async fn wait_for_span_close(&self, id: span::Id) {
        let (sender, receiver) = oneshot::channel();
        _ = self.wait_span_event_sender.unbounded_send((id, sender)).ok();
        _ = receiver.await.ok();
    }
}

#[cfg(test)]
mod tests {
    use futures::StreamExt as _;

    use super::*;

    #[test]
    fn test_span_close_waiter_creation() {
        let (sender, _receiver) = mpsc::unbounded();
        let waiter = SpanCloseWaiter::new(sender);

        // Test that the waiter was created successfully
        assert!(std::mem::size_of_val(&waiter) > 0);
    }

    #[test]
    fn test_span_close_waiter_clone() {
        let (sender, _receiver) = mpsc::unbounded();
        let waiter = SpanCloseWaiter::new(sender);
        let waiter_clone = waiter.clone();

        // Both waiters should be equivalent
        assert!(
            std::mem::size_of_val(&waiter)
                == std::mem::size_of_val(&waiter_clone)
        );
    }

    #[tokio::test]
    async fn test_wait_for_span_close_basic() {
        let (sender, mut receiver) = mpsc::unbounded();
        let waiter = SpanCloseWaiter::new(sender);

        let span_id = span::Id::from_u64(42);
        let wait = tokio::spawn(async move {
            waiter.wait_for_span_close(span_id).await;
        });

        // The subscription is sent, and the wait ends once it's answered.
        let (id, callback) = receiver.next().await.expect("no subscription");
        assert_eq!(id, span::Id::from_u64(42));
        callback.send(()).expect("waiter is gone");

        wait.await.unwrap();
    }

    #[tokio::test]
    async fn test_multiple_span_waiters() {
        let (sender, mut receiver) = mpsc::unbounded();
        let waiter = SpanCloseWaiter::new(sender);

        let span_id_1 = span::Id::from_u64(1);
        let span_id_2 = span::Id::from_u64(2);

        let waiter_1 = waiter.clone();
        let waiter_2 = waiter.clone();

        let wait_handle_1 = tokio::spawn(async move {
            waiter_1.wait_for_span_close(span_id_1).await;
        });
        let wait_handle_2 = tokio::spawn(async move {
            waiter_2.wait_for_span_close(span_id_2).await;
        });

        // Each waiter subscribes on its own, and is answered on its own.
        let mut answered = Vec::new();
        for _ in 0..2 {
            let (id, callback) =
                receiver.next().await.expect("no subscription");
            callback.send(()).expect("waiter is gone");
            answered.push(id);
        }
        answered.sort_by_key(span::Id::into_u64);
        assert_eq!(answered, [span::Id::from_u64(1), span::Id::from_u64(2)]);

        wait_handle_1.await.unwrap();
        wait_handle_2.await.unwrap();
    }

    #[test]
    fn test_waiter_with_closed_sender() {
        let (sender, receiver) = mpsc::unbounded::<(span::Id, Callback)>();
        drop(receiver); // Close the receiver

        let waiter = SpanCloseWaiter::new(sender);
        let span_id = span::Id::from_u64(42);

        // This should handle the closed sender gracefully
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            waiter.wait_for_span_close(span_id).await;
        });
    }
}
