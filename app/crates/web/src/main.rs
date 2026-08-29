//! The application binary.

use anyhow::Context as _;
use app_web::{AppState, router};
use std::net::SocketAddr;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with_target(false)
        .init();

    let state = AppState::from_env().context("building application state")?;

    // Binding to port 0 lets the end-to-end harness start a server without
    // racing another run for a fixed port; it prints the address it got.
    let port: u16 = match std::env::var("APP_PORT") {
        // A malformed value is a misconfiguration worth failing on, not
        // something to fall back from: silently binding the default would send
        // the end-to-end harness at whatever else is listening there.
        Ok(raw) => raw
            .parse()
            .with_context(|| format!("APP_PORT is not a valid port number ({raw:?})"))?,
        Err(_) => 8080,
    };
    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("binding {addr}"))?;
    let bound = listener.local_addr().context("reading the bound address")?;

    // The harness waits for this line before driving the browser.
    println!("listening on http://{bound}");
    tracing::info!(%bound, "listening");

    axum::serve(listener, router(state))
        .with_graceful_shutdown(shutdown())
        .await
        .context("serving")?;

    Ok(())
}

async fn shutdown() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("shutting down");
}
