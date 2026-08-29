//! What every handler is given.

use crate::assets::Assets;
use app_testkit::{Clock, FixedClock, SystemClock};
use std::sync::Arc;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    /// The clock. Never read wall time directly; take it from here.
    pub clock: Arc<dyn Clock>,
    /// Fingerprinted static assets.
    pub assets: Arc<Assets>,
}

impl AppState {
    /// Build state, choosing a clock from the environment.
    ///
    /// `APP_CLOCK`, if set to an RFC 3339 instant, freezes time. That is how
    /// the screenshot pipeline stays deterministic: with a moving clock, "due
    /// in 14 days" drifts between runs and every image churns. It is also how
    /// tests drive elapsed-time behaviour without sleeping.
    ///
    /// # Errors
    ///
    /// Returns an error if `APP_CLOCK` is set but is not a valid RFC 3339
    /// instant. A malformed value is a misconfiguration worth failing on, not
    /// something to silently fall back from — a screenshot run that quietly
    /// used the real clock would produce a diff nobody could explain.
    pub fn from_env() -> anyhow::Result<Self> {
        let clock: Arc<dyn Clock> = match std::env::var("APP_CLOCK") {
            Ok(raw) => {
                let at = OffsetDateTime::parse(&raw, &Rfc3339).map_err(|source| {
                    anyhow::anyhow!("APP_CLOCK is not a valid RFC 3339 instant ({raw:?}): {source}")
                })?;
                tracing::info!(frozen_at = %raw, "clock frozen from APP_CLOCK");
                Arc::new(FixedClock::new(at))
            }
            Err(_) => Arc::new(SystemClock),
        };

        Ok(Self {
            clock,
            assets: Arc::new(Assets::load()),
        })
    }

    /// State with a fixed clock, for tests.
    #[must_use]
    pub fn fixed_at(at: OffsetDateTime) -> Self {
        Self {
            clock: Arc::new(FixedClock::new(at)),
            assets: Arc::new(Assets::load()),
        }
    }
}
