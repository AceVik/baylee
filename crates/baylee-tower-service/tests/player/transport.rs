use axum::{
    Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
};
use bytes::Bytes;
use central_api::{Token, tower_access::Identity};
use http_body_util::{BodyExt as _, Full, Limited};
use hyper_util::{
    client::legacy::{Client as HttpClient, connect::HttpConnector},
    rt::TokioExecutor,
};
use std::{
    net::SocketAddr,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU16, AtomicUsize, Ordering},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::oneshot,
    task::JoinHandle,
};
use uuid::Uuid;

pub struct Server {
    address: SocketAddr,
    stop: oneshot::Sender<()>,
    task: JoinHandle<()>,
}
impl Server {
    pub async fn start(router: Router) -> Self {
        Self::bound(TcpListener::bind("127.0.0.1:0").await.unwrap(), router)
    }
    pub fn bound(listener: TcpListener, router: Router) -> Self {
        let address = listener.local_addr().unwrap();
        let (stop, closed) = oneshot::channel();
        let task = tokio::spawn(async move {
            axum::serve(listener, router)
                .with_graceful_shutdown(async {
                    let _ = closed.await;
                })
                .await
                .unwrap();
        });
        Self {
            address,
            stop,
            task,
        }
    }
    pub fn origin(&self) -> String {
        format!("http://{}", self.address)
    }
    pub async fn close(mut self) -> bool {
        let _ = self.stop.send(());
        let finished = if let Ok(result) =
            tokio::time::timeout(Duration::from_secs(10), &mut self.task).await
        {
            result.is_ok()
        } else {
            self.task.abort();
            let _ = self.task.await;
            false
        };
        let closed = matches!(
            tokio::time::timeout(Duration::from_secs(2), TcpStream::connect(self.address)).await,
            Ok(Err(_))
        );
        finished && closed
    }
}

pub struct Central {
    pub tokens: [Token; 2],
    pub accounts: [Uuid; 2],
    pub origin: Mutex<String>,
    pub status: AtomicU16,
    pub wrong_origin: AtomicBool,
    pub calls: AtomicUsize,
}
impl Central {
    pub fn new() -> Self {
        Self {
            tokens: [Token::generate().unwrap(), Token::generate().unwrap()],
            accounts: [Uuid::now_v7(), Uuid::now_v7()],
            origin: Mutex::new(String::new()),
            status: AtomicU16::new(200),
            wrong_origin: AtomicBool::new(false),
            calls: AtomicUsize::new(0),
        }
    }
    pub fn router(self: &Arc<Self>) -> Router {
        Router::new()
            .route("/v1/native", post(identity))
            .with_state(self.clone())
    }
}

async fn identity(
    State(central): State<Arc<Central>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    central.calls.fetch_add(1, Ordering::SeqCst);
    assert_eq!(
        headers.get(central_api::PROTOCOL_HEADER).unwrap(),
        central_api::PROTOCOL_VERSION
    );
    assert!(matches!(
        central_api::decode(&body),
        Ok(central_api::wire::Request::TowerIdentity {})
    ));
    let status = central.status.load(Ordering::SeqCst);
    if status != 200 {
        return (
            StatusCode::from_u16(status).unwrap(),
            [("location", "http://127.0.0.1:1/credential-trap")],
        )
            .into_response();
    }
    let Some(token) = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| Token::from_bearer(v).ok())
    else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let Some(index) = central
        .tokens
        .iter()
        .position(|known| known.digest() == token.digest())
    else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let expires_at = i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    )
    .unwrap()
        + 300;
    let response = central_api::wire::Response::TowerIdentity {
        identity: Identity {
            origin: if central.wrong_origin.load(Ordering::SeqCst) {
                "http://127.0.0.1:1".into()
            } else {
                central.origin.lock().unwrap().clone()
            },
            account: *central.accounts[index].as_bytes(),
            display_name: Some("SyntheticTower".into()),
            tag: Some(1234),
            expires_at,
        },
    };
    (
        [("content-type", "application/json")],
        serde_json::to_vec(&response).unwrap(),
    )
        .into_response()
}

pub struct Client {
    http: HttpClient<HttpConnector, Full<Bytes>>,
    origin: String,
}
impl Client {
    pub fn origin(&self) -> &str {
        &self.origin
    }
    pub fn new(origin: String) -> Self {
        Self {
            http: HttpClient::builder(TokioExecutor::new()).build_http(),
            origin,
        }
    }
    pub async fn send(&self, token: &Token, request: &baylee_tower_api::Request) -> (u16, Vec<u8>) {
        self.raw(Some(token), serde_json::to_vec(request).unwrap(), &[])
            .await
    }
    pub async fn raw(
        &self,
        token: Option<&Token>,
        body: Vec<u8>,
        extra: &[(&str, &str)],
    ) -> (u16, Vec<u8>) {
        self.route(
            token,
            body,
            extra,
            baylee_tower_api::PLAYER_PATH,
            "application/json",
        )
        .await
    }
    pub async fn route(
        &self,
        token: Option<&Token>,
        body: Vec<u8>,
        extra: &[(&str, &str)],
        path: &str,
        kind: &str,
    ) -> (u16, Vec<u8>) {
        let mut request = hyper::Request::post(format!("{}{path}", self.origin))
            .header("content-type", kind)
            .header(baylee_tower_api::VERSION_HEADER, baylee_tower_api::VERSION);
        if let Some(token) = token {
            request = request.header("authorization", token.bearer().as_str());
        }
        for (name, value) in extra {
            request = request.header(*name, *value);
        }
        let response = tokio::time::timeout(
            Duration::from_secs(10),
            self.http
                .request(request.body(Full::new(Bytes::from(body))).unwrap()),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(response.headers().get("cache-control").unwrap(), "no-store");
        let status = response.status().as_u16();
        let bytes = Limited::new(response.into_body(), baylee_tower_api::MAX_RESPONSE_BYTES)
            .collect()
            .await
            .unwrap()
            .to_bytes();
        (status, bytes.to_vec())
    }
}

pub fn run_response(response: (u16, Vec<u8>)) -> Box<baylee_tower_api::Run> {
    assert_eq!(response.0, 200);
    // Evidence/worker fields must not become an accidental player protocol.
    let json = String::from_utf8(response.1).unwrap();
    for forbidden in [
        "execution",
        "prefix",
        "binding",
        "policy",
        "shuffle",
        "lease_until",
    ] {
        assert!(!json.contains(&format!("\"{forbidden}\"")));
    }
    let baylee_tower_api::Response::Run { run } = serde_json::from_str(&json).unwrap() else {
        panic!("owner run expected");
    };
    run
}
