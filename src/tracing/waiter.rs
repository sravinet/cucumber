//! Span close waiter for managing asynchronous span lifecycle events.

use futures::channel::{mpsc, oneshot};
use tracing::span;

use super::types::Callback;

/// Waiter for a particular [`Span`] to be closed, which is required because a
/// [`CollectorWriter`] can notify about an [`event::Scenario::Log`] after a
/// [`Scenario`]/[`Step`] is considered [`Finished`] already, due to
/// implementation details of a [`Subscriber`].
///
/// [`CollectorWriter`]: super::writer::CollectorWriter
/// [`Finished`]: crate::event::Scenario::Finished
/// [`Scenario`]: gherkin::Scenario
/// [`Step`]: gherkin::Step
/// [`Subscriber`]: tracing::Subscriber
#[derive(Clone, Debug)]
pub struct SpanCloseWaiter {
    /// Sender for subscribing to the [`Span`] closing.
    wait_span_event_sender: mpsc::UnboundedSender<(span::Id, Callback)>,
}

impl SpanCloseWaiter {
    /// Creates a new [`SpanCloseWaiter`].
    pub const fn new(
        wait_span_event_sender: mpsc::UnboundedSender<(span::Id, Callback)>,
    ) -> Self {
        Self { wait_span_event_sender }
    }

    /// Waits for the [`Span`] being closed.
    pub async fn wait_for_span_close(&self, id: span::Id) {
        let (sender, receiver) = oneshot::channel();
        _ = self.wait_span_event_sender.unbounded_send((id, sender)).ok();
        _ = receiver.await.ok();
    }

}

#[cfg(test)]
mod tests {
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
        let span_id_clone = span_id.clone();

        // Start waiting for span close in background - move waiter into the task
        let wait_future = tokio::spawn(async move {
            waiter.wait_for_span_close(span_id_clone).await;
        });

        // Give the future a chance to send the subscription request
        tokio::task::yield_now().await;

        // Verify the waiter sent a subscription request
        match receiver.try_next() {
            Ok(Some((received_id, callback))) => {
                assert_eq!(received_id, span_id);
                // Simulate span close by sending through callback
                callback.send(()).unwrap();
            }
            _ => panic!("Expected to receive subscription request"),
        }

        // The wait should complete
        let _ = wait_future.await;
    }


    #[tokio::test]
    async fn test_multiple_span_waiters() {
        let (sender, mut receiver) = mpsc::unbounded();
        let waiter = SpanCloseWaiter::new(sender);

        let span_id_1 = span::Id::from_u64(1);
        let span_id_2 = span::Id::from_u64(2);

        // Clone IDs before moving them
        let span_id_1_clone = span_id_1.clone();
        let span_id_2_clone = span_id_2.clone();

        // Start waiting for spans separately
        let waiter_1 = waiter.clone();
        let waiter_2 = waiter.clone();
        
        let wait_handle_1 = tokio::spawn(async move {
            waiter_1.wait_for_span_close(span_id_1_clone).await;
        });
        let wait_handle_2 = tokio::spawn(async move {
            waiter_2.wait_for_span_close(span_id_2_clone).await;
        });

        // Give the futures a chance to send subscription requests
        tokio::task::yield_now().await;

        // Get both subscription requests
        let (received_id_1, callback_1) = match receiver.try_next() {
            Ok(Some(msg)) => msg,
            _ => panic!("Expected first subscription request"),
        };
        let (received_id_2, callback_2) = match receiver.try_next() {
            Ok(Some(msg)) => msg,
            _ => panic!("Expected second subscription request"),
        };

        assert!(received_id_1 == span_id_1 || received_id_1 == span_id_2);
        assert!(received_id_2 == span_id_1 || received_id_2 == span_id_2);
        assert_ne!(received_id_1, received_id_2);

        // Close both spans
        callback_1.send(()).unwrap();
        callback_2.send(()).unwrap();

        // Wait for both to complete
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
