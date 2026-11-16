//! Real socket, real PostgreSQL and a synthetic identity authority. No UI polling.
use super::{Central, Endpoint, Players, Server, support};
use baylee_tower::v2::{run::Command, shop::Delivery};
use baylee_tower_api::{
    Request, Response,
    socket::{self as wire, Refusal},
};
use futures_util::{SinkExt as _, StreamExt as _};
use std::{
    sync::{Arc, atomic::Ordering},
    time::Duration,
};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream,
    tungstenite::{Message, client::IntoClientRequest as _},
};
use uuid::Uuid;

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

async fn connect(origin: &str, token: &central_api::Token) -> Socket {
    let url = format!("{}{}", origin.replacen("http", "ws", 1), wire::PATH);
    let mut request = url.into_client_request().unwrap();
    request
        .headers_mut()
        .insert("authorization", token.bearer().parse().unwrap());
    let (mut socket, _) = tokio_tungstenite::connect_async(request).await.unwrap();
    assert!(
        matches!(receive(&mut socket).await, wire::Server::Ready { version } if version == baylee_tower_api::VERSION)
    );
    socket
}

async fn receive(socket: &mut Socket) -> wire::Server {
    tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            match socket.next().await.unwrap().unwrap() {
                Message::Binary(bytes) => break serde_json::from_slice(&bytes).unwrap(),
                Message::Text(text) => break serde_json::from_str(&text).unwrap(),
                Message::Ping(_) => socket.flush().await.unwrap(),
                _ => panic!("unexpected socket frame"),
            }
        }
    })
    .await
    .expect("bounded socket response")
}

async fn exchange(socket: &mut Socket, request: Request) -> Result<Response, Refusal> {
    let serial = Uuid::now_v7();
    socket
        .send(Message::Binary(
            serde_json::to_vec(&wire::Client { serial, request })
                .unwrap()
                .into(),
        ))
        .await
        .unwrap();
    loop {
        match receive(socket).await {
            wire::Server::Reply {
                serial: received,
                result,
            } => {
                assert_eq!(received, serial);
                return result;
            }
            wire::Server::Changed { .. } => {}
            wire::Server::Ready { .. } => panic!("unexpected ready"),
        }
    }
}

fn run(response: Response) -> Box<baylee_tower_api::Run> {
    match response {
        Response::Run { run } => run,
        _ => panic!("expected run"),
    }
}

#[tokio::test]
async fn live_run_updates_are_owned_committed_deduplicated_and_revocable() {
    let sandbox = support::Sandbox::open().await;
    let authority = Arc::new(Central::new());
    let central = Server::start(authority.router()).await;
    let endpoint = Endpoint::development(&central.origin()).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    *authority.origin.lock().unwrap() = origin.clone();
    let players =
        Arc::new(Players::new(sandbox.store.clone(), &endpoint, &origin, 1024 * 1024).unwrap());
    let server = Server::bound(listener, players.router());
    let store = sandbox.store.clone();
    let task = tokio::spawn(async move {
        let cohort = store.publish(&support::publication()).await.unwrap();
        let mut first = connect(&origin, &authority.tokens[0]).await;
        let Response::Cohorts { consent, .. } = exchange(&mut first, Request::Cohorts { before: None }).await.unwrap() else { panic!("cohorts") };
        let initial = run(exchange(&mut first, Request::Start { request: Uuid::now_v7(), cohort, starter: 0, consent }).await.unwrap());
        let mut second = connect(&origin, &authority.tokens[0]).await;
        assert_eq!(run(exchange(&mut second, Request::Run { run: initial.id }).await.unwrap()), initial);
        let mut foreign = connect(&origin, &authority.tokens[1]).await;
        assert_eq!(exchange(&mut foreign, Request::Run { run: initial.id }).await, Err(Refusal::Missing));
        let purchase = Request::Command {
            request: Uuid::now_v7(), run: initial.id, revision: initial.revision,
            command: Command::Buy { offer: initial.shop.offers[0].id, delivery: Delivery::Inventory },
        };
        let bought = run(exchange(&mut first, purchase.clone()).await.unwrap());
        assert_eq!(bought.revision, initial.revision + 1);
        assert!(matches!(receive(&mut second).await, wire::Server::Changed { response: Response::Run { run } } if run == bought));
        assert_eq!(run(exchange(&mut first, purchase.clone()).await.unwrap()), bought, "retry cannot spend twice");
        assert_eq!(exchange(&mut foreign, purchase).await, Err(Refusal::Missing));
        // A write made through the store, as a verifier/other authority would do,
        // must wake the existing socket without any client read request.
        let changed = store.command(authority.accounts[0], Uuid::now_v7(), initial.id, bought.revision, &Command::Refresh {}).await.unwrap();
        assert!(matches!(receive(&mut second).await, wire::Server::Changed { response: Response::Run { run } } if run.revision == changed.state.revision));
        // Same account may hold at most two sockets; a third consumes no work slot.
        let mut third = format!("{}{}", origin.replacen("http", "ws", 1), wire::PATH).into_client_request().unwrap();
        third.headers_mut().insert("authorization", authority.tokens[0].bearer().parse().unwrap());
        assert!(matches!(tokio_tungstenite::connect_async(third).await, Err(tokio_tungstenite::tungstenite::Error::Http(reply)) if reply.status().as_u16() == 429));
        authority.status.store(401, Ordering::SeqCst);
        assert_eq!(exchange(&mut first, Request::Run { run: initial.id }).await, Err(Refusal::Unauthorized));
        let _ = first.close(None).await;
        let _ = second.close(None).await;
        let _ = foreign.close(None).await;
    }).await;
    let tower_closed = server.close().await;
    let central_closed = central.close().await;
    sandbox.close().await;
    assert!(tower_closed && central_closed, "owned ports closed");
    task.expect("live socket acceptance completed");
}
