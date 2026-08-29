//! Injected time.

use std::sync::Mutex;
use time::{Duration, OffsetDateTime};

/// A source of the current instant.
///
/// Implementations are cheap to clone or share; the production implementation
/// holds no state and the test implementations hold only the instant they
/// report.
///
/// # Examples
///
/// ```
/// use app_testkit::{Clock, FixedClock};
/// use time::macros::datetime;
///
/// let clock = FixedClock::new(datetime!(2026-03-01 12:00:00 UTC));
/// assert_eq!(clock.now(), datetime!(2026-03-01 12:00:00 UTC));
/// ```
pub trait Clock: Send + Sync {
    /// The current instant, in UTC.
    fn now(&self) -> OffsetDateTime;
}

/// The real clock. The only place in the workspace that reads wall time.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> OffsetDateTime {
        // This is the single permitted read of wall-clock time in the
        // workspace. Everything else takes a `Clock`, so that elapsed-time
        // behaviour — cure periods, delay windows, expiries — can be driven
        // from a test instead of waited out in real time. A second call site
        // would mean some code path is untestable, which is why the lint that
        // forbids it has exactly one exception, here.
        #[allow(clippy::disallowed_methods)]
        OffsetDateTime::now_utc()
    }
}

/// A clock frozen at one instant.
///
/// Used by tests and by the screenshot pipeline, where a moving clock would
/// make "due in 14 days" drift between runs and churn every image.
#[derive(Debug, Clone, Copy)]
pub struct FixedClock(OffsetDateTime);

impl FixedClock {
    /// A clock that always reports `at`.
    #[must_use]
    pub const fn new(at: OffsetDateTime) -> Self {
        Self(at)
    }
}

impl Clock for FixedClock {
    fn now(&self) -> OffsetDateTime {
        self.0
    }
}

/// A clock a test can step forward.
///
/// This is how a deadline is tested: schedule it, advance past it, and assert
/// it fired — without sleeping.
#[derive(Debug)]
pub struct AdvancingClock(Mutex<OffsetDateTime>);

impl AdvancingClock {
    /// A clock starting at `at`.
    #[must_use]
    pub fn new(at: OffsetDateTime) -> Self {
        Self(Mutex::new(at))
    }

    /// Move the clock forward by `by`.
    ///
    /// # Panics
    ///
    /// Panics if the internal lock has been poisoned by a panic in another
    /// thread, which in a test means an earlier assertion already failed.
    pub fn advance(&self, by: Duration) {
        let mut guard = self.0.lock().expect("advancing clock lock poisoned");
        *guard += by;
    }
}

impl Clock for AdvancingClock {
    fn now(&self) -> OffsetDateTime {
        *self.0.lock().expect("advancing clock lock poisoned")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    #[test]
    fn fixed_clock_does_not_move() {
        let clock = FixedClock::new(datetime!(2026-03-01 12:00:00 UTC));
        let first = clock.now();
        let second = clock.now();
        assert_eq!(first, second);
    }

    #[test]
    fn advancing_clock_moves_only_when_told() {
        let clock = AdvancingClock::new(datetime!(2026-03-01 12:00:00 UTC));
        assert_eq!(clock.now(), datetime!(2026-03-01 12:00:00 UTC));
        clock.advance(Duration::days(30));
        assert_eq!(clock.now(), datetime!(2026-03-31 12:00:00 UTC));
    }

    #[test]
    fn system_clock_is_monotonic_enough_to_be_real() {
        let clock = SystemClock;
        assert!(clock.now() <= clock.now());
    }
}
