//! The step registry can be read as data, without running anything.

use cucumber::{World, given, step::definitions, then, when};
use gherkin::StepType;

#[derive(Debug, Default, World)]
struct Exported;

#[given(regex = r"^a (\d+)$")]
async fn a(_: &mut Exported, _n: u32) {}

#[when(expr = "b {word}")]
async fn b(_: &mut Exported, _s: String) {}

#[then("c")]
async fn c(_: &mut Exported) {}

#[test]
fn every_definition_is_exported_with_its_pattern_and_location() {
    let defs = definitions::all::<Exported>();
    assert_eq!(defs.len(), 3, "{defs:?}");

    let keywords: Vec<StepType> = defs.iter().map(|d| d.keyword).collect();
    assert_eq!(
        keywords,
        vec![StepType::Given, StepType::When, StepType::Then],
        "sorted by location, which here is declaration order"
    );
    for def in &defs {
        assert!(
            def.location.path.ends_with("step_definitions_export.rs"),
            "{}",
            def.location
        );
        assert!(def.location.line > 0);
    }
    assert!(defs[0].location.line < defs[1].location.line);
    assert!(defs[1].location.line < defs[2].location.line);

    assert_eq!(defs[0].pattern, r"^a (\d+)$", "a regex is exported verbatim");
    let compiled = regex::Regex::new(&defs[1].pattern)
        .expect("the expression compiled to a regex");
    assert!(compiled.is_match("b foo"), "{}", defs[1].pattern);
    assert!(
        !compiled.is_match("b foo bar"),
        "`{{word}}` takes one word: {}",
        defs[1].pattern
    );
    let literal =
        regex::Regex::new(&defs[2].pattern).expect("a literal compiles");
    assert!(literal.is_match("c"));
}

#[test]
fn the_json_export_is_one_array_of_interchange_objects() {
    let json = definitions::json::<Exported>();
    assert!(json.starts_with("[\n  {"), "{json}");
    assert!(json.trim_end().ends_with("}\n]"), "{json}");
    assert_eq!(json.matches("\"keyword\":").count(), 3);
    assert!(
        json.contains(r#""keyword":"given","pattern":"^a (\\d+)$""#),
        "{json}"
    );
    assert!(json.contains(r#""path":""#));
    assert!(json.contains(r#""line":"#));
}
