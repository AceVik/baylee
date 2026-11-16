//! Real Tower HTTP/storage with an explicit synthetic Central identity boundary.
#![forbid(unsafe_code)]
mod host;
mod ranking;
mod story;
#[path = "../../../baylee-tower-store/tests/support/mod.rs"]
mod support;
mod transport;

use baylee_tower_service::{Hosts, Players};
use central_api::Endpoint;
use std::sync::Arc;
use transport::{Central, Server};

#[tokio::test]
async fn native_player_scope_idempotency_and_private_projections() {
    let sandbox = support::Sandbox::open().await;
    let authority = Arc::new(Central::new());
    let central = Server::start(authority.router()).await;
    let endpoint = Endpoint::development(&central.origin()).unwrap();
    // Bind first so the operator's configured origin matches the actual Tower listener.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    *authority.origin.lock().unwrap() = origin.clone();
    let players = Players::new(sandbox.store.clone(), &endpoint, &origin, 1024 * 1024).unwrap();
    let router = Arc::new(players)
        .router()
        .merge(Arc::new(Hosts::new(sandbox.store.clone())).router());
    let server = Server::bound(listener, router);
    let store = sandbox.store.clone();
    let db = sandbox.db.clone();
    let schema = sandbox.schema.clone();
    let task =
        tokio::spawn(async move { story::run(store, authority, origin, db, schema).await }).await;
    let tower_closed = server.close().await;
    let identity_closed = central.close().await;
    sandbox.close().await;
    assert!(
        tower_closed && identity_closed,
        "both owned listeners stopped and ports closed"
    );
    task.expect("synthetic Tower HTTP scenario completed");
}
