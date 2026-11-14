//! Synthetic evidence tests transport/atomicity, never claim that these are legal game records.
use super::transport::Client;
use baylee_tower_api::{
    Run,
    host::{self as api, Batch, Request, Response},
};
use baylee_tower_store::{Role, Store};
use central_api::Token;
use uuid::Uuid;

async fn request(client: &Client, token: &Token, request: &Request) -> (u16, Vec<u8>) {
    client
        .route(
            Some(token),
            serde_json::to_vec(request).unwrap(),
            &[],
            api::CONTROL_PATH,
            "application/json",
        )
        .await
}

async fn append(client: &Client, token: &Token, bytes: Vec<u8>) -> (u16, Vec<u8>) {
    client
        .route(
            Some(token),
            bytes,
            &[],
            api::RECORD_PATH,
            "application/octet-stream",
        )
        .await
}

async fn other_operators(store: &Store, client: &Client, claim: &Request) -> Token {
    let wrong = Token::generate().unwrap();
    store
        .provision_operator("other host", Role::Host, wrong.digest())
        .await
        .unwrap();
    assert_eq!(request(client, &wrong, claim).await.0, 404);
    let verifier = Token::generate().unwrap();
    store
        .provision_operator("verification only", Role::Verifier, verifier.digest())
        .await
        .unwrap();
    assert_eq!(request(client, &verifier, claim).await.0, 401);
    wrong
}

pub async fn run(store: &Store, client: &Client, player: &Token, host: &Token, run: &Run) {
    let sdk = baylee_account::TowerHost::new(
        client.origin(),
        Token::from_secret_bytes(host.to_secret_bytes()),
        true,
    )
    .unwrap();
    let claim = Request::Claim {
        run: run.id,
        claim: Uuid::now_v7(),
    };
    assert_eq!(request(client, player, &claim).await.0, 401);
    assert_eq!(
        client
            .send(host, &baylee_tower_api::Request::Run { run: run.id })
            .await
            .0,
        401
    );
    let wrong = other_operators(store, client, &claim).await;
    let response = request(client, host, &claim).await;
    assert_eq!(response.0, 200);
    let Response::Claimed { game } = serde_json::from_slice(&response.1).unwrap() else {
        panic!("private claim");
    };
    assert_eq!(game.account.get_version_num(), 7);
    assert_eq!(store.get(game.account, run.id).await.unwrap().id, run.id);
    assert_eq!(
        request(client, host, &claim).await,
        response,
        "claim retries retain every private seed/binding"
    );
    assert!(
        matches!(sdk.control(&claim).await.unwrap(), Response::Claimed { game: repeated } if repeated.lease == game.lease && repeated.account == game.account)
    );
    assert_eq!(
        request(client, host, &Request::Renew { lease: game.lease })
            .await
            .0,
        200
    );
    let batch = Batch {
        lease: game.lease,
        expected: game.prefix,
        chunks: vec![b"first ".to_vec(), b"complete operation\n".to_vec()],
    };
    assert_eq!(append(client, player, batch.encode().unwrap()).await.0, 401);
    assert_eq!(append(client, &wrong, batch.encode().unwrap()).await.0, 404);
    let response = append(client, host, batch.encode().unwrap()).await;
    assert_eq!(response.0, 200);
    assert_eq!(
        append(client, host, batch.encode().unwrap()).await,
        response,
        "whole-batch retry returns original ACK"
    );
    let Response::Acknowledged { acknowledgement } = serde_json::from_slice(&response.1).unwrap()
    else {
        panic!("committed head");
    };
    assert_eq!(acknowledgement.prefix.chunks, 2);
    assert_eq!(sdk.append(&batch).await.unwrap(), acknowledgement);
    read_chunks(client, host, &batch).await;
    let mut tampered = batch;
    tampered.chunks[0][0] ^= 1;
    assert_eq!(
        append(client, host, tampered.encode().unwrap()).await.0,
        409
    );
    let mut truncated = tampered.encode().unwrap();
    truncated.pop();
    assert_eq!(append(client, host, truncated).await.0, 400);
    tampered.lease.fence += 1;
    assert_eq!(
        append(client, host, tampered.encode().unwrap()).await.0,
        409
    );
    let seal = Request::Seal {
        lease: game.lease,
        expected: acknowledgement.prefix,
    };
    let sealed = request(client, host, &seal).await;
    assert_eq!(sealed.0, 200);
    assert_eq!(request(client, host, &seal).await, sealed);
    let state = store.get(game.account, run.id).await.unwrap();
    assert_eq!(state.state.phase, baylee_tower::v2::run::Phase::Verifying);
    assert!(
        state.state.history.is_empty(),
        "host seal cannot grant a rules outcome"
    );
    assert_eq!(
        state.state.economy, run.economy,
        "host seal cannot award currency"
    );
}

async fn read_chunks(client: &Client, host: &Token, batch: &Batch) {
    for (index, expected) in batch.chunks.iter().enumerate() {
        let response = request(
            client,
            host,
            &Request::Read {
                lease: batch.lease,
                index: index as u64,
            },
        )
        .await;
        assert_eq!(response.0, 200);
        let Response::Chunk { chunk } = serde_json::from_slice(&response.1).unwrap() else {
            panic!("private chunk");
        };
        assert_eq!(&chunk.data, expected);
    }
}
