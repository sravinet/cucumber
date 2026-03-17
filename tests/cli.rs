use clap::Parser;
use cucumber::{World as _, cli, given, writer::Stats};

#[derive(cli::Args)]
struct CustomCli {
    #[command(subcommand)]
    command: Option<SubCommand>,
}

#[derive(clap::Subcommand)]
enum SubCommand {
    Smoke(Smoke),
}

#[derive(cli::Args)]
struct Smoke {
    #[arg(long)]
    report_name: String,
}

#[derive(Clone, Copy, Debug, Default, cucumber::World)]
struct World;

#[given("an invalid step")]
fn invalid_step(_world: &mut World) {
    assert!(false);
}

// This test uses a subcommand with the global option `--tags` to filter on two
// failing tests and verifies that the error output contains 2 failing steps.
#[tokio::test]
async fn tags_option_filters_all_scenarios_with_subcommand() {
    let cli = cli::Opts::<_, _, _, CustomCli>::try_parse_from(&[
        "test",
        "smoke",
        r#"--report-name="smoke.report""#,
        "--tags=@all",
    ])
    .expect("Invalid command line");

    let writer = World::cucumber().with_cli(cli).run("tests/features/cli").await;

    assert!(writer.execution_has_failed(), "Cucumber should have failed");
    assert_eq!(writer.failed_steps(), 2, "Expected 2 failed steps");
}

// This test uses a subcommand with the global option `--tags` to filter on one
// failing test and verifies that the error output contains 1 failing step.
#[tokio::test]
async fn tags_option_filters_scenario1_with_subcommand() {
    let cli = cli::Opts::<_, _, _, CustomCli>::try_parse_from(&[
        "test",
        "smoke",
        r#"--report-name="smoke.report""#,
        "--tags=@scenario-1",
    ])
    .expect("Invalid command line");

    let writer = World::cucumber().with_cli(cli).run("tests/features/cli").await;

    assert!(writer.execution_has_failed(), "Cucumber should have failed");
    assert_eq!(writer.failed_steps(), 1, "Expected 1 failed step");
}

// This test verifies that the global option `--tags` is still available without
// subcommands and that the error output contains 1 failing step.
#[tokio::test]
async fn tags_option_filters_scenario1_no_subcommand() {
    let cli = cli::Opts::<_, _, _, CustomCli>::try_parse_from(&[
        "test",
        "--tags=@scenario-1",
    ])
    .expect("Invalid command line");

    let writer = World::cucumber().with_cli(cli).run("tests/features/cli").await;

    assert!(writer.execution_has_failed(), "Cucumber should have failed");
    assert_eq!(writer.failed_steps(), 1, "Expected 1 failed step");
}
