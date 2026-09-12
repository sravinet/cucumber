use std::{fs, io, panic::AssertUnwindSafe, time::Duration};

use cucumber::{World as _, WriterExt as _, given, writer, writer::Coloring};
use derive_more::with_trait::Display;
use futures::FutureExt as _;
use regex::Regex;
use tokio::{spawn, time};
use tracing_subscriber::{
    Layer,
    filter::LevelFilter,
    fmt::format::{DefaultFields, Format},
    layer::SubscriberExt as _,
};

#[tokio::main]
async fn main() {
    spawn(async {
        let mut id = 0;
        loop {
            time::sleep(Duration::from_millis(600)).await;
            tracing::info!("not in span: {id}");
            id += 1;
        }
    });

    let mut out = Vec::<u8>::new();

    let res = World::cucumber()
        .with_writer(
            writer::Basic::raw(&mut out, Coloring::Never, 0)
                .discard_stats_writes()
                .tee::<World, _>(
                    writer::Basic::raw(io::stdout(), Coloring::Never, 0)
                        .summarized(),
                )
                .normalized(),
        )
        .fail_on_skipped()
        .with_default_cli()
        .configure_and_init_tracing(
            DefaultFields::new(),
            Format::default().with_ansi(false).without_time(),
            |layer| {
                tracing_subscriber::registry()
                    .with(LevelFilter::INFO.and_then(layer))
            },
        )
        .run_and_exit("tests/features/tracing");

    AssertUnwindSafe(res).catch_unwind().await.unwrap();

    // Required to strip out non-deterministic parts of output, so we could
    // compare them well.
    let non_deterministic = Regex::new(
        " ([^\"\\n\\s]*)[/\\\\]([A-z1-9-_]*)\\.(feature|rs)(:\\d+:\\d+)?",
    )
    .unwrap();
    // The task logging outside any `Span` does so on a wall-clock interval,
    // so how many of its `Event`s are forwarded into each `gherkin::Scenario`
    // depends on how loaded the machine is. Erasing their counter and
    // collapsing repeats keeps the assertion on what is deterministic: the
    // order of the `gherkin::Scenario`/`Step` lines, and each `Step`'s log
    // being emitted inside its own `Span`, before the `Step` finishes.
    let outside_span = Regex::new("not in span: \\d+").unwrap();
    let normalize = |output: &str| {
        let mut lines = Vec::<String>::new();
        for line in output.lines() {
            let line = non_deterministic.replace_all(line, "");
            let line = outside_span.replace_all(&line, "not in span: N");
            if line.ends_with("not in span: N")
                && lines.last().is_some_and(|last| *last == line)
            {
                continue;
            }
            lines.push(line.into_owned());
        }
        lines
    };

    assert_eq!(
        normalize(String::from_utf8_lossy(&out).as_ref()),
        normalize(
            &fs::read_to_string("tests/features/tracing/correct.stdout")
                .unwrap(),
        ),
    );
}

#[given(regex = "step (\\d+)")]
async fn step(_: &mut World, n: usize) {
    time::sleep(Duration::from_secs(1)).await;
    tracing::info!("in span: {n:?}");
}

#[derive(Clone, Debug, Default, Display, cucumber::World)]
struct World;
