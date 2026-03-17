use std::time::Duration;

use cucumber::{Parameter, World as _, cli, given, then, when, writer, writer::Stats};
use derive_more::with_trait::{Deref, FromStr};
use futures::FutureExt as _;
use tokio::time;

#[derive(cli::Args)]
struct CustomCli {
    /// Additional time to wait in before and after hooks.
    #[arg(
        long,
        default_value = "10ms",
        value_parser = humantime::parse_duration,
    )]
    pause: Duration,
}

#[tokio::main]
async fn main() {
    let cli = cli::Opts::<_, _, _, CustomCli>::parsed();

    let writer = World::cucumber()
        .before(move |_, _, _, w| {
            async move {
                w.0 = 0;
                time::sleep(cli.custom.pause).await;
            }
            .boxed_local()
        })
        .after(move |_, _, _, _, _| time::sleep(cli.custom.pause).boxed_local())
        .with_writer(writer::Libtest::or_basic())
        .fail_on_skipped()
        .with_cli(cli)
        .run("tests/features/wait")
        .await;

    assert!(writer.execution_has_failed(), "Cucumber should have failed");
    assert_eq!(writer.failed_steps(), 10, "Expected 10 failed steps");
    assert_eq!(writer.parsing_errors(), 0, "Expected no parsing errors");
}

#[given(regex = r"(\d+) secs?")]
#[when(regex = r"(\d+) secs?")]
async fn step(world: &mut World, secs: CustomU64) {
    time::sleep(Duration::from_secs(*secs)).await;

    world.0 += 1;
    assert!(world.0 < 4, "Too much!");
}

#[given(expr = "{int} sec")]
#[when(expr = "{int} sec")]
async fn step_singular_gw(world: &mut World, secs: usize) {
    time::sleep(Duration::from_secs(secs as u64)).await;

    world.0 += 1;
    assert!(world.0 < 4, "Too much!");
}

#[then(regex = r"^(\d+) secs?$")]
async fn then_step(world: &mut World, secs: CustomU64) {
    time::sleep(Duration::from_secs(*secs)).await;

    world.0 += 1;
    assert!(world.0 < 4, "Too much!");
}

#[then("unknown")]
async fn unknown(_world: &mut World) {
    // This step is meant to cause failures
    panic!("Unknown step executed");
}

#[then(expr = "{int} sec")]
async fn then_step_singular(world: &mut World, secs: usize) {
    time::sleep(Duration::from_secs(secs as u64)).await;

    world.0 += 1;
    assert!(world.0 < 4, "Too much!");
}


#[derive(Deref, FromStr, Parameter)]
#[param(regex = r"\d+", name = "u64")]
struct CustomU64(u64);

#[derive(Clone, Copy, Debug, Default, cucumber::World)]
struct World(usize);
