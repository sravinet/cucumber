//! Step builder traits and patterns for modular BDD architectures.
//!
//! This module provides the [`StepBuilder`] trait and related patterns that enable
//! enterprise-scale BDD testing by allowing different teams to own different 
//! domain-specific step definitions.

use super::Collection;

/// Trait for modular step definition builders.
/// 
/// This trait enables domain-driven organization of step definitions, where each
/// domain (e.g., authentication, cryptography, monitoring) can have its own
/// step builder implementation owned by the relevant team.
/// 
/// # Enterprise Benefits
/// 
/// - **Team Ownership**: Each domain team owns their step definitions
/// - **Reusability**: Step builders can be composed into different test suites  
/// - **Testability**: Individual step groups can be unit tested
/// - **Scalability**: Supports 200+ steps without conflicts
/// - **Maintainability**: Clean separation prevents merge conflicts
/// 
/// # Example
/// 
/// ```rust
/// use cucumber::step::{Collection, StepBuilder};
/// use regex::Regex;
/// use futures::future::LocalBoxFuture;
/// 
/// #[derive(Default)]
/// struct TestWorld;
/// 
/// fn test_step(_world: &mut TestWorld, _ctx: cucumber::step::Context) -> LocalBoxFuture<'_, ()> {
///     Box::pin(async {})
/// }
/// 
/// pub struct AuthenticationSteps;
/// 
/// impl StepBuilder<TestWorld> for AuthenticationSteps {
///     fn register_steps(collection: Collection<TestWorld>) -> Collection<TestWorld> {
///         collection
///             .given(None, Regex::new(r"user is logged in").unwrap(), test_step)
///             .when(None, Regex::new(r"user performs login").unwrap(), test_step)
///             .then(None, Regex::new(r"user should be authenticated").unwrap(), test_step)
///     }
///     
///     fn domain_name() -> &'static str {
///         "Authentication & Authorization"
///     }
/// }
/// 
/// // Build modular collection
/// let steps = AuthenticationSteps::register_steps(Collection::new());
/// ```
pub trait StepBuilder<World> {
    /// Registers all step definitions for this domain into the provided collection.
    /// 
    /// This method should add all Given/When/Then step definitions that belong
    /// to this domain's responsibility area.
    fn register_steps(collection: Collection<World>) -> Collection<World>;
    
    /// Returns the human-readable name of this step builder's domain.
    /// 
    /// This is used for documentation and debugging purposes to identify
    /// which team or domain owns these step definitions.
    fn domain_name() -> &'static str;
}

/// Composes multiple step builders into a single collection.
/// 
/// This function takes a vector of step builder registration functions and
/// combines them into a unified step collection, enabling enterprise-scale
/// BDD testing with clear domain separation.
/// 
/// # Example
/// 
/// ```rust
/// use cucumber::step::{Collection, compose_step_builders};
/// use regex::Regex;
/// use futures::future::LocalBoxFuture;
/// 
/// #[derive(Default)]
/// struct TestWorld;
/// 
/// fn test_step(_world: &mut TestWorld, _ctx: cucumber::step::Context) -> LocalBoxFuture<'_, ()> {
///     Box::pin(async {})
/// }
/// 
/// let builders: Vec<Box<dyn Fn(Collection<TestWorld>) -> Collection<TestWorld>>> = vec![
///     Box::new(|c| c.given(None, Regex::new(r"auth").unwrap(), test_step)),
///     Box::new(|c| c.when(None, Regex::new(r"crypto").unwrap(), test_step)),
///     Box::new(|c| c.then(None, Regex::new(r"audit").unwrap(), test_step)),
/// ];
/// 
/// let enterprise_steps = compose_step_builders(builders);
/// ```
pub fn compose_step_builders<World>(
    builders: Vec<Box<dyn Fn(Collection<World>) -> Collection<World>>>
) -> Collection<World> {
    builders.into_iter().fold(Collection::new(), |acc, builder| builder(acc))
}

