//! Templates.
//!
//! Askama checks these at compile time, so a renamed field is a build error
//! rather than a page that breaks in production. Auto-escaping is on; rendering
//! raw HTML requires an explicit, reviewed escape hatch.

use askama::Template;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

/// The Phase 0 page.
#[derive(Template)]
#[template(path = "harness.html")]
pub struct Harness {
    /// Fingerprinted URL for the vendored htmx build.
    pub htmx_url: String,
    /// Fingerprinted URL for the stylesheet.
    pub css_url: String,
    /// The current instant, from the injected clock.
    pub now: String,
}

/// The fragment htmx swaps in.
#[derive(Template)]
#[template(path = "checked.html")]
pub struct Checked {
    /// The instant the check ran, from the injected clock.
    pub now: String,
}

/// Render a template, or fail loudly.
///
/// A template that fails to render is a defect in this build, not a condition
/// to recover from, so it becomes a 500 with the reason logged rather than a
/// partial page.
pub fn render<T: Template>(template: &T) -> Response {
    match template.render() {
        Ok(body) => Html(body).into_response(),
        Err(error) => {
            tracing::error!(%error, "template failed to render");
            (StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response()
        }
    }
}
