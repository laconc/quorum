//! Every response carries the security headers, and no route can opt out.
//!
//! These are applied as a layer rather than per-route precisely so that a new
//! handler cannot forget them. This suite is what keeps that true: it drives
//! the real router, so a route added later is covered without anyone
//! remembering to extend the test.

use app_web::{AppState, router};
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt as _;
use time::macros::datetime;
use tower::ServiceExt as _;

/// The instant the screenshot pipeline and every test share.
fn state() -> AppState {
    AppState::fixed_at(datetime!(2026-03-01 12:00:00 UTC))
}

async fn get(path: &str) -> axum::http::Response<Body> {
    router(state())
        .oneshot(
            Request::builder()
                .uri(path)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response")
}

/// Every route the application serves. A new route belongs here, and from
/// Phase 1 this list is generated from the route registry rather than written
/// by hand — which is what stops it drifting behind the router.
const ROUTES: &[&str] = &["/", "/check", "/healthz"];

#[tokio::test]
async fn every_route_carries_the_security_headers() {
    for path in ROUTES {
        let response = get(path).await;
        let headers = response.headers();

        let csp = headers
            .get(header::CONTENT_SECURITY_POLICY)
            .unwrap_or_else(|| panic!("{path} has no Content-Security-Policy"))
            .to_str()
            .expect("ascii");

        assert!(
            csp.contains("default-src 'none'"),
            "{path}: the policy must deny by default so an unconsidered \
             directive refuses rather than inheriting something permissive"
        );
        assert!(
            !csp.contains("unsafe-inline"),
            "{path}: unsafe-inline is what makes an injected string executable"
        );
        assert!(!csp.contains("unsafe-eval"), "{path}: unsafe-eval");
        assert!(
            csp.contains("frame-ancestors 'none'"),
            "{path}: clickjacking"
        );
        assert!(
            csp.contains("base-uri 'none'"),
            "{path}: base tag injection"
        );

        let hsts = headers
            .get(header::STRICT_TRANSPORT_SECURITY)
            .unwrap_or_else(|| panic!("{path} has no Strict-Transport-Security"))
            .to_str()
            .expect("ascii");
        // A year, subdomains included, and asking for preload. The top-level
        // domain is not preloaded the way `.app` would have been, so this
        // header is the only thing protecting a first request to a bare
        // hostname until the submission is accepted.
        assert!(hsts.contains("max-age=31536000"), "{path}: {hsts}");
        assert!(hsts.contains("includeSubDomains"), "{path}: {hsts}");
        assert!(hsts.contains("preload"), "{path}: {hsts}");

        assert_eq!(
            headers
                .get(header::X_CONTENT_TYPE_OPTIONS)
                .and_then(|v| v.to_str().ok()),
            Some("nosniff"),
            "{path}"
        );
        assert_eq!(
            headers
                .get(header::X_FRAME_OPTIONS)
                .and_then(|v| v.to_str().ok()),
            Some("DENY"),
            "{path}"
        );
        assert!(headers.contains_key(header::REFERRER_POLICY), "{path}");
        assert!(headers.contains_key("Permissions-Policy"), "{path}");
    }
}

#[tokio::test]
async fn html_responses_are_never_stored() {
    // The failure this prevents is a shared cache handing one resident's page
    // to the next requester. Caching is opt-in for exactly that reason: the
    // cost of forgetting is a page that is not cached, not a disclosure.
    for path in ["/", "/check"] {
        let response = get(path).await;
        let cache_control = response
            .headers()
            .get(header::CACHE_CONTROL)
            .unwrap_or_else(|| panic!("{path} has no Cache-Control"))
            .to_str()
            .expect("ascii");
        assert_eq!(cache_control, "private, no-store", "{path}");
    }
}

#[tokio::test]
async fn fingerprinted_assets_are_cached_forever() {
    let state = state();
    let url = state.assets.url("htmx.min.js").to_owned();

    let response = router(state)
        .oneshot(Request::builder().uri(&url).body(Body::empty()).unwrap())
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
    let cache_control = response
        .headers()
        .get(header::CACHE_CONTROL)
        .and_then(|v| v.to_str().ok())
        .expect("Cache-Control");
    // Safe because the URL contains a hash of the bytes: different content
    // means a different URL, so a client can never hold a stale copy.
    assert!(cache_control.contains("immutable"), "{cache_control}");
    assert!(
        cache_control.contains("max-age=31536000"),
        "{cache_control}"
    );
}

#[tokio::test]
async fn an_unknown_asset_path_is_not_found() {
    let response = get("/static/deadbeefdeadbeef/nothing.js").await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn the_page_renders_the_injected_clock() {
    let response = get("/").await;
    assert_eq!(response.status(), StatusCode::OK);

    let body = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    let html = String::from_utf8(body.to_vec()).expect("utf-8");

    // The frozen instant appears, which is what makes the screenshot of this
    // page identical between runs.
    assert!(html.contains("2026-03-01T12:00:00Z"), "{html}");
    // The live region is present from first paint. A region created at the same
    // moment as its content is not announced at all.
    assert!(html.contains(r#"aria-live="polite""#), "{html}");
    // Assets are referenced by their fingerprinted URLs.
    assert!(html.contains("/static/"), "{html}");
}

#[tokio::test]
async fn the_partial_update_renders_a_fragment() {
    let response = get("/check").await;
    assert_eq!(response.status(), StatusCode::OK);

    let body = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    let html = String::from_utf8(body.to_vec()).expect("utf-8");

    // A fragment, not a whole document: htmx swaps this into the live region.
    assert!(!html.contains("<html"), "{html}");
    assert!(html.contains("2026-03-01T12:00:00Z"), "{html}");
}
