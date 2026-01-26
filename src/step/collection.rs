//! Step collection management and matching functionality.
//!
//! This module provides the [`Collection`] struct for storing and matching
//! step definitions with their corresponding regex patterns.

use std::{collections::HashMap, iter};

use derive_more::with_trait::Debug;
use futures::future::LocalBoxFuture;
use gherkin::StepType;
use itertools::Itertools as _;
use regex::Regex;

use super::{
    context::Context, error::AmbiguousMatchError, location::Location,
    regex::HashableRegex,
};

/// Alias for a [`gherkin::Step`] function that returns a [`LocalBoxFuture`].
pub type Step<World> =
    for<'a> fn(&'a mut World, Context) -> LocalBoxFuture<'a, ()>;

/// Alias for a [`Step`] with [`regex::CaptureLocations`], [`Location`] and
/// [`Context`] returned by [`Collection::find()`].
pub type WithContext<'me, World> =
    (&'me Step<World>, regex::CaptureLocations, Option<Location>, Context);

/// Collection of [`Step`]s.
///
/// Every [`Step`] has to match with exactly 1 [`Regex`].
#[derive(Debug)]
pub struct Collection<World> {
    /// Collection of [Given] [`Step`]s.
    ///
    /// [Given]: https://cucumber.io/docs/gherkin/reference#given
    #[debug("{:?}",
        given.iter()
            .map(|(re, step)| (re, format!("{step:p}")))
            .collect::<HashMap<_, _>>(),
    )]
    given: HashMap<(HashableRegex, Option<Location>), Step<World>>,

    /// Collection of [When] [`Step`]s.
    ///
    /// [When]: https://cucumber.io/docs/gherkin/reference#when
    #[debug("{:?}",
        when.iter()
            .map(|(re, step)| (re, format!("{step:p}")))
            .collect::<HashMap<_, _>>(),
    )]
    when: HashMap<(HashableRegex, Option<Location>), Step<World>>,

    /// Collection of [Then] [`Step`]s.
    ///
    /// [Then]: https://cucumber.io/docs/gherkin/reference#then
    #[debug("{:?}",
        then.iter()
            .map(|(re, step)| (re, format!("{step:p}")))
            .collect::<HashMap<_, _>>(),
    )]
    then: HashMap<(HashableRegex, Option<Location>), Step<World>>,
}

// Implemented manually to omit redundant `World: Clone` trait bound, imposed by
// `#[derive(Clone)]`.
impl<World> Clone for Collection<World> {
    fn clone(&self) -> Self {
        Self {
            given: self.given.clone(),
            when: self.when.clone(),
            then: self.then.clone(),
        }
    }
}

// Implemented manually to omit redundant `World: Default` trait bound, imposed
// by `#[derive(Default)]`.
impl<World> Default for Collection<World> {
    fn default() -> Self {
        Self {
            given: HashMap::new(),
            when: HashMap::new(),
            then: HashMap::new(),
        }
    }
}

impl<World> Collection<World> {
    /// Creates a new empty [`Collection`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Merges another [`Collection`] into this one, enabling modular composition.
    /// 
    /// This method allows combining step definitions from multiple collections,
    /// which is essential for enterprise-scale BDD testing where different teams
    /// own different domain-specific step definitions.
    /// 
    /// # Example
    /// 
    /// ```rust
    /// # use cucumber::step::Collection;
    /// # use regex::Regex;
    /// # use futures::future::LocalBoxFuture;
    /// # 
    /// # #[derive(Default)]
    /// # struct TestWorld;
    /// # 
    /// # fn test_step(_world: &mut TestWorld, _ctx: cucumber::step::Context) -> LocalBoxFuture<'_, ()> {
    /// #     Box::pin(async {})
    /// # }
    /// 
    /// // Create domain-specific collections
    /// let auth_steps = Collection::new()
    ///     .given(None, Regex::new(r"user is logged in").unwrap(), test_step);
    ///     
    /// let crypto_steps = Collection::new()
    ///     .when(None, Regex::new(r"creating a key").unwrap(), test_step);
    /// 
    /// // Merge collections for comprehensive testing
    /// let all_steps = auth_steps.merge(crypto_steps);
    /// ```
    #[must_use]
    pub fn merge(mut self, other: Self) -> Self {
        self.given.extend(other.given);
        self.when.extend(other.when);
        self.then.extend(other.then);
        self
    }

    /// Composes multiple [`Collection`]s into a single collection.
    /// 
    /// This is a convenience method for merging many collections at once,
    /// particularly useful when building enterprise-scale BDD architectures
    /// with multiple domain-specific step builders.
    /// 
    /// # Example
    /// 
    /// ```rust
    /// # use cucumber::step::Collection;
    /// # use regex::Regex;
    /// # use futures::future::LocalBoxFuture;
    /// # 
    /// # #[derive(Default)]
    /// # struct TestWorld;
    /// # 
    /// # fn test_step(_world: &mut TestWorld, _ctx: cucumber::step::Context) -> LocalBoxFuture<'_, ()> {
    /// #     Box::pin(async {})
    /// # }
    /// 
    /// let collections = vec![
    ///     Collection::new().given(None, Regex::new(r"auth").unwrap(), test_step),
    ///     Collection::new().when(None, Regex::new(r"crypto").unwrap(), test_step),
    ///     Collection::new().then(None, Regex::new(r"audit").unwrap(), test_step),
    /// ];
    /// 
    /// let enterprise_steps = Collection::compose(collections);
    /// ```
    #[must_use]
    pub fn compose(collections: Vec<Self>) -> Self {
        collections.into_iter().fold(Self::new(), |acc, collection| acc.merge(collection))
    }

