use crate::error::{DeskwrightError, DeskwrightResult};

/// A strategy that can report which approach it is, for results and error messages.
pub trait NamedStrategy {
    fn name(&self) -> &'static str;
}

/// Which strategy completed an interaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InteractionOutcome {
    pub strategy_name: &'static str,
}

/// Chain of Responsibility over strategies: tries each in order and stops at the first
/// success. When all fail, the error lists every attempt so the caller can see why.
pub struct StrategyChain<Strategy: ?Sized> {
    action: &'static str,
    strategies: Vec<Box<Strategy>>,
}

impl<Strategy: ?Sized + NamedStrategy> StrategyChain<Strategy> {
    pub fn new(action: &'static str, strategies: Vec<Box<Strategy>>) -> Self {
        Self { action, strategies }
    }

    pub fn run_until_success(
        &self,
        mut attempt: impl FnMut(&Strategy) -> DeskwrightResult<()>,
    ) -> DeskwrightResult<InteractionOutcome> {
        let mut failed_attempts = Vec::new();
        for strategy in &self.strategies {
            match attempt(strategy) {
                Ok(()) => {
                    return Ok(InteractionOutcome {
                        strategy_name: strategy.name(),
                    });
                }
                Err(error) => failed_attempts.push(format!("{}: {error}", strategy.name())),
            }
        }
        Err(DeskwrightError::AllStrategiesFailed {
            action: self.action,
            attempts: failed_attempts,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FixedStrategy {
        name: &'static str,
        succeeds: bool,
    }

    impl NamedStrategy for FixedStrategy {
        fn name(&self) -> &'static str {
            self.name
        }
    }

    fn chain_of(outcomes: &[(&'static str, bool)]) -> StrategyChain<FixedStrategy> {
        let strategies = outcomes
            .iter()
            .map(|&(name, succeeds)| Box::new(FixedStrategy { name, succeeds }))
            .collect();
        StrategyChain::new("do the thing", strategies)
    }

    fn attempt(strategy: &FixedStrategy) -> DeskwrightResult<()> {
        if strategy.succeeds {
            Ok(())
        } else {
            Err(DeskwrightError::operation_failed("attempt", "no luck"))
        }
    }

    #[test]
    fn stops_at_the_first_successful_strategy() {
        let chain = chain_of(&[("first", false), ("second", true), ("third", true)]);

        let outcome = chain.run_until_success(attempt).unwrap();

        assert_eq!(outcome.strategy_name, "second");
    }

    #[test]
    fn reports_every_failed_attempt() {
        let chain = chain_of(&[("first", false), ("second", false)]);

        let error = chain.run_until_success(attempt).unwrap_err();

        assert_eq!(
            error,
            DeskwrightError::AllStrategiesFailed {
                action: "do the thing",
                attempts: vec![
                    "first: attempt failed: no luck".to_owned(),
                    "second: attempt failed: no luck".to_owned(),
                ],
            }
        );
    }
}
