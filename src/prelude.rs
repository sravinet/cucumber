//! Common types and traits for external integrations

// Re-export observer types when available
#[cfg(feature = "observability")]
pub use crate::observer::{ObservationContext, ObserverRegistry, TestObserver};
// Re-export runner types
pub use crate::runner::Basic as BasicRunner;
// Re-export writer types
pub use crate::writer::{Basic as BasicWriter, Writer};
// Re-export the `World` trait, the `Event` wrapper and core event types
pub use crate::{
    Event, World,
    event::{
        self, Cucumber as CucumberEvent, Feature as FeatureEvent,
        RetryableScenario, Scenario as ScenarioEvent, Source,
        Step as StepEvent,
    },
};
