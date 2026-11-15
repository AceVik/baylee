//! Owned Central and gateway listeners for the synthetic integration fixture.
use super::{BridgeFile, Endpoint, Token, aliases, bridge_file, common, token_keys};
use central_crypto::opaque::server::ServerKeys;
use central_http::DevelopmentServer;
use central_identity::IdentityService;
use central_postgres::{AccountStore, LegacyStore};
use std::{
    net::{SocketAddr, TcpStream},
    sync::Arc,
    time::Duration,
};
use tokio::{runtime::Runtime, sync::oneshot, task::JoinHandle};

/// A Central development server over the fixture's store, its identity
/// service (with `terms` in force) and a gateway bridged to it.
pub(super) struct Served {
    pub(super) endpoint: Endpoint,
    pub(super) gateway: common::Gateway,
    pub(super) origin: String,
    pub(super) service: Arc<IdentityService>,
    /// The bridge credential the gateway holds, for a second bridge-mode
    /// gateway (`central/official.rs`).
    pub(super) bridge: Token,
    pub(super) secret_file: BridgeFile,
    pub(super) stop: oneshot::Sender<()>,
    pub(super) serving: JoinHandle<()>,
    pub(super) address: SocketAddr,
    pub(super) gateway_address: SocketAddr,
    pub(super) tower: Option<super::tower_native::Fixture>,
}

pub(super) fn serve(
    runtime: &Runtime,
    store: &AccountStore,
    legacy: &LegacyStore,
    label: &str,
    terms: Option<baylee_account::terms::TermsDocument>,
) -> Served {
    let native_listen = if std::env::var("BAYLEE_CENTRAL_DEMO").as_deref() == Ok("1") {
        let port = std::env::var("BAYLEE_CENTRAL_DEMO_NATIVE_PORT")
            .ok()
            .map_or(0, |port| port.parse::<u16>().expect("demo native port"));
        format!("127.0.0.1:{port}")
    } else {
        "127.0.0.1:0".into()
    };
    let server = runtime
        .block_on(DevelopmentServer::bind(
            central_policy::config::Config::from_pairs([
                ("BAYLEE_CENTRAL_LISTEN", native_listen.as_str()),
                ("BAYLEE_CENTRAL_REQUEST_BYTES", "1056768"),
            ])
            .unwrap(),
        ))
        .unwrap();
    let address = server.local_addr();
    let endpoint = Endpoint::development(&format!("http://{address}")).unwrap();
    let bridge = Token::generate().unwrap();
    let secret_file = bridge_file(&endpoint, &bridge);
    let tower = runtime.block_on(super::tower_native::Fixture::optional(&endpoint));
    let mut environment = vec![
        (
            "BAYLEE_CENTRAL_CONFIG_FILE",
            secret_file.0.to_string_lossy().into_owned(),
        ),
        ("BAYLEE_BIND", "127.0.0.1".into()),
        ("BAYLEE_REGISTRATION", "open".into()),
    ];
    if let Some(tower) = &tower {
        environment.extend(tower.environment());
    }
    let gateway = common::spawn_gateway_with(label, &environment);
    let gateway_address = format!("127.0.0.1:{}", gateway.port).parse().unwrap();
    let origin = format!("http://{gateway_address}");
    let service = Arc::new(
        IdentityService::new(
            store.clone(),
            &endpoint,
            ServerKeys::generate(),
            &bridge,
            &origin,
            token_keys(),
        )
        .unwrap()
        .with_aliases(vec![aliases::ALIAS.into()])
        .unwrap()
        .with_terms(terms)
        .with_tower(tower.as_ref().map(|tower| tower.origin.as_str()))
        .unwrap()
        .with_registration_challenge(0, central_crypto::proof::MIN_MEMORY, Duration::ZERO)
        .with_legacy(legacy.clone()),
    );
    let (stop, stopped) = oneshot::channel();
    let served_endpoint = endpoint.clone();
    let identity = Arc::clone(&service);
    let serving = runtime.spawn(async move {
        server
            .run_with_identity(&served_endpoint, identity, async {
                let _ = stopped.await;
            })
            .await
            .unwrap();
    });
    common::central_ready(gateway.port);
    Served {
        endpoint,
        gateway,
        origin,
        service,
        bridge,
        secret_file,
        stop,
        serving,
        address,
        gateway_address,
        tower,
    }
}

impl Served {
    /// Stops the gateway and Central, and checks both listeners are gone.
    pub(super) fn close(self, runtime: &Runtime) {
        drop(self.gateway);
        drop(self.secret_file);
        if let Some(tower) = self.tower {
            runtime.block_on(tower.close());
        }
        let _ = self.stop.send(());
        runtime.block_on(async {
            tokio::time::timeout(Duration::from_secs(5), self.serving)
                .await
                .unwrap()
                .unwrap();
        });
        for closed in [self.address, self.gateway_address] {
            // Windows may report loopback refusal after about two seconds; a
            // shorter timeout is not evidence that a listener remains or closed.
            let error = TcpStream::connect_timeout(&closed, Duration::from_secs(5))
                .expect_err("owned service listener must be closed");
            assert_eq!(error.kind(), std::io::ErrorKind::ConnectionRefused);
        }
    }
}
