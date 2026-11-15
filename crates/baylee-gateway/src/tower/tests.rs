use super::*;
use axum::{Json, Router, extract::State, http::StatusCode, routing::post};
use baylee_account::Token;
use std::time::Duration;

async fn reply(
    State(claims): State<Arc<Mutex<Vec<Uuid>>>>,
    Json(request): Json<Request>,
) -> (StatusCode, [(&'static str, &'static str); 2], &'static str) {
    let Request::Claim { claim, .. } = request else {
        panic!("expected claim")
    };
    let mut claims = claims.lock();
    claims.push(claim);
    let status = match claims.len() {
        1 => StatusCode::OK,
        2 => StatusCode::SERVICE_UNAVAILABLE,
        _ => StatusCode::CONFLICT,
    };
    (
        status,
        [
            ("content-type", "application/json"),
            (baylee_tower_api::VERSION_HEADER, baylee_tower_api::VERSION),
        ],
        "{",
    )
}

#[tokio::test]
async fn corrupted_or_lost_authority_replies_reuse_the_claim_on_the_wire() {
    let claims = Arc::new(Mutex::new(Vec::new()));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let router = Router::new()
        .route(baylee_tower_api::host::CONTROL_PATH, post(reply))
        .with_state(claims.clone());
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let mut server = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async {
                let _ = stopped.await;
            })
            .await
    });
    let observed = tokio::spawn(async move {
        let origin = format!("http://{address}");
        let tower = Tower {
            client: TowerHost::new(&origin, Token::generate().unwrap(), true).unwrap(),
            origin,
            starts: tokio::sync::Mutex::new(()),
            pending: Mutex::new(pending::Claims::default()),
            games: Mutex::new(BTreeMap::new()),
        };
        let account = Uuid::now_v7();
        let run = Uuid::now_v7();
        assert!(matches!(
            tower.claim(account, run).await,
            Err(Error::Rejected)
        ));
        assert!(matches!(
            tower.claim(account, run).await,
            Err(Error::Unavailable)
        ));
        assert!(matches!(
            tower.claim(account, run).await,
            Err(Error::Conflict)
        ));
        assert!(matches!(
            tower.claim(account, run).await,
            Err(Error::Conflict)
        ));
        let claims = claims.lock();
        assert_eq!(claims.len(), 4);
        assert_eq!(claims[0], claims[1]);
        assert_eq!(claims[1], claims[2]);
        assert_ne!(claims[2], claims[3]);
    })
    .await;
    let _ = stop.send(());
    let closed = tokio::time::timeout(Duration::from_secs(5), &mut server).await;
    if closed.is_err() {
        server.abort();
        let _ = server.await;
    }
    assert!(
        matches!(closed, Ok(Ok(Ok(())))),
        "owned authority fixture stopped"
    );
    assert!(matches!(
        tokio::time::timeout(
            Duration::from_secs(1),
            tokio::net::TcpStream::connect(address)
        )
        .await,
        Ok(Err(_))
    ));
    observed.expect("claim retry transport scenario passed");
}
