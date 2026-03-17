//! Step-level events and errors.

use std::sync::Arc;

use derive_more::with_trait::{Display, Error, From};

use super::event_struct::Info;
use crate::{step, writer::basic::coerce_error};

/// Event specific to a particular [Step].
///
/// [Step]: https://cucumber.io/docs/gherkin/reference#step
#[derive(Debug)]
pub enum Step<World> {
    /// [`crate::step::Step`] execution being started.
    ///
    /// [`crate::step::Step`]: gherkin::Step
    Started,

    /// [`crate::step::Step`] being skipped.
    ///
    /// That means there is no [`regex::Regex`] matching [`crate::step::Step`] in a
    /// [`step::Collection`].
    ///
    /// [`regex::Regex`]: regex::Regex
    /// [`crate::step::Step`]: gherkin::Step
    /// [`step::Collection`]: crate::step::Collection
    Skipped,

    /// [`crate::step::Step`] passed.
    ///
    /// [`crate::step::Step`]: gherkin::Step
    Passed {
        /// [`regex::Regex`] [`CaptureLocations`] of the matched [`crate::step::Step`].
        ///
        /// [`CaptureLocations`]: regex::CaptureLocations
        /// [`regex::Regex`]: regex::Regex
        /// [`crate::step::Step`]: gherkin::Step
        captures: regex::CaptureLocations,

        /// [`Location`] of the [`fn`] that matched this [`crate::step::Step`].
        ///
        /// [`Location`]: step::Location
        /// [`crate::step::Step`]: gherkin::Step
        location: Option<step::Location>,
    },

    /// [`crate::step::Step`] failed.
    ///
    /// [`crate::step::Step`]: gherkin::Step
    Failed {
        /// [`regex::Regex`] [`CaptureLocations`] of the matched [`crate::step::Step`] (if any).
        ///
        /// [`CaptureLocations`]: regex::CaptureLocations
        /// [`regex::Regex`]: regex::Regex
        /// [`crate::step::Step`]: gherkin::Step
        captures: Option<regex::CaptureLocations>,

        /// [`Location`] of the [`fn`] that matched this [`crate::step::Step`] (if any).
        ///
        /// [`Location`]: step::Location
        /// [`crate::step::Step`]: gherkin::Step
        location: Option<step::Location>,

        /// [`crate::World`] at the time [`crate::step::Step`] has failed (if any).
        ///
        /// [`crate::step::Step`]: gherkin::Step
        world: Option<Arc<World>>,

        /// Error that caused the [`crate::step::Step`] to fail.
        ///
        /// [`crate::step::Step`]: gherkin::Step
        error: StepError,
    },
}

// Manual implementation is required to omit the redundant `World: Clone` trait
// bound imposed by `#[derive(Clone)]`.
impl<World> Clone for Step<World> {
    fn clone(&self) -> Self {
        match self {
            Self::Started => Self::Started,
            Self::Skipped => Self::Skipped,
            Self::Passed { captures, location } => {
                Self::Passed { captures: captures.clone(), location: *location }
            }
            Self::Failed { captures, location, world, error } => Self::Failed {
                captures: captures.clone(),
                location: *location,
                world: world.clone(),
                error: error.clone(),
            },
        }
    }
}

/// Error of executing a [`crate::step::Step`].
///
/// [`crate::step::Step`]: gherkin::Step
#[derive(Clone, Debug, Display, Error, From)]
pub enum StepError {
    /// [`crate::step::Step`] doesn't match any [`regex::Regex`].
    ///
    /// It's emitted whenever a [`Step::Skipped`] event cannot be tolerated
    /// (such as when [`fail_on_skipped()`] is used).
    ///
    /// [`regex::Regex`]: regex::Regex
    /// [`fail_on_skipped()`]: crate::WriterExt::fail_on_skipped()
    #[display("Step doesn't match any function")]
    NotFound,

    /// [`crate::step::Step`] matches multiple [`regex::Regex`]es.
    ///
    /// [`regex::Regex`]: regex::Regex
    /// [`crate::step::Step`]: gherkin::Step
    #[display("Step match is ambiguous: {_0}")]
    AmbiguousMatch(step::AmbiguousMatchError),

    /// [`crate::step::Step`] panicked.
    ///
    /// [`crate::step::Step`]: gherkin::Step
    #[display("Step panicked. Captured output: {}", coerce_error(_0))]
    Panic(#[error(not(source))] Info),
}
