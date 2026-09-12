//! Supporting structures and utilities for the Basic runner.

use std::{
    any::Any,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use derive_more::with_trait::{Display, FromStr};
use regex::CaptureLocations;

use crate::{
    event::{self, Info, Metadata, source::Source},
    step,
};

/// ID of a [`gherkin::Scenario`], uniquely identifying it.
///
/// **NOTE**: Retried [`gherkin::Scenario`] has a different ID from a failed one.
///
/// [`gherkin::Scenario`]: gherkin::Scenario
#[derive(Clone, Copy, Debug, Display, Eq, FromStr, Hash, PartialEq)]
pub struct ScenarioId(pub u64);

impl ScenarioId {
    /// Creates a new unique [`ScenarioId`].
    pub fn new() -> Self {
        /// [`AtomicU64`] ID.
        static ID: AtomicU64 = AtomicU64::new(0);

        Self(ID.fetch_add(1, Ordering::Relaxed))
    }
}

impl Default for ScenarioId {
    fn default() -> Self {
        Self::new()
    }
}

/// Alias for a failed [`gherkin::Scenario`].
///
/// [`gherkin::Scenario`]: gherkin::Scenario
pub(super) type IsFailed = bool;

/// Alias for a retried [`gherkin::Scenario`].
///
/// [`gherkin::Scenario`]: gherkin::Scenario
pub(super) type IsRetried = bool;

/// Panic of an [`event::HookType::Before`] hook, aborting its
/// [`gherkin::Scenario`].
#[derive(Debug)]
pub(super) struct BeforeHookPanicked {
    /// [`catch_unwind()`] of the [`event::HookType::Before`] panic.
    ///
    /// [`catch_unwind()`]: std::panic::catch_unwind
    pub(super) panic_info: Info,

    /// [`Metadata`] at the time the [`event::HookType::Before`] hook panicked.
    pub(super) meta: Metadata,
}

impl BeforeHookPanicked {
    /// Creates an [`event::ScenarioFinished`] out of this panic, describing
    /// the [`gherkin::Scenario`] outcome to the [`event::HookType::After`]
    /// hook.
    pub(super) fn scenario_finished_event(&self) -> event::ScenarioFinished {
        event::ScenarioFinished::BeforeHookFailed(Arc::clone(&self.panic_info))
    }
}

/// Failure of a [`crate::step::Step`], aborting its [`gherkin::Scenario`].
///
/// [`crate::step::Step`]: gherkin::Step
#[derive(Debug)]
pub(super) struct StepFailure {
    /// [`crate::step::Step`] that failed.
    ///
    /// [`crate::step::Step`]: gherkin::Step
    pub(super) step: Source<gherkin::Step>,

    /// [`regex`] [`CaptureLocations`] of the [`crate::step::Step`], if it
    /// matched a function at all.
    ///
    /// [`crate::step::Step`]: gherkin::Step
    pub(super) captures: Option<CaptureLocations>,

    /// [`Location`] of the [`fn`] that matched this [`crate::step::Step`].
    ///
    /// [`Location`]: step::Location
    /// [`crate::step::Step`]: gherkin::Step
    pub(super) loc: Option<step::Location>,

    /// [`event::StepError`] of the [`crate::step::Step`].
    ///
    /// [`crate::step::Step`]: gherkin::Step
    pub(super) err: event::StepError,

    /// [`Metadata`] at the time the [`crate::step::Step`] failed.
    ///
    /// [`crate::step::Step`]: gherkin::Step
    pub(super) meta: Metadata,

    /// Indicator whether the [`crate::step::Step`] was a [`Background`] one.
    ///
    /// [`Background`]: gherkin::Background
    /// [`crate::step::Step`]: gherkin::Step
    pub(super) is_background: bool,
}

/// Outcome of running the [`crate::step::Step`]s of a [`gherkin::Scenario`].
///
/// [`crate::step::Step`]: gherkin::Step
#[derive(Debug)]
pub(super) enum StepsOutcome {
    /// Every [`crate::step::Step`] passed.
    ///
    /// [`crate::step::Step`]: gherkin::Step
    Passed,

    /// A [`crate::step::Step`] matched no function, ending the
    /// [`gherkin::Scenario`].
    ///
    /// [`crate::step::Step`]: gherkin::Step
    Skipped,

    /// A [`crate::step::Step`] failed, ending the [`gherkin::Scenario`].
    ///
    /// [`crate::step::Step`]: gherkin::Step
    Failed(StepFailure),
}

impl StepsOutcome {
    /// Creates an [`event::ScenarioFinished`] out of this outcome, describing
    /// the [`gherkin::Scenario`] outcome to the [`event::HookType::After`]
    /// hook.
    pub(super) fn scenario_finished_event(&self) -> event::ScenarioFinished {
        match self {
            Self::Passed => event::ScenarioFinished::StepPassed,
            Self::Skipped => event::ScenarioFinished::StepSkipped,
            Self::Failed(f) => event::ScenarioFinished::StepFailed(
                f.captures.clone(),
                f.loc,
                f.err.clone(),
            ),
        }
    }
}

/// [`Metadata`] of the [`event::HookType::After`] hook run.
///
/// Its events are emitted only after the failure events of whatever ended the
/// [`gherkin::Scenario`], so their [`Metadata`] is taken while the hook runs
/// and carried to the emitting.
pub(super) struct AfterHookEventsMeta {
    /// [`Metadata`] at the time the [`event::HookType::After`] hook started.
    pub(super) started: Metadata,

    /// [`Metadata`] at the time the [`event::HookType::After`] hook finished.
    pub(super) finished: Metadata,
}

/// Coerces the given `value` into a type-erased [`Info`].
pub(super) fn coerce_into_info<T: Any + Send + 'static>(val: T) -> Info {
    Arc::new(val)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scenario_id_creation() {
        let id1 = ScenarioId::new();
        let id2 = ScenarioId::new();

        assert_ne!(id1, id2);
        assert!(id2.0 > id1.0);
    }

    #[test]
    fn test_scenario_id_default() {
        let id1 = ScenarioId::default();
        let id2 = ScenarioId::default();

        assert_ne!(id1, id2);
    }

    #[test]
    fn test_scenario_id_display() {
        let id = ScenarioId(42);
        assert_eq!(format!("{}", id), "42");
    }

    #[test]
    fn test_scenario_id_hash() {
        use std::collections::HashMap;

        let id1 = ScenarioId(1);
        let id2 = ScenarioId(2);

        let mut map = HashMap::new();
        map.insert(id1, "first");
        map.insert(id2, "second");

        assert_eq!(map.get(&id1), Some(&"first"));
        assert_eq!(map.get(&id2), Some(&"second"));
    }

    #[cfg(feature = "tracing")]
    #[test]
    fn test_scenario_tracing_spans() {
        let id = ScenarioId(123);

        let scenario_span = id.scenario_span();
        let step_span = id.step_span(false);
        let bg_step_span = id.step_span(true);
        let before_hook_span = id.hook_span(event::HookType::Before);
        let after_hook_span = id.hook_span(event::HookType::After);

        // Just test that spans are created without errors - metadata might be None without a subscriber
        if let Some(metadata) = scenario_span.metadata() {
            assert_eq!(metadata.name(), "scenario");
        }
        if let Some(metadata) = step_span.metadata() {
            assert_eq!(metadata.name(), "step");
        }
        if let Some(metadata) = bg_step_span.metadata() {
            assert_eq!(metadata.name(), "background step");
        }
        if let Some(metadata) = before_hook_span.metadata() {
            assert_eq!(metadata.name(), "before hook");
        }
        if let Some(metadata) = after_hook_span.metadata() {
            assert_eq!(metadata.name(), "after hook");
        }

        // Test that spans can be created regardless of metadata availability
        assert!(std::mem::size_of_val(&scenario_span) > 0);
        assert!(std::mem::size_of_val(&step_span) > 0);
        assert!(std::mem::size_of_val(&bg_step_span) > 0);
        assert!(std::mem::size_of_val(&before_hook_span) > 0);
        assert!(std::mem::size_of_val(&after_hook_span) > 0);
    }

    #[test]
    fn test_coerce_into_info() {
        let info = coerce_into_info("test string");

        // Should be able to downcast back
        assert!(info.downcast_ref::<&str>().is_some());
    }

    #[test]
    fn test_before_hook_panicked_scenario_finished_event() {
        let failure = BeforeHookPanicked {
            panic_info: coerce_into_info("panic message"),
            meta: Metadata::new(()),
        };

        match failure.scenario_finished_event() {
            event::ScenarioFinished::BeforeHookFailed(info) => {
                assert_eq!(
                    info.downcast_ref::<&str>().copied(),
                    Some("panic message"),
                );
            }
            _ => panic!("Expected `BeforeHookFailed` outcome"),
        }
    }
}
