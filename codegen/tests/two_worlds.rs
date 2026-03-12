use std::time::Duration;

use cucumber::{StatsWriter as _, World, gherkin::Step, given, when, then, writer};
use tokio::time;

#[derive(Debug, Default, World)]
pub struct FirstWorld {
    foo: i32,
}

#[derive(Debug, Default, World)]
pub struct SecondWorld {
    foo: i32,
}

#[given(regex = r"(\S+) is (\d+)")]
#[when(regex = r"(\S+) is (\d+)")]
async fn test_regex_async(
    w: &mut FirstWorld,
    step: String,
    #[step] ctx: &Step,
    num: usize,
) {
    time::sleep(Duration::new(1, 0)).await;

    assert_eq!(step, "foo");
    assert_eq!(num, 0);
    assert_eq!(ctx.value, "foo is 0");

    w.foo += 1;
}

#[given(regex = r"(\S+) is sync (\d+)")]
fn test_regex_sync_slice(w: &mut SecondWorld, step: &Step, matches: &[String]) {
    assert_eq!(matches[0], "foo");
    assert_eq!(matches[1].parse::<usize>().unwrap(), 0);
    assert_eq!(step.value, "foo is sync 0");

    w.foo += 1;
}

// Add step definitions for SecondWorld to handle all steps
#[given(regex = r"(\S+) is (\d+)")]
#[when(regex = r"(\S+) is (\d+)")]
async fn test_regex_async_second(
    w: &mut SecondWorld,
    step: String,
    #[step] ctx: &Step,
    num: usize,
) {
    time::sleep(Duration::new(1, 0)).await;

    assert_eq!(step, "foo");
    assert_eq!(num, 0);
    assert_eq!(ctx.value, "foo is 0");

    w.foo += 1;
}

// Step definitions for both worlds to handle file operations
#[when(regex = r#"I write "([^"]*)" to '([^']*)'"#)]
fn write_to_file(_world: &mut FirstWorld, content: String, file: String) {
    std::fs::write(&file, content).unwrap();
}

#[when(regex = r#"I write "([^"]*)" to '([^']*)'"#)]
fn write_to_file_second(_world: &mut SecondWorld, content: String, file: String) {
    std::fs::write(&file, content).unwrap();
}

#[then(regex = r#"the file '([^']*)' should contain "([^"]*)""#)]
fn file_should_contain(_world: &mut FirstWorld, file: String, expected: String) {
    let content = std::fs::read_to_string(&file);
    assert!(content.is_ok(), "File '{}' should exist", file);
    let content = content.unwrap();
    assert_eq!(content, expected);
}

#[then(regex = r#"the file '([^']*)' should contain "([^"]*)""#)]
fn file_should_contain_second(_world: &mut SecondWorld, file: String, expected: String) {
    let content = std::fs::read_to_string(&file);
    assert!(content.is_ok(), "File '{}' should exist", file);
    let content = content.unwrap();
    assert_eq!(content, expected);
}

#[then(regex = r#""([^"]*)" contains '([^']*)'"#)]
fn file_contains(_world: &mut FirstWorld, file: String, expected: String) {
    let content = std::fs::read_to_string(&file).unwrap();
    assert!(content.contains(&expected));
}

#[then(regex = r#""([^"]*)" contains '([^']*)'"#)]
fn file_contains_second(_world: &mut SecondWorld, file: String, expected: String) {
    let content = std::fs::read_to_string(&file).unwrap();
    assert!(content.contains(&expected));
}

#[given("foo is not bar")]
fn foo_is_not_bar(_world: &mut FirstWorld) {
    // This step is meant to be different from the others
}

#[given("foo is not bar")]
fn foo_is_not_bar_second(_world: &mut SecondWorld) {
    // This step is meant to be different from the others
}

#[tokio::main]
async fn main() {
    let writer = FirstWorld::cucumber()
        .max_concurrent_scenarios(None)
        .with_writer(writer::Libtest::or_basic())
        .run("./tests/features")
        .await;

    assert_eq!(writer.passed_steps(), 13);
    assert_eq!(writer.skipped_steps(), 0);
    assert_eq!(writer.failed_steps(), 1);

    let writer = SecondWorld::cucumber()
        .max_concurrent_scenarios(None)
        .with_writer(writer::Libtest::or_basic())
        .run("./tests/features")
        .await;

    assert_eq!(writer.passed_steps(), 14);
    assert_eq!(writer.skipped_steps(), 0);
    assert_eq!(writer.failed_steps(), 1);
}
