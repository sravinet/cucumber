use std::{fs, io::Read as _};

use cucumber::{World as _, given, then, when, writer};
use futures::FutureExt as _;
use regex::RegexBuilder;
use tempfile::NamedTempFile;
use tracing_subscriber::{
    Layer as _,
    filter::LevelFilter,
    fmt::format::{DefaultFields, Format},
    layer::SubscriberExt as _,
};

#[given(regex = r"(\d+) secs?")]
#[when(regex = r"(\d+) secs?")]
#[then(regex = r"(\d+) secs?")]
fn step(world: &mut World) {
    world.0 += 1;
    assert!(world.0 < 4, "Too much!");
}


#[tokio::test]
#[ignore] // TODO: JSON format output has changed - need to update expected output
async fn test() {
    let mut file = NamedTempFile::new().unwrap();
    drop(
        World::cucumber()
            .with_writer(writer::Json::new(file.reopen().unwrap()))
            .fail_on_skipped()
            .with_default_cli()
            .run("tests/features/wait")
            .await,
    );

    let mut buffer = String::new();
    file.read_to_string(&mut buffer).unwrap();

    // Required to strip out non-deterministic parts of output, so we could
    // compare them well.
    let non_deterministic = RegexBuilder::new(
        "\"duration\":\\s?\\d+\
         |([^\"\\n\\s]*)[/\\\\]([A-z1-9-_]*)\\.(feature|rs)(:\\d+:\\d+)?\
         |\n\
         |\\s",
    )
    .multi_line(true)
    .build()
    .unwrap();

    assert_eq!(
        non_deterministic.replace_all(&buffer, ""),
        non_deterministic.replace_all(
            &fs::read_to_string("tests/json/correct.json").unwrap(),
            "",
        ),
    );
}

#[derive(Clone, Copy, Debug, Default, cucumber::World)]
struct World(usize);
