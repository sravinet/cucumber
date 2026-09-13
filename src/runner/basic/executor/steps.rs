//! Step execution logic for the Basic executor.

use std::panic::AssertUnwindSafe;

use futures::FutureExt as _;

use super::super::supporting_structures::{
    ScenarioId, StepFailure, StepsOutcome, coerce_into_info,
};
use crate::{
    Event, World,
    event::{self, source::Source},
    step,
};

/// Step execution functionality for the Executor.
pub(super) struct StepExecutor;

impl StepExecutor {
    /// Runs all steps for a scenario.
    pub(super) async fn run_steps<W>(
        collection: &step::Collection<W>,
        id: ScenarioId,
        feature: Source<gherkin::Feature>,
        rule: Option<Source<gherkin::Rule>>,
        scenario: Source<gherkin::Scenario>,
        world: &mut W,
        retries: Option<crate::event::Retries>,
        send_event: impl Fn(event::Cucumber<W>) + Clone,
        #[cfg(feature = "tracing")] waiter: Option<
            &crate::tracing::SpanCloseWaiter,
        >,
    ) -> StepsOutcome
    where
        W: World,
    {
        // `Background` `Step`s run before the `Scenario`s own ones: those of
        // the `Feature` first, then those of the `Rule`, if any.
        let background = feature
            .background
            .iter()
            .chain(rule.iter().filter_map(|r| r.background.as_ref()))
            .flat_map(|bg| bg.steps.iter().map(|s| (s, true)));
        let all_steps = background
            .chain(scenario.steps.iter().map(|s| (s, false)))
            .map(|(s, is_bg)| (Source::new(s.clone()), is_bg))
            .collect::<Vec<_>>();

        // A `Step` that fails or is skipped ends its `Scenario`: the `Step`s
        // after it are neither run nor reported, so a `Scenario` yields
        // exactly one terminal `Step` event.
        for (step, is_background) in all_steps {
            let outcome = Self::run_step(
                collection,
                id,
                feature.clone(),
                rule.clone(),
                scenario.clone(),
                step,
                is_background,
                world,
                retries,
                send_event.clone(),
                #[cfg(feature = "tracing")]
                waiter,
            )
            .await;

            match outcome {
                StepsOutcome::Passed => {}
                outcome => return outcome,
            }
        }

        StepsOutcome::Passed
    }

    /// Runs a single step.
    #[expect(clippy::too_many_arguments, reason = "needs refactoring")]
    async fn run_step<W>(
        collection: &step::Collection<W>,
        _id: ScenarioId,
        feature: Source<gherkin::Feature>,
        rule: Option<Source<gherkin::Rule>>,
        scenario: Source<gherkin::Scenario>,
        step: Source<gherkin::Step>,
        is_background: bool,
        world: &mut W,
        retries: Option<crate::event::Retries>,
        send_event: impl Fn(event::Cucumber<W>),
        #[cfg(feature = "tracing")] waiter: Option<
            &crate::tracing::SpanCloseWaiter,
        >,
    ) -> StepsOutcome
    where
        W: World,
    {
        Self::emit_step_event(
            feature.clone(),
            rule.clone(),
            scenario.clone(),
            step.clone(),
            event::Step::Started,
            retries,
            is_background,
            &send_event,
        );

        #[cfg(feature = "tracing")]
        let span = _id.step_span(is_background);
        #[cfg(feature = "tracing")]
        let span_id = span.id();

        let (result, location, captures) = match collection.find(&*step) {
            Ok(Some((step_fn, captures, loc, ctx))) => {
                let run = AssertUnwindSafe(step_fn(world, ctx)).catch_unwind();
                // Instrumenting the future, rather than entering the `Span`,
                // is what keeps concurrently running `Scenario`s out of it:
                // an entered guard stays on the thread across `.await`s, so
                // every `Span` opened meanwhile would nest inside this one.
                #[cfg(feature = "tracing")]
                let run = tracing::Instrument::instrument(run, span);

                (run.await, loc, Some(captures))
            }
            Ok(None) => {
                // A `Step` matching no function is skipped, not failed.
                // `WriterExt::fail_on_skipped()` is what turns this into a
                // `StepError::NotFound` failure, when that's wanted.
                Self::emit_step_event(
                    feature,
                    rule,
                    scenario,
                    step,
                    event::Step::Skipped,
                    retries,
                    is_background,
                    &send_event,
                );
                return StepsOutcome::Skipped;
            }
            Err(ambiguous) => {
                // The `Step` never ran, so there's no `World` state worth
                // reporting, and nothing to wait for.
                return StepsOutcome::Failed(StepFailure {
                    step,
                    captures: None,
                    loc: None,
                    err: event::StepError::AmbiguousMatch(ambiguous),
                    meta: event::Metadata::new(()),
                    is_background,
                });
            }
        };

        #[cfg(feature = "tracing")]
        if let Some((waiter, span_id)) = waiter.zip(span_id) {
            waiter.wait_for_span_close(span_id).await;
        }

        match result {
            Ok(()) => {
                Self::emit_step_event(
                    feature,
                    rule,
                    scenario,
                    step,
                    event::Step::Passed {
                        captures: captures.unwrap_or_else(|| {
                            regex::Regex::new("").unwrap().capture_locations()
                        }),
                        location,
                    },
                    retries,
                    is_background,
                    &send_event,
                );
                StepsOutcome::Passed
            }
            // The `Step::Failed` event isn't emitted here: it carries the
            // `World`, which the `After` hook still needs mutably. See
            // `failure_events` for the whole story.
            Err(panic_info) => StepsOutcome::Failed(StepFailure {
                step,
                captures,
                loc: location,
                err: event::StepError::Panic(coerce_into_info(panic_info)),
                meta: event::Metadata::new(()),
                is_background,
            }),
        }
    }

