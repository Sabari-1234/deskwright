use std::time::Instant;

/// The current time, injected so step timings are deterministic in tests.
pub trait Clock: Send + Sync {
    fn now(&self) -> Instant;
}

/// The real monotonic clock.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_system_clock_moves_forward() {
        let first = SystemClock.now();

        assert!(SystemClock.now() >= first);
    }
}
