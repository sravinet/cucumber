//! Step execution logic for the Basic executor.

use std::panic::AssertUnwindSafe;

use futures::FutureExt as _;

use super::super::supporting_structures::{ScenarioId, coerce_into_info};
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
    ) -> event::ScenarioFinished
    where
        W: World,
    {
        let mut skipped_steps = 0;
        let mut step_failed = false;
        // A `Step` matching no function aborts the `Scenario`, exactly like a
        // failing one does, so the `Step`s after it are never run.
        let mut step_skipped = false;
        let mut last_failure: Option<(
            Option<regex::CaptureLocations>,
            Option<step::Location>,
            event::StepError,
        )> = None;

        // Collect all steps to execute (background steps + scenario steps)
        let mut all_steps = Vec::new();

        // 1. Add feature-level background steps (if any)
        if let Some(background) = &feature.background {
            for step in &background.steps {
                all_steps.push((step.clone(), true)); // true = background step
            }
        }

        // 2. Add rule-level background steps (if any)
        if let Some(rule) = &rule {
            if let Some(background) = &rule.background {
                for step in &background.steps {
                    all_steps.push((step.clone(), true)); // true = background step
                }
            }
        }

        // 3. Add scenario steps
        for step in &scenario.steps {
            all_steps.push((step.clone(), false)); // false = regular step
        }

        // Execute all steps. A `Step` that fails or is skipped ends its
        // `Scenario`: the `Step`s after it are neither run nor reported,
        // so a `Scenario` yields exactly one terminal `Step` event.
        for (step, is_background) in all_steps {
            let step_result = if is_background {
                Self::run_background_step(
                    collection,
                    id,
                    feature.clone(),
                    rule.clone(),
                    scenario.clone(),
                    Source::new(step.clone()),
                    world,
                    retries,
                    send_event.clone(),
                    #[cfg(feature = "tracing")]
                    waiter,
                )
                .await
            } else {
                Self::run_step(
                    collection,
                    id,
                    feature.clone(),
                    rule.clone(),
                    scenario.clone(),
                    Source::new(step.clone()),
                    world,
                    retries,
                    send_event.clone(),
                    #[cfg(feature = "tracing")]
                    waiter,
                )
                .await
            };

            match step_result {
                event::Step::Started => {
                    // This shouldn't happen as run_step returns the final result
                    // But we need to handle it for exhaustive matching
                }
                event::Step::Passed { .. } => {}
                event::Step::Skipped => {
                    skipped_steps += 1;
                    step_skipped = true;
                }
                event::Step::Failed { captures, location, error, .. } => {
                    step_failed = true;
                    last_failure =
                        Some((captures.clone(), location, error.clone()));
                }
            }

            if step_skipped || step_failed {
                break;
            }
        }

        // Determine the scenario outcome based on canonical Cucumber behavior:
        // 1. If any step failed -> StepFailed
        // 2. If any step was skipped (but none failed) -> StepSkipped
        // 3. If all steps passed -> StepPassed
        if let Some((captures, location, error)) = last_failure {
            event::ScenarioFinished::StepFailed(captures, location, error)
        } else if skipped_steps > 0 {
            event::ScenarioFinished::StepSkipped
        } else {
            event::ScenarioFinished::StepPassed
        }
    }

    /// Runs a single step.
    async fn run_step<W>(
        collection: &step::Collection<W>,
        _id: ScenarioId,
        feature: Source<gherkin::Feature>,
        rule: Option<Source<gherkin::Rule>>,
        scenario: Source<gherkin::Scenario>,
        step: Source<gherkin::Step>,
        world: &mut W,
        retries: Option<crate::event::Retries>,
        send_event: impl Fn(event::Cucumber<W>),
        #[cfg(feature = "tracing")] waiter: Option<
            &crate::tracing::SpanCloseWaiter,
        >,
    ) -> event::Step<W>
    where
        W: World,
    {
        let event = Event::new(event::Cucumber::scenario(
            feature.clone(),
            rule.clone(),
            scenario.clone(),
            event::RetryableScenario {
                event: event::Scenario::Step(
                    step.clone(),
                    event::Step::Started,
                ),
                retries,
            },
        ));
        send_event(event.value);

        #[cfg(feature = "tracing")]
        let span = _id.step_span(false);
        #[cfg(feature = "tracing")]
        let span_id = span.id();

        let step_fn = collection.find(&*step);
        let (result, location, step_captures) = match step_fn {
            Ok(Some((step_fn, captures, loc, ctx))) => {
                // Extract the actual capture locations for the event
                let actual_captures = captures.clone();

                let run = AssertUnwindSafe(step_fn(world, ctx)).catch_unwind();
                // Instrumenting the future, rather than entering the `Span`,
                // is what keeps concurrently running `Scenario`s out of it:
                // an entered guard stays on the thread across `.await`s, so
                // every `Span` opened meanwhile would nest inside this one.
                #[cfg(feature = "tracing")]
                let run = tracing::Instrument::instrument(run, span);
                let result = run.await;

                (result, loc, Some(actual_captures))
            }
            Ok(None) => {
                // A `Step` matching no function is skipped, not failed.
                // `WriterExt::fail_on_skipped()` is what turns this into a
                // `StepError::NotFound` failure, when that's wanted.
                let step_event = event::Step::Skipped;
                Self::emit_finished_step_event(
                    feature,
                    rule,
                    scenario,
                    step,
                    step_event.clone(),
                    retries,
                    false,
                    send_event,
                );
                return step_event;
            }
            Err(ambiguous_err) => {
                let step_event = event::Step::Failed {
                    captures: None,
                    location: None,
                    world: None,
                    error: event::StepError::AmbiguousMatch(ambiguous_err),
                };
                Self::emit_finished_step_event(
                    feature,
                    rule,
                    scenario,
                    step,
                    step_event.clone(),
                    retries,
                    false,
                    send_event,
                );
                return step_event;
            }
        };

        #[cfg(feature = "tracing")]
        if let Some((waiter, span_id)) = waiter.zip(span_id) {
            waiter.wait_for_span_close(span_id).await;
        }

        let step_event = match result {
            Ok(()) => event::Step::Passed {
                captures: step_captures.unwrap_or_else(|| {
                    regex::Regex::new("").unwrap().capture_locations()
                }),
                location,
            },
            Err(err) => {
                let info = coerce_into_info(err);
                event::Step::Failed {
                    captures: step_captures,
                    location,
                    world: None,
                    error: event::StepError::Panic(info),
                }
            }
        };

        Self::emit_finished_step_event(
            feature,
            rule,
            scenario,
            step,
            step_event.clone(),
            retries,
            false,
            send_event,
        );

        step_event
    }

    /// Runs a single background step.
    async fn run_background_step<W>(
        collection: &step::Collection<W>,
        _id: ScenarioId,
        feature: Source<gherkin::Feature>,
        rule: Option<Source<gherkin::Rule>>,
        scenario: Source<gherkin::Scenario>,
        step: Source<gherkin::Step>,
        world: &mut W,
        retries: Option<crate::event::Retries>,
        send_event: impl Fn(event::Cucumber<W>),
        #[cfg(feature = "tracing")] waiter: Option<
            &crate::tracing::SpanCloseWaiter,
        >,
    ) -> event::Step<W>
    where
        W: World,
    {
        // Send background step started event
        let event = Event::new(event::Cucumber::scenario(
            feature.clone(),
            rule.clone(),
            scenario.clone(),
            event::RetryableScenario {
                event: event::Scenario::Background(
                    step.clone(),
                    event::Step::Started,
                ),
                retries,
            },
        ));
        send_event(event.value);

        #[cfg(feature = "tracing")]
        let span = _id.step_span(true); // true for background
        #[cfg(feature = "tracing")]
        let span_id = span.id();

        // Run the actual step (same logic as run_step)
        let step_fn = collection.find(&*step);
        let (result, location, step_captures) = match step_fn {
            Ok(Some((step_fn, captures, loc, ctx))) => {
                // Extract the actual capture locations for the event
                let actual_captures = captures.clone();

                let run = AssertUnwindSafe(step_fn(world, ctx)).catch_unwind();
                #[cfg(feature = "tracing")]
                let run = tracing::Instrument::instrument(run, span);
                let result = run.await;

                (result, loc, Some(actual_captures))
            }
            Ok(None) => {
                // A `Step` matching no function is skipped, not failed.
                // `WriterExt::fail_on_skipped()` is what turns this into a
                // `StepError::NotFound` failure, when that's wanted.
                let step_event = event::Step::Skipped;
                Self::emit_finished_step_event(
                    feature,
                    rule,
                    scenario,
                    step,
                    step_event.clone(),
                    retries,
                    true,
                    send_event,
                );
                return step_event;
            }
            Err(ambiguous_err) => {
                let step_event = event::Step::Failed {
                    captures: None,
                    location: None,
                    world: None,
                    error: event::StepError::AmbiguousMatch(ambiguous_err),
                };
                Self::emit_finished_step_event(
                    feature,
                    rule,
                    scenario,
                    step,
                    step_event.clone(),
                    retries,
                    true,
                    send_event,
                );
                return step_event;
            }
        };

        #[cfg(feature = "tracing")]
        if let Some((waiter, span_id)) = waiter.zip(span_id) {
            waiter.wait_for_span_close(span_id).await;
        }

        let step_event = match result {
            Ok(()) => event::Step::Passed {
                captures: step_captures.unwrap_or_else(|| {
                    regex::Regex::new("").unwrap().capture_locations()
                }),
                location,
            },
            Err(err) => {
                let info = coerce_into_info(err);
                event::Step::Failed {
                    captures: step_captures,
                    location,
                    world: None,
                    error: event::StepError::Panic(info),
                }
            }
        };

        Self::emit_finished_step_event(
            feature,
            rule,
            scenario,
            step,
            step_event.clone(),
            retries,
            true,
            send_event,
        );

        step_event
    }

    /// Emits the finished event for either a regular or background step.
    fn emit_finished_step_event<W>(
        feature: Source<gherkin::Feature>,
        rule: Option<Source<gherkin::Rule>>,
        scenario: Source<gherkin::Scenario>,
        step: Source<gherkin::Step>,
        step_event: event::Step<W>,
        retries: Option<crate::event::Retries>,
        is_background: bool,
        send_event: impl Fn(event::Cucumber<W>),
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
    use crate::{event, test_utils::common::TestWorld};

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
        assert!(matches!(
            scenario_finished,
            event::ScenarioFinished::StepPassed,
        ));
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
        assert!(matches!(
            scenario_finished,
            event::ScenarioFinished::StepSkipped,
        ));
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
