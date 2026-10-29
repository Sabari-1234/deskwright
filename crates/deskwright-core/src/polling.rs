//! Waiting for the operating system to catch up: many desktop changes (window activation,
//! cursor moves) are applied asynchronously, so adapters poll for the expected state.

use std::thread;
use std::time::{Duration, Instant};

/// The pauses between checks: each in turn, then the last one repeated. Starting short and
/// backing off (like Playwright's auto-wait) notices fast UI changes quickly without
/// re-reading a slow app's tree in a tight loop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PollSchedule {
    pauses: Vec<Duration>,
}

impl PollSchedule {
    /// Pauses for `pauses` in order, then keeps repeating the last one. An empty list means
    /// no pause at all.
    pub fn backoff(pauses: &[Duration]) -> Self {
        Self {
            pauses: pauses.to_vec(),
        }
    }

    /// The pause after the `attempt`-th failed check (0-based).
    pub fn pause_after(&self, attempt: usize) -> Duration {
        self.pauses
            .get(attempt)
            .or_else(|| self.pauses.last())
            .copied()
            .unwrap_or(Duration::ZERO)
    }
}

impl From<Duration> for PollSchedule {
    /// The same pause between every check.
    fn from(pause: Duration) -> Self {
        Self {
            pauses: vec![pause],
        }
    }
}

/// Polls `is_done` until it returns true or `timeout` passes; returns whether it finished.
/// The last pause is cut short at the deadline, so a wait never overruns its timeout by a
/// whole pause.
pub fn wait_until(
    timeout: Duration,
    schedule: impl Into<PollSchedule>,
    mut is_done: impl FnMut() -> bool,
) -> bool {
    let schedule = schedule.into();
    let deadline = Instant::now() + timeout;
    let mut attempt = 0;
    loop {
        if is_done() {
            return true;
        }
        let now = Instant::now();
        if now >= deadline {
            return false;
        }
        thread::sleep(schedule.pause_after(attempt).min(deadline - now));
        attempt = attempt.saturating_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stops_as_soon_as_the_condition_holds() {
        let mut polls = 0;

        let is_done = wait_until(Duration::from_secs(5), Duration::ZERO, || {
            polls += 1;
            polls == 3
        });

        assert!(is_done);
        assert_eq!(polls, 3);
    }

    #[test]
    fn gives_up_after_the_timeout() {
        assert!(!wait_until(Duration::ZERO, Duration::ZERO, || false));
    }

    #[test]
    fn never_sleeps_past_the_deadline() {
        let started = Instant::now();

        let is_done = wait_until(Duration::from_millis(30), Duration::from_secs(10), || false);

        assert!(!is_done);
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn backs_off_then_repeats_the_last_pause() {
        let schedule =
            PollSchedule::backoff(&[Duration::from_millis(20), Duration::from_millis(50)]);

        assert_eq!(schedule.pause_after(0), Duration::from_millis(20));
        assert_eq!(schedule.pause_after(1), Duration::from_millis(50));
        assert_eq!(schedule.pause_after(9), Duration::from_millis(50));
        assert_eq!(PollSchedule::backoff(&[]).pause_after(0), Duration::ZERO);
    }
}
