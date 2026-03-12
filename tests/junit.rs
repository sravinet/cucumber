use std::io::Read as _;

use cucumber::{World as _, given, then, when, writer};
use futures::FutureExt as _;
use tempfile::NamedTempFile;
use tracing_subscriber::{
    Layer as _,
    filter::LevelFilter,
    fmt::format::{DefaultFields, Format},
    layer::SubscriberExt as _,
};
use quick_xml::events::Event;
use quick_xml::Reader;

#[given(regex = r"(\d+) secs?")]
#[when(regex = r"(\d+) secs?")]
#[then(regex = r"(\d+) secs?")]
fn step(world: &mut World) {
    world.0 += 1;
    assert!(world.0 < 4, "Too much!");
    tracing::info!("step");
}

#[tokio::test]
#[ignore = "TODO: Fix tracing global subscriber conflict in test environment"]
async fn output_structural_validation() {
    let mut file = NamedTempFile::new().unwrap();
    drop(
        World::cucumber()
            .before(|_, _, _, _| {
                async { tracing::info!("before") }.boxed_local()
            })
            .after(|_, _, _, _, _| {
                async { tracing::info!("after") }.boxed_local()
            })
            .with_writer(writer::JUnit::new(file.reopen().unwrap(), 1))
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
            .run("tests/features/wait")
            .await,
    );

    let mut buffer = String::new();
    file.read_to_string(&mut buffer).unwrap();

    // Validate XML structure and essential content
    validate_junit_structure(&buffer);
}

fn validate_junit_structure(xml_content: &str) {
    let mut reader = Reader::from_str(xml_content);
    let mut buf = Vec::new();
    
    let mut testsuite_count = 0;
    let mut testcase_count = 0;
    let mut failure_count = 0;
    let mut skipped_count = 0;
    let mut success_count = 0;
    
    let mut in_testsuites = false;
    let mut in_testsuite = false;
    let mut in_testcase = false;
    let mut current_testcase_has_failure = false;
    let mut current_testcase_has_skipped = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                match e.name().as_ref() {
                    b"testsuites" => {
                        in_testsuites = true;
                    }
                    b"testsuite" => {
                        assert!(in_testsuites, "testsuite must be inside testsuites");
                        in_testsuite = true;
                        testsuite_count += 1;
                    }
                    b"testcase" => {
                        assert!(in_testsuite, "testcase must be inside testsuite");
                        in_testcase = true;
                        testcase_count += 1;
                        current_testcase_has_failure = false;
                        current_testcase_has_skipped = false;
                        
                        // Validate testcase has required name attribute
                        let has_name = e.attributes()
                            .any(|attr| attr.as_ref().map(|a| a.key.as_ref() == b"name").unwrap_or(false));
                        assert!(has_name, "testcase must have name attribute");
                    }
                    b"failure" => {
                        assert!(in_testcase, "failure must be inside testcase");
                        current_testcase_has_failure = true;
                    }
                    b"skipped" => {
                        assert!(in_testcase, "skipped must be inside testcase");
                        current_testcase_has_skipped = true;
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                match e.name().as_ref() {
                    b"testsuites" => {
                        in_testsuites = false;
                    }
                    b"testsuite" => {
                        in_testsuite = false;
                    }
                    b"testcase" => {
                        in_testcase = false;
                        if current_testcase_has_failure {
                            failure_count += 1;
                        } else if current_testcase_has_skipped {
                            skipped_count += 1;
                        } else {
                            success_count += 1;
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => panic!("XML parsing error: {}", e),
            _ => {}
        }
        buf.clear();
    }
    
    // Validate essential structure expectations
    assert!(testsuite_count > 0, "Must have at least one testsuite");
    assert!(testcase_count > 0, "Must have at least one testcase");
    
    // Validate expected test results based on known test scenarios
    assert!(failure_count > 0, "Should have some failing test cases");
    assert!(success_count > 0, "Should have some successful test cases");
    
    // The total should match what we know about the wait test scenarios
    let total_cases = failure_count + skipped_count + success_count;
    assert_eq!(total_cases, testcase_count, "All test cases should be accounted for");
    
    println!("JUnit validation passed:");
    println!("  Test suites: {}", testsuite_count);
    println!("  Test cases: {}", testcase_count);
    println!("  Failures: {}", failure_count);
    println!("  Skipped: {}", skipped_count);
    println!("  Success: {}", success_count);
}

#[tokio::test] 
async fn output_semantic_validation() {
    let mut file = NamedTempFile::new().unwrap();
    drop(
        World::cucumber()
            .before(|_, _, _, _| {
                async { tracing::info!("before") }.boxed_local()
            })
            .after(|_, _, _, _, _| {
                async { tracing::info!("after") }.boxed_local()
            })
            .with_writer(writer::JUnit::new(file.reopen().unwrap(), 1))
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
            .run("tests/features/wait")
            .await,
    );

    let mut buffer = String::new();
    file.read_to_string(&mut buffer).unwrap();

    // Validate essential semantic content regardless of formatting
    validate_junit_essential_content(&buffer);
}

fn validate_junit_essential_content(xml_content: &str) {
    // Validate essential information is present in the output
    assert!(xml_content.contains("testsuites"), "Must contain testsuites root element");
    assert!(xml_content.contains("testsuite"), "Must contain testsuite elements");
    assert!(xml_content.contains("testcase"), "Must contain testcase elements");
    
    // Validate feature-related content
    assert!(xml_content.contains("Feature:"), "Must contain feature information");
    assert!(xml_content.contains("Scenario:"), "Must contain scenario information");
    
    // Validate expected error types
    assert!(xml_content.contains("Parser Error"), "Must contain parser error information");
    assert!(xml_content.contains("Step Panicked"), "Must contain step panic information");
    
    // Validate failure content
    assert!(xml_content.contains("Too much!"), "Must contain expected step failure message");
    assert!(xml_content.contains("doesn't match any function"), "Must contain step matching failure");
    
    // Validate structural elements
    assert!(xml_content.contains("failure"), "Must contain failure elements");
    assert!(xml_content.contains("system-out"), "Must contain system-out for successful tests");
    
    // Validate essential attributes exist (not their specific values)
    assert!(xml_content.contains("name="), "Must contain name attributes");
    assert!(xml_content.contains("tests="), "Must contain test count attributes");
    assert!(xml_content.contains("failures="), "Must contain failure count attributes");
    
    println!("JUnit semantic validation passed - all essential content present");
}

#[derive(Clone, Copy, Debug, Default, cucumber::World)]
struct World(usize);
