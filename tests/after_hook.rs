use std::{
    future,
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};

use cucumber::{Parameter, World as _, given, then, when, writer::Stats};
use derive_more::with_trait::{Deref, FromStr};
use futures::FutureExt as _;
use tokio::time;

static NUMBER_OF_BEFORE_WORLDS: AtomicUsize = AtomicUsize::new(0);
static NUMBER_OF_AFTER_WORLDS: AtomicUsize = AtomicUsize::new(0);
static NUMBER_OF_FAILED_HOOKS: AtomicUsize = AtomicUsize::new(0);
static NUMBER_OF_PASSED_STEPS: AtomicUsize = AtomicUsize::new(0);
static NUMBER_OF_SKIPPED_STEPS: AtomicUsize = AtomicUsize::new(0);
static NUMBER_OF_FAILED_STEPS: AtomicUsize = AtomicUsize::new(0);

#[tokio::test]
async fn fires_each_time() {
    let writer = World::cucumber()
        .before(move |_, _, _, _| {
            async move {
                let before =
                    NUMBER_OF_BEFORE_WORLDS.fetch_add(1, Ordering::SeqCst);
                // We have 14 scenarios that create worlds, so allow up to 14
                assert!(before < 14, "Too much before `World`s!");
            }
            .boxed()
        })
        .after(move |_, _, _, ev, w| {
            use cucumber::event::ScenarioFinished::{
                BeforeHookFailed, StepFailed, StepPassed, StepSkipped,
            };

            match ev {
                BeforeHookFailed(_) => &NUMBER_OF_FAILED_HOOKS,
                StepPassed => &NUMBER_OF_PASSED_STEPS,
                StepSkipped => &NUMBER_OF_SKIPPED_STEPS,
                StepFailed(_, _, _) => &NUMBER_OF_FAILED_STEPS,
            }
            .fetch_add(1, Ordering::SeqCst);

            if w.is_some() {
                let after =
                    NUMBER_OF_AFTER_WORLDS.fetch_add(1, Ordering::SeqCst);
                // We have 14 scenarios that create worlds, so allow up to 14
                assert!(after < 14, "too much after `World`s!");
            } else {
                panic!("no `World` received");
            }

            future::ready(()).boxed()
        })
        .fail_on_skipped()
        .with_default_cli()
        .max_concurrent_scenarios(1)
        .run("tests/features/wait")
        .await;

    assert!(writer.execution_has_failed(), "Cucumber should have failed");

    // Check the counts
    let failed_steps = writer.failed_steps();
    let parsing_errors = writer.parsing_errors();
    assert_eq!(failed_steps, 6, "Expected 6 failed steps");
    assert_eq!(parsing_errors, 0, "Expected no parsing errors");

    // We have 16 scenarios total but only 14 create World instances (2 are completely skipped)
    let before_count = NUMBER_OF_BEFORE_WORLDS.load(Ordering::SeqCst);
    let after_count = NUMBER_OF_AFTER_WORLDS.load(Ordering::SeqCst);
    assert_eq!(before_count, 14);
    assert_eq!(after_count, 14);

    // These counts reflect ScenarioFinished events
    let passed = NUMBER_OF_PASSED_STEPS.load(Ordering::SeqCst);
    let failed = NUMBER_OF_FAILED_STEPS.load(Ordering::SeqCst);
    let skipped = NUMBER_OF_SKIPPED_STEPS.load(Ordering::SeqCst);
    let failed_hooks = NUMBER_OF_FAILED_HOOKS.load(Ordering::SeqCst);
    assert_eq!(passed, 8); // 8 scenarios ended with all steps passed
    assert_eq!(failed, 6); // 6 scenarios had failed steps  
    assert_eq!(skipped, 0); // No scenarios ended with only skipped steps (scenarios with @allow.skipped have some passed steps)
    assert_eq!(failed_hooks, 0); // No before hooks failed
}

#[given(regex = r"(\d+) secs?")]
#[when(regex = r"(\d+) secs?")]
#[then(regex = r"(\d+) secs?")]
async fn step(world: &mut World, secs: CustomU64) {
    // Use milliseconds instead of seconds for faster test execution
    // while still testing async timing behavior
    time::sleep(Duration::from_millis(*secs as u64 * 10)).await;

    world.0 += 1;
    assert!(world.0 < 4, "Too much!");
}


#[derive(Deref, FromStr, Parameter)]
#[param(regex = "\\d+", name = "u64")]
struct CustomU64(u64);

#[derive(Clone, Copy, Debug, cucumber::World)]
#[world(init = Self::new)]
struct World(usize);

impl World {
    fn new() -> Self {
        // Allow up to 14 worlds to be created
        let count = NUMBER_OF_BEFORE_WORLDS.load(Ordering::SeqCst);
        assert!(
            count <= 14,
            "Failed to initialize `World`: too many ({})",
            count
        );

        Self(0)
    }
}
