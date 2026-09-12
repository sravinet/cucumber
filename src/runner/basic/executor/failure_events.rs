//! Emitting of the [`gherkin::Scenario`] events that carry its [`World`].
//!
//! The [`HookType::After`] hook takes the [`World`] by mutable reference,
//! while the failure events store it by shared one, for easier debugging. To
//! avoid requiring the [`World`] to be [`Clone`], the hook runs first, without
//! emitting anything, and only then are the events emitted: the failure of the
//! [`crate::step::Step`] or the [`HookType::Before`] hook that ended the
//! [`gherkin::Scenario`], and after it the [`HookType::After`] hook's own
//! ones. This keeps the [`order guarantees`][1] without restricting the
//! [`HookType::After`] hook to a shared [`World`]. The only downside is that
//! the failure events may report a [`World`] the [`HookType::After`] hook has
//! since changed.
//!
//! [1]: crate::Runner#order-guarantees
//! [`crate::step::Step`]: gherkin::Step

use std::sync::Arc;

use super::super::supporting_structures::{
    AfterHookEventsMeta, BeforeHookPanicked, StepFailure,
};
use crate::{
    World,
    event::{self, HookType, Info, Metadata, Retries, source::Source},
};

/// What ended a [`gherkin::Scenario`] before its [`crate::step::Step`]s were
/// through.
///
/// [`crate::step::Step`]: gherkin::Step
pub(super) enum ScenarioFailure {
    /// [`HookType::Before`] hook panicked.
    BeforeHook(BeforeHookPanicked),

    /// [`crate::step::Step`] failed.
    ///
    /// [`crate::step::Step`]: gherkin::Step
    Step(StepFailure),
}

/// Emits the failure event of whatever ended the given [`gherkin::Scenario`],
/// with the [`World`] it ended with.
pub(super) fn emit_failure_event<W: World>(
    feature: Source<gherkin::Feature>,
    rule: Option<Source<gherkin::Rule>>,
    scenario: Source<gherkin::Scenario>,
    world: Option<Arc<W>>,
    failure: ScenarioFailure,
    retries: Option<Retries>,
    send_event: &impl Fn(event::Cucumber<W>, &Metadata),
) {
    let (ev, meta) = match failure {
        ScenarioFailure::BeforeHook(BeforeHookPanicked {
            panic_info,
            meta,
        }) => (
            event::Scenario::hook_failed(HookType::Before, world, panic_info),
            meta,
        ),
        ScenarioFailure::Step(StepFailure {
            step,
            captures,
            loc,
            err,
            meta,
            is_background,
        }) => {
            let ev = if is_background {
                event::Scenario::background_step_failed(
                    step, captures, loc, world, err,
                )
            } else {
                event::Scenario::step_failed(step, captures, loc, world, err)
            };
            (ev, meta)
        }
    };

    send_event(
        event::Cucumber::scenario(
            feature,
            rule,
            scenario,
            ev.with_retries(retries),
        ),
        &meta,
    );
}

/// Emits the [`HookType::After`] hook events of the given
/// [`gherkin::Scenario`], with the [`World`] the hook has left behind.
pub(super) fn emit_after_hook_events<W: World>(
    feature: Source<gherkin::Feature>,
    rule: Option<Source<gherkin::Rule>>,
    scenario: Source<gherkin::Scenario>,
    world: Option<Arc<W>>,
    meta: AfterHookEventsMeta,
    err: Option<Info>,
    retries: Option<Retries>,
    send_event: &impl Fn(event::Cucumber<W>, &Metadata),
) {
    send_event(
        event::Cucumber::scenario(
            feature.clone(),
            rule.clone(),
            scenario.clone(),
            event::Scenario::hook_started(HookType::After)
                .with_retries(retries),
        ),
        &meta.started,
    );

    let ev = err.map_or_else(
        || event::Scenario::hook_passed(HookType::After),
        |info| event::Scenario::hook_failed(HookType::After, world, info),
    );
    send_event(
        event::Cucumber::scenario(
            feature,
            rule,
            scenario,
            ev.with_retries(retries),
        ),
        &meta.finished,
    );
}

/// Wraps the given [`World`] into an [`Arc`], to be shared by the events
/// reporting it.
pub(super) fn share<W: World>(world: W) -> Option<Arc<W>> {
    Some(Arc::new(world))
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::{event::Metadata, test_utils::common::TestWorld};

    fn test_sources() -> (Source<gherkin::Feature>, Source<gherkin::Scenario>) {
        let scenario = gherkin::Scenario {
            keyword: "Scenario".to_owned(),
            name: "test".to_owned(),
            description: None,
            steps: vec![],
            examples: vec![],
            tags: vec![],
            span: gherkin::Span { start: 0, end: 0 },
            position: gherkin::LineCol { line: 1, col: 1 },
        };
        let feature = gherkin::Feature {
            keyword: "Feature".to_owned(),
            name: "test".to_owned(),
            description: None,
            background: None,
            scenarios: vec![scenario.clone()],
            rules: vec![],
            tags: vec![],
            span: gherkin::Span { start: 0, end: 0 },
            position: gherkin::LineCol { line: 1, col: 1 },
            path: None,
        };

        (Source::new(feature), Source::new(scenario))
    }

    #[test]
    fn before_hook_failure_carries_the_world() {
        let (feature, scenario) = test_sources();
        let events = Arc::new(Mutex::new(Vec::new()));
        let sent = Arc::clone(&events);

        emit_failure_event(
            feature,
            None,
            scenario,
            share(TestWorld),
            ScenarioFailure::BeforeHook(BeforeHookPanicked {
                panic_info: Arc::new("boom"),
                meta: Metadata::new(()),
            }),
            None,
            &move |ev: event::Cucumber<TestWorld>, _: &Metadata| {
                sent.lock().unwrap().push(ev);
            },
        );

        let events = events.lock().unwrap();
        assert_eq!(events.len(), 1);
        match &events[0] {
            event::Cucumber::Feature(
                _,
                event::Feature::Scenario(
                    _,
                    event::RetryableScenario {
                        event:
                            event::Scenario::Hook(
                                HookType::Before,
                                event::Hook::Failed(world, _),
                            ),
                        ..
                    },
                ),
            ) => assert!(world.is_some(), "`World` is not reported"),
            _ => panic!("expected a failed `Before` hook event"),
        }
    }

    #[test]
    fn after_hook_events_come_in_order() {
        let (feature, scenario) = test_sources();
        let events = Arc::new(Mutex::new(Vec::new()));
        let sent = Arc::clone(&events);

        emit_after_hook_events(
            feature,
            None,
            scenario,
            share(TestWorld),
            AfterHookEventsMeta {
                started: Metadata::new(()),
                finished: Metadata::new(()),
            },
            None,
            None,
            &move |ev: event::Cucumber<TestWorld>, _: &Metadata| {
                sent.lock().unwrap().push(ev);
            },
        );

        let events = events.lock().unwrap();
        assert_eq!(events.len(), 2);
        assert!(matches!(
            &events[0],
            event::Cucumber::Feature(
                _,
                event::Feature::Scenario(
                    _,
                    event::RetryableScenario {
                        event: event::Scenario::Hook(
                            HookType::After,
                            event::Hook::Started,
                        ),
                        ..
                    },
                ),
            ),
        ));
        assert!(matches!(
            &events[1],
            event::Cucumber::Feature(
                _,
                event::Feature::Scenario(
                    _,
                    event::RetryableScenario {
                        event: event::Scenario::Hook(
                            HookType::After,
                            event::Hook::Passed,
                        ),
                        ..
                    },
                ),
            ),
        ));
    }
}
