//! Extensions to Cucumber for better integration with external systems

use crate::{Cucumber, Parser, World, Writer, runner::Basic};

impl<W, P, I, Wr, Cli> Cucumber<W, P, I, Basic<W>, Wr, Cli>
where
    W: World,
    P: Parser<I>,
    Wr: Writer<W>,
    Cli: clap::Args,
{
    /// Register an observer for test execution monitoring
    ///
    /// This allows external systems like ObservaBDD to observe test execution
    /// without modifying the writer chain.
    ///
    /// # Example
    /// ```rust
    /// # use cucumber::{Cucumber, World};
    /// # #[derive(Debug, Default, World)]
    /// # struct TestWorld;
    /// # #[cfg(feature = "observability")]
    /// # async fn example() {
    /// # use cucumber::observer::{TestObserver, ObservationContext};
    /// # use cucumber::Event;
    /// # struct MyObserver;
    /// # impl TestObserver<TestWorld> for MyObserver {
    /// #     fn on_event(&mut self, _event: &Event<cucumber::event::Cucumber<TestWorld>>, _ctx: &ObservationContext) {}
    /// # }
    /// # let my_observer = MyObserver;
    /// let cucumber =
    ///     TestWorld::cucumber::<&str>().register_observer(Box::new(my_observer));
    /// # }
    /// ```
    #[cfg(feature = "observability")]
    pub fn register_observer(
        mut self,
        observer: Box<dyn crate::observer::TestObserver<W>>,
    ) -> Self {
        self.runner = self.runner.register_observer(observer);
        self
    }
}
