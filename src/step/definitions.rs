//! Enumerating step definitions without running a single scenario.
//!
//! Every `#[given]`, `#[when]` and `#[then]` function is registered with
//! [`inventory`] at link time as a pattern plus the [`Location`] of its
//! attribute. [`crate::World::collection()`] drains that registry into a
//! [`Collection`] for the runner, whose maps are private and which exposes
//! no iterator — so until now the only way to learn which definitions a
//! suite has was to run it and read the events.
//!
//! This module reads the same registry and hands the list back as data:
//! the compiled regex (a cucumber expression arrives already translated, so
//! what is exported is what [`Collection::find`] matches with) and where
//! the definition lives. That is what a step-reuse index or an impact graph
//! needs, and neither should have to execute the suite to build it.
//!
//! [`Collection`]: crate::step::Collection
//! [`Collection::find`]: crate::step::Collection::find
//! [`inventory`]: crate::codegen::inventory

use std::fmt::Write as _;

use gherkin::StepType;

use crate::{
    codegen::{LazyRegex, StepConstructor as _, WorldInventory, inventory},
    step::Location,
};

/// One registered step function: which keyword it answers, the regex it
/// matches with, and where its attribute sits.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct StepDefinition {
    /// The keyword bucket the definition was registered under.
    pub keyword: StepType,

    /// The pattern exactly as the runner compiled it.
    pub pattern: String,

    /// Where the `#[given]`/`#[when]`/`#[then]` attribute is.
    pub location: Location,
}

impl StepDefinition {
    /// `given`, `when` or `then`.
    #[must_use]
    pub const fn keyword_name(&self) -> &'static str {
        match self.keyword {
            StepType::Given => "given",
            StepType::When => "when",
            StepType::Then => "then",
        }
    }

    /// This definition as one JSON object, in the interchange shape
    /// consumed by the `screenplay-graph` loader:
    /// `{"keyword","pattern","path","line","column"}`.
    ///
    /// Hand-rendered rather than through `serde`, which is an optional
    /// dependency here: the export should not cost a feature flag.
    #[must_use]
    pub fn to_json(&self) -> String {
        format!(
            r#"{{"keyword":"{}","pattern":{},"path":{},"line":{},"column":{}}}"#,
            self.keyword_name(),
            json_string(&self.pattern),
            json_string(self.location.path),
            self.location.line,
            self.location.column,
        )
    }
}

/// Every step definition registered for `W`, sorted by location so the
/// export is stable across runs and link orders.
#[must_use]
pub fn all<W: WorldInventory>() -> Vec<StepDefinition> {
    let mut out = Vec::new();
    for given in inventory::iter::<W::Given> {
        out.push(definition(StepType::Given, given.inner()));
    }
    for when in inventory::iter::<W::When> {
        out.push(definition(StepType::When, when.inner()));
    }
    for then in inventory::iter::<W::Then> {
        out.push(definition(StepType::Then, then.inner()));
    }
    out.sort_by(|a, b| {
        (a.location, rank(a.keyword), &a.pattern).cmp(&(
            b.location,
            rank(b.keyword),
            &b.pattern,
        ))
    });
    out
}

/// The whole registry as a JSON array — the file the graph loader reads.
#[must_use]
pub fn json<W: WorldInventory>() -> String {
    let items =
        all::<W>().iter().map(StepDefinition::to_json).collect::<Vec<_>>();
    format!("[\n  {}\n]", items.join(",\n  "))
}

/// Builds a [`StepDefinition`] from one registry entry.
fn definition<W>(
    keyword: StepType,
    (location, regex, _): (Location, LazyRegex, crate::Step<W>),
) -> StepDefinition {
    StepDefinition { keyword, pattern: regex().as_str().to_owned(), location }
}

/// A total order over [`StepType`], which derives none.
const fn rank(keyword: StepType) -> u8 {
    match keyword {
        StepType::Given => 0,
        StepType::When => 1,
        StepType::Then => 2,
    }
}

/// `s` as a JSON string literal, quotes included.
fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if u32::from(c) < 0x20 => {
                // Writing to a `String` cannot fail.
                _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_escapes_what_a_regex_carries() {
        assert_eq!(json_string(r#"^a (\d+) "b"$"#), r#""^a (\\d+) \"b\"$""#);
        assert_eq!(json_string("tab\there"), r#""tab\there""#);
        assert_eq!(json_string("\u{1}"), r#""\u0001""#);
    }

    #[test]
    fn a_definition_renders_in_the_interchange_shape() {
        let def = StepDefinition {
            keyword: StepType::When,
            pattern: r"^(\S+) pays$".to_owned(),
            location: Location::new("tests/steps.rs", 12, 1),
        };
        assert_eq!(
            def.to_json(),
            r#"{"keyword":"when","pattern":"^(\\S+) pays$","path":"tests/steps.rs","line":12,"column":1}"#
        );
        assert_eq!(def.keyword_name(), "when");
    }

    #[test]
    fn keywords_rank_in_gherkin_order() {
        assert!(rank(StepType::Given) < rank(StepType::When));
        assert!(rank(StepType::When) < rank(StepType::Then));
    }
}
