/// Observer trait for external systems like ObservaBDD
///
/// This provides a lightweight integration point for observability
/// without adding runtime overhead when not in use.
use crate::{Event, World, event};

/// Context provided to observers containing execution metadata
#[derive(Clone, Debug)]
pub struct ObservationContext {
    /// Unique identifier for the scenario being observed
    pub scenario_id: Option<u64>,
    /// Name of the feature containing the scenario
    pub feature_name: String,
    /// Name of the rule containing the scenario (if any)
    pub rule_name: Option<String>,
    /// Name of the scenario being executed
    pub scenario_name: String,
    /// Information about retry attempts for this scenario execution
    pub retry_info: Option<event::Retries>,
    /// Tags associated with the scenario for filtering and categorization
    pub tags: Vec<String>,
    /// Timestamp when this observation context was created
    pub timestamp: std::time::Instant,
}

/// Observer trait for monitoring test execution
pub trait TestObserver<W: World>: Send + Sync {
    /// Called when an event occurs
    fn on_event(
        &mut self,
        event: &Event<event::Cucumber<W>>,
        context: &ObservationContext,
    );

    /// Called when execution starts
    fn on_start(&mut self) {}

    /// Called when execution completes
    fn on_finish(&mut self) {}
}

/// No-op observer for when observation is disabled
#[derive(Clone, Copy, Debug)]
pub struct NullObserver;

impl<W: World> TestObserver<W> for NullObserver {
    fn on_event(
        &mut self,
        _: &Event<event::Cucumber<W>>,
        _: &ObservationContext,
    ) {
    }
}

/// Registry for managing multiple observers
/// 
/// Provides efficient batch notification to all registered observers
/// with minimal overhead when no observers are registered.
pub struct ObserverRegistry<W> {
    observers: Vec<Box<dyn TestObserver<W>>>,
    enabled: bool,
}

impl<W> ObserverRegistry<W> {
    /// Creates a new empty observer registry
    /// 
    /// The registry starts with no observers and is initially disabled
    /// for optimal performance when observation is not needed.
    pub fn new() -> Self {
        Self { observers: Vec::new(), enabled: false }
    }

    /// Registers a new observer with the registry
    /// 
    /// Once an observer is registered, the registry is automatically
    /// enabled and will notify all observers of future events.
    pub fn register(&mut self, observer: Box<dyn TestObserver<W>>)
    where
        W: World,
    {
        self.observers.push(observer);
        self.enabled = true;
    }

    /// Notifies all registered observers about an event
    /// 
    /// This method is optimized to skip all processing when no
    /// observers are registered, providing zero-cost observation
    /// when not in use.
    #[inline]
    pub fn notify(
        &mut self,
        event: &Event<event::Cucumber<W>>,
        context: &ObservationContext,
    ) where
        W: World,
    {
        if self.enabled {
            for observer in &mut self.observers {
                observer.on_event(event, context);
            }
        }
    }
}

impl<W: World> Default for ObserverRegistry<W> {
    fn default() -> Self {
        Self::new()
    }
}

impl<W> std::fmt::Debug for ObserverRegistry<W> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ObserverRegistry")
            .field("observer_count", &self.observers.len())
            .field("enabled", &self.enabled)
            .finish()
    }
}
