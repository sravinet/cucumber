// Copyright (c) 2018-2025  Brendan Molloy <brendan@bbqsrc.net>,
//                          Ilya Solovyiov <ilya.solovyiov@gmail.com>,
//                          Kai Ren <tyranron@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Tools for executing [`crate::step::Step`]s.
//!
//! [Gherkin]: https://cucumber.io/docs/gherkin/reference/

pub mod basic;

use futures::Stream;

#[doc(inline)]
pub use self::basic::{Basic, ScenarioType};
use crate::{Event, event, parser};
#[cfg(doc)]
use crate::{Step, event::Source};

/// Executor of [`Parser`] output producing [`Cucumber`] events for [`crate::Writer`].
///
/// # Order guarantees
///
/// Implementors are expected to source events in a [happened-before] order. For
/// example [`event::Scenario::Started`] for a single [`gherkin::Scenario`] should
/// predate any other events of this [`gherkin::Scenario`], while
/// [`event::Scenario::Finished`] should be the last one. [`crate::step::Step`] events of
/// this [`gherkin::Scenario`] should be emitted in order of declaration in `.feature`
/// file. But as [`gherkin::Scenario`]s can be executed concurrently, events from one
/// [`gherkin::Scenario`] can be interrupted by events of a different one (which are also
/// following the [happened-before] order). Those rules are applied also to
/// [`Rule`]s and [`Feature`]s. If you want to avoid those interruptions for
/// some [`gherkin::Scenario`], it should be resolved as [`ScenarioType::Serial`] by the
/// [`crate::runner::Runner`].
///
/// Because of that, [`crate::Writer`], accepting events produced by a [`crate::runner::Runner`] has
/// to be [`Normalized`].
///
/// All those rules are considered in a [`Basic`] reference [`crate::runner::Runner`]
/// implementation.
///
/// # Identity guarantees
///
/// Implementations are responsible for the returned [`event`]s to contain
/// [`Source`]d [`Feature`]s, [`Rule`]s, [`gherkin::Scenario`]s and [`crate::step::Step`]s uniquely
/// identifying the ones from the received [`Feature`]s, without duplicates
/// (i.e. the same [`gherkin::Scenario`] pointed by two different [`Source`]s), since a
/// [`Source`] is solely identified by its pointer, which is considered by
/// [`crate::runner::Runner`]s operating on them.
///
/// This rule is considered in a [`Basic`] reference [`crate::runner::Runner`] implementation.
///
/// [`Cucumber`]: event::Cucumber
/// [`Feature`]: gherkin::Feature
/// [`Normalized`]: crate::writer::Normalized
/// [`Parser`]: crate::Parser
/// [`Rule`]: gherkin::Rule
/// [`gherkin::Scenario`]: gherkin::Scenario
/// [`crate::Writer`]: crate::Writer
///
/// [happened-before]: https://en.wikipedia.org/wiki/Happened-before
pub trait Runner<World> {
    /// CLI options of this [`crate::runner::Runner`]. In case no options should be introduced,
    /// just use [`cli::Empty`].
    ///
    /// All CLI options from [`Parser`], [`crate::runner::Runner`] and [`crate::Writer`] will be
    /// merged together, so overlapping arguments will cause a runtime panic.
    ///
    /// [`cli::Empty`]: crate::cli::Empty
    /// [`Parser`]: crate::Parser
    /// [`crate::Writer`]: crate::Writer
    type Cli: clap::Args;

    /// Output events [`Stream`].
    type EventStream: Stream<
        Item = parser::Result<Event<event::Cucumber<World>>>,
    >;

    /// Executes the given [`Stream`] of [`Feature`]s transforming it into
    /// a [`Stream`] of executed [`Cucumber`] events.
    ///
    /// [`Cucumber`]: event::Cucumber
    /// [`Feature`]: gherkin::Feature
    fn run<S>(self, features: S, cli: Self::Cli) -> Self::EventStream
    where
        S: Stream<Item = parser::Result<gherkin::Feature>> + 'static;
}
