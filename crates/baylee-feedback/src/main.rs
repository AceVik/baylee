//! `baylee-feedback`: keeps the reports gateways pass on (`docs/feedback.md`).
//!
//! `FEEDBACK_DATABASE_URL` (required), `FEEDBACK_GATEWAY_TOKENS`
//! (`name=token,…`), `FEEDBACK_READ_TOKEN`, `FEEDBACK_ADMIN_TOKEN`,
//! `FEEDBACK_BIND` (`127.0.0.1:28780`), `FEEDBACK_POOL` (4).

use std::sync::Arc;

use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
    let config = baylee_feedback::Config::from_env()?;
    let url = std::env::var("FEEDBACK_DATABASE_URL")
        .ok()
        .filter(|u| !u.is_empty())
        .ok_or_else(|| anyhow::anyhow!("FEEDBACK_DATABASE_URL is not set"))?;
    let pool = std::env::var("FEEDBACK_POOL")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(4);
    let db = baylee_feedback::connect(&url, pool).await?;
    let bind = std::env::var("FEEDBACK_BIND").unwrap_or_else(|_| "127.0.0.1:28780".to_owned());
    let listener = tokio::net::TcpListener::bind(&bind).await?;
    tracing::info!(
        bind,
        version = baylee_build::short(),
        "baylee-feedback serving"
    );
    let app = baylee_feedback::app(Arc::new(baylee_feedback::AppState { db, config }));
    axum::serve(listener, app).await?;
    Ok(())
}
