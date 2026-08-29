//! The web application.
//!
//! Server-rendered HTML with htmx for partial updates, and no client-side
//! framework. There is no session token in JavaScript's reach, so a scripting
//! flaw cannot exfiltrate one; the application is forms and lists, which a
//! bundle download and a token-handling problem would not improve.
//!
//! Phase 0 serves one page. It exists to prove the harness end to end: Rust
//! renders it, templates are checked at compile time, htmx swaps into a live
//! region, the Content Security Policy admits the vendored script and nothing
//! else, and a screenshot of it is byte-identical between runs.

pub mod assets;
pub mod headers;
pub mod routes;
pub mod state;
pub mod views;

pub use routes::router;
pub use state::AppState;
