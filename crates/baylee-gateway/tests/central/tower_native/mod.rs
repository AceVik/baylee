//! Real native Tower attachment, global authority and independent adjudication.
use super::{Endpoint, Served, common, fresh};
use baylee_account::Token;
use baylee_protocol::execution::Identity;
use baylee_tower_service::{Artifact, Hosts, Players, Queue, QueueSummary};
use baylee_tower_store::{Principal, Role, Store};
use std::{
    net::{SocketAddr, TcpStream},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};
use tokio::{runtime::Runtime, sync::watch, task::JoinHandle};
use uuid::Uuid;

mod native;
mod recovery;
mod story;
#[path = "../../../../baylee-tower-store/tests/support/mod.rs"]
mod support;

pub(super) struct Fixture {
    pub origin: String,
    binary: PathBuf,
    identity: Identity,
    cohort: Uuid,
    host: Token,
    sandbox: support::Sandbox,
    stop: watch::Sender<bool>,
    server: JoinHandle<()>,
    worker: JoinHandle<Result<QueueSummary, baylee_tower_service::Error>>,
    address: SocketAddr,
}

async fn operator(store: &Store, name: &str, role: Role) -> (Token, Principal) {
    let token = Token::generate().unwrap();
    store
        .provision_operator(name, role, token.digest())
        .await
        .unwrap();
    let principal = store.authenticate_operator(token.digest()).await.unwrap();
    (token, principal)
}

impl Fixture {
    pub(super) async fn optional(endpoint: &Endpoint) -> Option<Self> {
        let binary = PathBuf::from(std::env::var_os("BAYLEE_TOWER_V2_ENGINE")?);
        assert!(binary.is_absolute(), "explicit local native Tower artifact");
        let artifact = Artifact::probe(&binary).await.unwrap();
        let identity = artifact.identity().clone();
        let sandbox = support::Sandbox::open().await;
        let mut publication = support::publication();
        let forest = baylee_cards::decks::by_name("Forest").unwrap();
        publication.catalog.cards.truncate(1);
        publication.catalog.cards[0].card = forest;
        for starter in &mut publication.starters {
            starter.deck[0].card = forest;
        }
        publication.cohort =
            baylee_tower::v2::package::Cohort::new(identity.clone(), &publication.catalog).unwrap();
        let cohort = sandbox.store.publish(&publication).await.unwrap();
        let (_, publisher) =
            operator(&sandbox.store, "native fixture publisher", Role::Publisher).await;
        sandbox
            .store
            .publish_house(
                &publisher,
                cohort,
                1,
                1,
                &baylee_tower::v2::package::Build {
                    deck: publication.starters[0].deck.clone(),
                    active: Vec::new(),
                    potions: [0; 2],
                },
            )
            .await
            .unwrap();
        let (host, _) = operator(&sandbox.store, "native fixture gateway", Role::Host).await;
        let (_, verifier) =
            operator(&sandbox.store, "native fixture verifier", Role::Verifier).await;
        let (_, supervisor) = operator(
            &sandbox.store,
            "native fixture supervisor",
            Role::Supervisor,
        )
        .await;
        let queue = Queue::new(
            sandbox.store.clone(),
            verifier,
            supervisor,
            artifact,
            baylee_tower::v2::verification::Resources::default(),
        )
        .unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let origin = format!("http://{address}");
        let players =
            Players::new(sandbox.store.clone(), endpoint, &origin, 64 * 1024 * 1024).unwrap();
        let router = Arc::new(players)
            .router()
            .merge(Arc::new(Hosts::new(sandbox.store.clone())).router());
        let (stop, mut stopped) = watch::channel(false);
        let mut work_stopped = stopped.clone();
        let server = tokio::spawn(async move {
            axum::serve(listener, router)
                .with_graceful_shutdown(async move {
                    let _ = stopped.changed().await;
                })
                .await
                .unwrap();
        });
        let worker = tokio::spawn(async move {
            queue
                .serve(cohort, async move {
                    let _ = work_stopped.changed().await;
                })
                .await
        });
        Some(Self {
            origin,
            binary,
            identity,
            cohort,
            host,
            sandbox,
            stop,
            server,
            worker,
            address,
        })
    }

    pub(super) fn environment(&self) -> Vec<(&'static str, String)> {
        vec![
            ("BAYLEE_TOWER_ORIGIN", self.origin.clone()),
            ("BAYLEE_TOWER_HOST_TOKEN", self.host.hex().to_string()),
            ("BAYLEE_TOWER_DEVELOPMENT", "1".into()),
        ]
    }

    pub(super) async fn close(self) {
        let _ = self.stop.send(true);
        let server = tokio::time::timeout(Duration::from_secs(10), self.server).await;
        let worker = tokio::time::timeout(Duration::from_secs(10), self.worker).await;
        self.sandbox.close().await;
        server.expect("Tower HTTP shutdown").unwrap();
        let summary = worker
            .expect("independent verifier shutdown")
            .unwrap()
            .unwrap();
        let error = TcpStream::connect_timeout(&self.address, Duration::from_secs(5))
            .expect_err("owned Tower port closed");
        assert_eq!(error.kind(), std::io::ErrorKind::ConnectionRefused);
        println!(
            "Tower authority closed; settlements: {}, quarantines: {}, recovered executions: {}, execution quarantines: {}",
            summary.settled, summary.quarantined, summary.recovered, summary.execution_quarantined
        );
    }
}

pub(super) fn exercise(runtime: &Runtime, served: &Served) {
    story::run(runtime, served);
}
