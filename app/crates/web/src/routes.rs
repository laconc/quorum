//! Route registration.
//!
//! Phase 1 replaces this with a `routes!` registry that is the single source of
//! truth for the router, the authorization matrix, the unauthenticated
//! cache-leak sweep, and the tenant-isolation sweep. Until those harnesses
//! exist there is nothing to generate, so this stays an ordinary router — but
//! new routes belong in the registry from the moment it lands, because a route
//! registered outside it silently opts out of all three checks.

use crate::state::AppState;
use crate::views::{Checked, Harness, render};
use axum::extract::{Path, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Router, middleware};
use time::format_description::well_known::Rfc3339;

/// Build the router.
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(harness))
        .route("/check", get(check))
        .route("/healthz", get(healthz))
        .route("/static/{*path}", get(static_asset))
        .layer(middleware::from_fn(crate::headers::security_headers))
        .with_state(state)
}

fn now_string(state: &AppState) -> String {
    state
        .clock
        .now()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "unknown".to_owned())
}

async fn harness(State(state): State<AppState>) -> Response {
    render(&Harness {
        htmx_url: state.assets.url("htmx.min.js").to_owned(),
        css_url: state.assets.url("app.css").to_owned(),
        now: now_string(&state),
    })
}

/// The target of the page's one htmx swap.
///
/// A `GET` because it changes nothing. State-changing requests never use `GET`,
/// without exception — that rule is what lets a link be safe to follow and a
/// page be safe to prefetch.
async fn check(State(state): State<AppState>) -> Response {
    render(&Checked {
        now: now_string(&state),
    })
}

async fn healthz() -> &'static str {
    "ok"
}

async fn static_asset(State(state): State<AppState>, Path(path): Path<String>) -> Response {
    let full = format!("/static/{path}");
    let Some(asset) = state.assets.get(&full) else {
        return (StatusCode::NOT_FOUND, "not found").into_response();
    };

    (
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static(asset.content_type),
            ),
            // Safe to cache forever because the URL contains a hash of the
            // contents: different bytes mean a different URL.
            (
                header::CACHE_CONTROL,
                HeaderValue::from_static("public, max-age=31536000, immutable"),
            ),
        ],
        asset.body,
    )
        .into_response()
}