    /// Returns the number of Given step definitions in this collection.
    /// 
    /// This is useful for testing and validation of modular step builders.
    #[must_use]
    pub fn given_len(&self) -> usize {
        self.given.len()
    }

    /// Returns the number of When step definitions in this collection.
    /// 
    /// This is useful for testing and validation of modular step builders.
    #[must_use]
    pub fn when_len(&self) -> usize {
        self.when.len()
    }

    /// Returns the number of Then step definitions in this collection.
    /// 
    /// This is useful for testing and validation of modular step builders.
    #[must_use]
    pub fn then_len(&self) -> usize {
        self.then.len()
    }

    /// Returns the total number of step definitions in this collection.
    /// 
    /// This is useful for testing and validation of modular step builders.
    #[must_use]
    pub fn total_len(&self) -> usize {
        self.given.len() + self.when.len() + self.then.len()
    }

    /// Adds a [Given] [`Step`] matching the given `regex`.
    ///
    /// [Given]: https://cucumber.io/docs/gherkin/reference#given
    #[must_use]
    pub fn given(
        mut self,
        loc: Option<Location>,
        regex: Regex,
        step: Step<World>,
    ) -> Self {
        _ = self.given.insert((regex.into(), loc), step);
        self
    }

    /// Adds a [When] [`Step`] matching the given `regex`.
    ///
    /// [When]: https://cucumber.io/docs/gherkin/reference#when
    #[must_use]
    pub fn when(
        mut self,
        loc: Option<Location>,
        regex: Regex,
        step: Step<World>,
    ) -> Self {
        _ = self.when.insert((regex.into(), loc), step);
        self
    }

    /// Adds a [Then] [`Step`] matching the given `regex`.
    ///
    /// [Then]: https://cucumber.io/docs/gherkin/reference#then
    #[must_use]
    pub fn then(
        mut self,
        loc: Option<Location>,
        regex: Regex,
        step: Step<World>,
    ) -> Self {
        _ = self.then.insert((regex.into(), loc), step);
        self
    }

    /// Returns a [`Step`] function matching the given [`gherkin::Step`], if
    /// any.
    ///
    /// # Errors
    ///
    /// If the given [`gherkin::Step`] matches multiple [`Regex`]es.
    pub fn find(
        &self,
        step: &gherkin::Step,
    ) -> Result<Option<WithContext<'_, World>>, AmbiguousMatchError> {
        let collection = match step.ty {
            StepType::Given => &self.given,
            StepType::When => &self.when,
            StepType::Then => &self.then,
        };

        let mut captures = collection
            .iter()
            .filter_map(|((re, loc), step_fn)| {
                let mut captures = re.capture_locations();
                let names = re.capture_names();
                re.captures_read(&mut captures, &step.value)
                    .map(|m| (re, loc, m, captures, names, step_fn))
            })
            .collect::<Vec<_>>();

        let (_, loc, whole_match, captures, names, step_fn) =
            match captures.len() {
                0 => return Ok(None),
                // Instead of `.unwrap()` to avoid documenting `# Panics`.
                1 => captures.pop().unwrap_or_else(|| unreachable!()),
                _ => {
                    return Err(AmbiguousMatchError {
                        possible_matches: captures
                            .into_iter()
                            .map(|(re, loc, ..)| (re.clone(), *loc))
                            .sorted()
                            .collect(),
                    });
                }
            };

