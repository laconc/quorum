//! The security headers every response carries.
//!
//! These are applied as a layer rather than per-route so that a new handler
//! cannot forget them. A route that needs different caching opts in explicitly;
//! it cannot opt out of the rest.

use axum::extract::Request;
use axum::http::HeaderValue;
use axum::http::header::{
    CACHE_CONTROL, CONTENT_SECURITY_POLICY, REFERRER_POLICY, STRICT_TRANSPORT_SECURITY,
    X_CONTENT_TYPE_OPTIONS, X_FRAME_OPTIONS,
};
use axum::middleware::Next;
use axum::response::Response;

/// The Content Security Policy.
///
/// `default-src 'none'` and then an explicit allowance per resource type, so a
/// directive nobody thought about denies rather than inherits something
/// permissive.
///
/// Nothing is admitted that is not actually used: `img-src` has no `data:`
/// allowance because no page needs one yet. Widening this is a deliberate
/// act; an unused allowance nobody notices becomes permanent.
///
/// There is no `unsafe-inline` and no `unsafe-eval`. That forbids htmx's
/// `hx-on` attributes, which is accepted deliberately: inline handlers are the
/// sink that makes an injected string executable, and giving them up costs
/// little in an application that is forms and lists.
const CSP: &str = "default-src 'none'; \
     script-src 'self'; \
     style-src 'self'; \
     img-src 'self'; \
     font-src 'self'; \
     connect-src 'self'; \
     form-action 'self'; \
     base-uri 'none'; \
     frame-ancestors 'none'";

/// One year, with subdomains, and asking to be preloaded.
///
/// This matters more than it would on a preloaded top-level domain. The
/// original design assumed `.app`, where browsers enforce HTTPS before a
/// request is ever made; `.community` carries no such guarantee, so until the
/// domain is accepted onto the preload list the first request to a bare
/// hostname is unprotected. Serving the header is what makes that submission
/// possible.
const HSTS: &str = "max-age=31536000; includeSubDomains; preload";

/// Apply the headers.
///
/// `Cache-Control: private, no-store` is the default for every response.
/// Anything publicly cacheable sets its own value, and this layer leaves an
/// existing `Cache-Control` alone — so caching is opt-in, and the failure mode
/// of forgetting is a page that is not cached rather than a member's document
/// handed to the next requester.
pub async fn security_headers(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    headers.insert(CONTENT_SECURITY_POLICY, HeaderValue::from_static(CSP));
    headers.insert(STRICT_TRANSPORT_SECURITY, HeaderValue::from_static(HSTS));
    headers.insert(X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    headers.insert(X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    headers.insert(REFERRER_POLICY, HeaderValue::from_static("same-origin"));
    // Nothing here needs a camera yet. Phase 3 opens it to `self` when direct
    // photo capture arrives for violations and architectural requests — the one
    // place where a phone genuinely beats a desktop. Until then it is denied,
    // for the same reason `img-src` carries no `data:` allowance: a capability
    // granted before anything asks for it never gets revisited.
    headers.insert(
        "Permissions-Policy",
        HeaderValue::from_static("camera=(), geolocation=(), microphone=(), payment=()"),
    );

    if !headers.contains_key(CACHE_CONTROL) {
        headers.insert(CACHE_CONTROL, HeaderValue::from_static("private, no-store"));
    }

    response
}