    /// Emits the given [`event::Step`] event of a regular or a [`Background`]
    /// [`crate::step::Step`].
    ///
    /// [`Background`]: gherkin::Background
    /// [`crate::step::Step`]: gherkin::Step
    fn emit_step_event<W>(
        feature: Source<gherkin::Feature>,
        rule: Option<Source<gherkin::Rule>>,
        scenario: Source<gherkin::Scenario>,
        step: Source<gherkin::Step>,
        step_event: event::Step<W>,
        retries: Option<crate::event::Retries>,
        is_background: bool,
        send_event: &impl Fn(event::Cucumber<W>),
    ) where
        W: World,
    {
        let scenario_event = if is_background {
            event::Scenario::Background(step, step_event)
        } else {
            event::Scenario::Step(step, step_event)
        };

        let event = Event::new(event::Cucumber::scenario(
            feature,
            rule,
            scenario,
            event::RetryableScenario { event: scenario_event, retries },
        ));
        send_event(event.value);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::test_utils::common::TestWorld;

    #[tokio::test]
    async fn test_run_steps_empty_scenario() {
        let collection = step::Collection::<TestWorld>::new();
        let id = ScenarioId::new();
        let (feature, scenario) = create_test_feature_and_scenario();
        let mut world = TestWorld;
        let events = Arc::new(Mutex::new(Vec::new()));
        let events_clone = events.clone();

        let scenario_finished = StepExecutor::run_steps(
            &collection,
            id,
            feature,
            None,
            scenario,
            &mut world,
            None, // retries
            move |event| events_clone.lock().unwrap().push(event),
            #[cfg(feature = "tracing")]
            None,
        )
        .await;

        // A `Scenario` without `Step`s has nothing to fail or skip.
        assert!(matches!(scenario_finished, StepsOutcome::Passed,));
    }

    #[tokio::test]
    async fn test_run_steps_with_background_steps() {
        let collection = step::Collection::<TestWorld>::new();
        let id = ScenarioId::new();
        let (feature, scenario) = create_test_scenario_with_steps();
        let mut world = TestWorld;
        let events = Arc::new(Mutex::new(Vec::new()));
        let events_clone = events.clone();

        let scenario_finished = StepExecutor::run_steps(
            &collection,
            id,
            feature,
            None,
            scenario,
            &mut world,
            None, // retries
            move |event| events_clone.lock().unwrap().push(event),
            #[cfg(feature = "tracing")]
            None,
        )
        .await;

        // No `Step` of the `Collection` matches, so the first one is
        // skipped, and that ends the `Scenario`.
        assert!(matches!(scenario_finished, StepsOutcome::Skipped,));
    }

    fn create_test_feature_and_scenario()
    -> (Source<gherkin::Feature>, Source<gherkin::Scenario>) {
        use gherkin::{Feature, Scenario};

        let feature = Feature {
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

        let scenario = Scenario {
            keyword: "Scenario".to_string(),
            name: "Test Scenario".to_string(),
            description: None,
            steps: vec![],
            examples: vec![],
            tags: vec![],
            span: gherkin::Span { start: 0, end: 0 },
            position: gherkin::LineCol { line: 2, col: 1 },
        };

        (Source::new(feature), Source::new(scenario))
    }

    fn create_test_scenario_with_steps()
    -> (Source<gherkin::Feature>, Source<gherkin::Scenario>) {
        use gherkin::{Feature, Scenario, Step};

        let feature = Feature {
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

        let step = Step {
            ty: gherkin::StepType::Given,
            keyword: "Given".to_string(),
            value: "I have a test step".to_string(),
            docstring: None,
            table: None,
            span: gherkin::Span { start: 0, end: 0 },
            position: gherkin::LineCol { line: 3, col: 1 },
        };

        let scenario = Scenario {
            keyword: "Scenario".to_string(),
            name: "Test Scenario".to_string(),
            description: None,
            steps: vec![step],
            examples: vec![],
            tags: vec![],
            span: gherkin::Span { start: 0, end: 0 },
            position: gherkin::LineCol { line: 2, col: 1 },
        };

        (Source::new(feature), Source::new(scenario))
    }

    fn create_test_step() -> Source<gherkin::Step> {
        use gherkin::Step;

        let step = Step {
            ty: gherkin::StepType::Given,
            keyword: "Given".to_string(),
            value: "I have a test step".to_string(),
            docstring: None,
            table: None,
            span: gherkin::Span { start: 0, end: 0 },
            position: gherkin::LineCol { line: 3, col: 1 },
        };

        Source::new(step)
    }
}