/// Macro for implementing step builders with consistent patterns.
/// 
/// This macro reduces boilerplate when creating domain-specific step builders
/// and ensures consistent implementation patterns across teams.
/// 
/// # Example
/// 
/// ```rust
/// use cucumber::step_builder;
/// use cucumber::step::{Collection, StepBuilder};
/// use regex::Regex;
/// use futures::future::LocalBoxFuture;
/// 
/// #[derive(Default)]
/// struct TestWorld;
/// 
/// fn test_step(_world: &mut TestWorld, _ctx: cucumber::step::Context) -> LocalBoxFuture<'_, ()> {
///     Box::pin(async {})
/// }
/// 
/// step_builder!(
///     CryptoSteps,
///     "Cryptographic Operations", 
///     TestWorld,
///     |collection| {
///         collection
///             .when(None, Regex::new(r"creating a key").unwrap(), test_step)
///             .then(None, Regex::new(r"key should be created").unwrap(), test_step)
///     }
/// );
/// 
/// // Use the generated step builder
/// let crypto_steps = CryptoSteps::register_steps(Collection::new());
/// ```
#[macro_export]
macro_rules! step_builder {
    ($name:ident, $domain:expr, $world:ty, |$collection:ident| $body:expr) => {
        pub struct $name;
        
        impl StepBuilder<$world> for $name {
            fn register_steps($collection: Collection<$world>) -> Collection<$world> {
                $body
            }
            
            fn domain_name() -> &'static str {
                $domain
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::step::Context;
    use futures::future::LocalBoxFuture;
    use regex::Regex;

    #[derive(Default)]
    struct TestWorld;

    fn test_step(
        _world: &mut TestWorld,
        _ctx: Context,
    ) -> LocalBoxFuture<'_, ()> {
        Box::pin(async {})
    }

    struct MockAuthSteps;
    
    impl StepBuilder<TestWorld> for MockAuthSteps {
        fn register_steps(collection: Collection<TestWorld>) -> Collection<TestWorld> {
            collection
                .given(None, Regex::new(r"user is logged in").unwrap(), test_step)
                .when(None, Regex::new(r"user performs login").unwrap(), test_step)
        }
        
        fn domain_name() -> &'static str {
            "Authentication & Authorization"
        }
    }

    struct MockCryptoSteps;
    
    impl StepBuilder<TestWorld> for MockCryptoSteps {
        fn register_steps(collection: Collection<TestWorld>) -> Collection<TestWorld> {
            collection
                .when(None, Regex::new(r"creating a key").unwrap(), test_step)
                .then(None, Regex::new(r"key should be created").unwrap(), test_step)
        }
        
        fn domain_name() -> &'static str {
            "Cryptographic Operations"
        }
    }

    #[test]
    fn step_builder_trait_implementation() {
        let auth_steps = MockAuthSteps::register_steps(Collection::new());
        assert_eq!(auth_steps.given_len(), 1);
        assert_eq!(auth_steps.when_len(), 1);
        assert_eq!(auth_steps.then_len(), 0);
        
        assert_eq!(MockAuthSteps::domain_name(), "Authentication & Authorization");
    }

    #[test]
    fn compose_step_builders_functionality() {
        let builders: Vec<Box<dyn Fn(Collection<TestWorld>) -> Collection<TestWorld>>> = vec![
            Box::new(MockAuthSteps::register_steps),
            Box::new(MockCryptoSteps::register_steps),
        ];

        let composed = compose_step_builders(builders);
        
        assert_eq!(composed.given_len(), 1); // auth given
        assert_eq!(composed.when_len(), 2);  // auth + crypto when
        assert_eq!(composed.then_len(), 1); // crypto then
    }

    #[test]
    fn macro_step_builder_pattern() {
        step_builder!(
            MonitoringSteps,
            "Health & Monitoring",
            TestWorld,
            |collection| {
                collection
                    .given(None, Regex::new(r"service is healthy").unwrap(), test_step)
                    .when(None, Regex::new(r"checking health endpoint").unwrap(), test_step)
                    .then(None, Regex::new(r"should return healthy status").unwrap(), test_step)
            }
        );

        let monitoring_steps = MonitoringSteps::register_steps(Collection::new());
        assert_eq!(monitoring_steps.given_len(), 1);
        assert_eq!(monitoring_steps.when_len(), 1);
        assert_eq!(monitoring_steps.then_len(), 1);
        
        assert_eq!(MonitoringSteps::domain_name(), "Health & Monitoring");
    }

    #[test]
    fn enterprise_pattern_integration() {
        // Demonstrate the full enterprise pattern
        let enterprise_collection = Collection::compose(vec![
            MockAuthSteps::register_steps(Collection::new()),
            MockCryptoSteps::register_steps(Collection::new()),
        ]);

        // Verify comprehensive step coverage
        assert_eq!(enterprise_collection.given_len(), 1);
        assert_eq!(enterprise_collection.when_len(), 2);
        assert_eq!(enterprise_collection.then_len(), 1);
        
        println!("✅ Enterprise pattern: {} total steps across {} domains", 
                enterprise_collection.total_len(),
                2);
    }
}