        #[expect( // intentional
            clippy::string_slice,
            reason = "all indices are obtained from the source string"
        )]
        let matches = names
            .map(|opt| opt.map(str::to_owned))
            .zip(iter::once(whole_match.as_str().to_owned()).chain(
                (1..captures.len()).map(|group_id| {
                    captures
                        .get(group_id)
                        .map_or("", |(s, e)| &step.value[s..e])
                        .to_owned()
                }),
            ))
            .collect();

        Ok(Some((
            step_fn,
            captures,
            *loc,
            Context { step: step.clone(), matches },
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gherkin::{Step as GherkinStep, StepType};
    use regex::Regex;

    #[derive(Default)]
    struct TestWorld;

    fn test_step(
        _world: &mut TestWorld,
        _ctx: Context,
    ) -> LocalBoxFuture<'_, ()> {
        Box::pin(async {})
    }

    #[test]
    fn collection_creation_and_step_addition() {
        let collection: Collection<TestWorld> = Collection::new();
        assert!(collection.given.is_empty());

        let regex = Regex::new(r"I have (\d+) cucumbers").unwrap();
        let collection = collection.given(None, regex, test_step);
        assert_eq!(collection.given.len(), 1);
    }

    #[test]
    fn collection_find_functionality() {
        let regex = Regex::new(r"I have (\d+) cucumbers").unwrap();
        let collection = Collection::new().given(None, regex, test_step);

        let step = GherkinStep {
            keyword: "Given".to_string(),
            ty: StepType::Given,
            value: "I have 5 cucumbers".to_string(),
            docstring: None,
            table: None,
            span: gherkin::Span { start: 0, end: 0 },
            position: gherkin::LineCol { line: 1, col: 1 },
        };

        let result = collection.find(&step).unwrap();
        assert!(result.is_some());

        let (_, _, _, context) = result.unwrap();
        assert_eq!(context.matches.len(), 2);
        assert_eq!(context.matches[1].1, "5");
    }

    #[test]
    fn collection_clone_and_default() {
        let regex = Regex::new(r"test").unwrap();
        let collection = Collection::new().given(None, regex, test_step);
        let cloned = collection.clone();
        assert_eq!(cloned.given.len(), 1);

        let default_collection: Collection<TestWorld> = Collection::default();
        assert!(default_collection.given.is_empty());
    }

    #[test]
    fn collection_merge_functionality() {
        // Create first collection with auth steps
        let auth_steps = Collection::new()
            .given(None, Regex::new(r"user is logged in").unwrap(), test_step)
            .when(None, Regex::new(r"user performs auth action").unwrap(), test_step);

        // Create second collection with crypto steps
        let crypto_steps = Collection::new()
            .given(None, Regex::new(r"crypto service is available").unwrap(), test_step)
            .then(None, Regex::new(r"key should be created").unwrap(), test_step);

        // Merge collections
        let merged = auth_steps.merge(crypto_steps);

        // Verify all steps are present
        assert_eq!(merged.given_len(), 2);
        assert_eq!(merged.when_len(), 1);
        assert_eq!(merged.then_len(), 1);
    }

    #[test]
    fn collection_compose_multiple() {
        let collections = vec![
            Collection::new().given(None, Regex::new(r"auth step").unwrap(), test_step),
            Collection::new().when(None, Regex::new(r"crypto step").unwrap(), test_step),
            Collection::new().then(None, Regex::new(r"audit step").unwrap(), test_step),
        ];

        let composed = Collection::compose(collections);

        assert_eq!(composed.given_len(), 1);
        assert_eq!(composed.when_len(), 1);
        assert_eq!(composed.then_len(), 1);
    }

    #[test]
    fn collection_compose_empty_vec() {
        let collections: Vec<Collection<TestWorld>> = vec![];
        let composed = Collection::compose(collections);
        
        assert_eq!(composed.given_len(), 0);
        assert_eq!(composed.when_len(), 0);
        assert_eq!(composed.then_len(), 0);
    }

    #[test]
    fn collection_merge_maintains_step_uniqueness() {
        let regex1 = Regex::new(r"first step").unwrap();
        let regex2 = Regex::new(r"second step").unwrap();
        let same_regex = Regex::new(r"duplicate step").unwrap();

        let collection1 = Collection::new()
            .given(None, regex1, test_step)
            .given(None, same_regex.clone(), test_step);

        let collection2 = Collection::new()
            .given(None, regex2, test_step)
            .given(None, same_regex, test_step); // Duplicate regex

        let merged = collection1.merge(collection2);
        
        // Should have 3 steps total (duplicate regex overwrites)
        assert_eq!(merged.given_len(), 3);
    }

    #[test]
    fn enterprise_modular_pattern_example() {
        // Simulate enterprise domain-specific step builders
        fn create_infrastructure_steps() -> Collection<TestWorld> {
            Collection::new()
                .given(None, Regex::new(r"the vault service is running").unwrap(), test_step)
                .when(None, Regex::new(r"checking the health endpoint").unwrap(), test_step)
        }

        fn create_user_management_steps() -> Collection<TestWorld> {
            Collection::new()
                .given(None, Regex::new(r"(\w+) is an admin user").unwrap(), test_step)
                .when(None, Regex::new(r"(\w+) logs in with credentials").unwrap(), test_step)
        }

        fn create_key_operations_steps() -> Collection<TestWorld> {
            Collection::new()
                .when(None, Regex::new(r#"(\w+) creates a key "([^"]+)""#).unwrap(), test_step)
                .then(None, Regex::new(r"the key should be created successfully").unwrap(), test_step)
        }

        // Build enterprise step collection using modular composition
        let enterprise_collection = Collection::compose(vec![
            create_infrastructure_steps(),
            create_user_management_steps(),
            create_key_operations_steps(),
        ]);

        // Verify comprehensive coverage
        assert_eq!(enterprise_collection.given_len(), 2); // vault running, admin user
        assert_eq!(enterprise_collection.when_len(), 3);  // health check, login, create key  
        assert_eq!(enterprise_collection.then_len(), 1); // key created
        
        println!("✅ Enterprise modular pattern: {} total steps registered", 
                enterprise_collection.total_len());
    }
}
