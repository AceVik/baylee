//! The official gateway in Central's directory (baylee-central
//! `docs/gateway-registry.md` §"Discovery, heartbeats and administrators"):
//! the operator lists a row for the bridge's origin, and a bridge-mode gateway
//! with `metadata` beats for it with the bridge credential, so it goes online
//! with that metadata and the gateway's own version. Synthetic data only.
use super::{CentralHttp, Endpoint, Token, common, config_file, sdk};
use baylee_account::gateway::{GatewayListing, GatewayMetadata};
use sea_orm::{ConnectionTrait as _, DatabaseConnection, DbBackend, Statement};
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;

fn listed(http: &CentralHttp, origin: &str) -> Option<GatewayListing> {
    sdk::gateway_directory(http, None)
        .unwrap()
        .into_iter()
        .find(|entry| entry.origin == origin)
}

pub(super) fn run(
    runtime: &Runtime,
    operator: &DatabaseConnection,
    endpoint: &Endpoint,
    bridge: &Token,
    origin: &str,
) {
    let http = CentralHttp::new(endpoint.clone());
    assert!(
        listed(&http, origin).is_none(),
        "not listed before the operator"
    );
    // The operator's statements (baylee-central `docs/operations.md`
    // §"The official gateway"), for this run's bridge origin.
    runtime.block_on(async {
        operator
            .execute_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "INSERT INTO central_identity.gateways (official, origin, name, version, website)
                 VALUES (true, $1, 'Baylee', '0', $2)",
                [origin.into(), format!("{origin}/").into()],
            ))
            .await
            .unwrap();
        operator
            .execute_unprepared(
                "INSERT INTO central_identity.gateway_approvals (gateway_id)
                 SELECT id FROM central_identity.gateways WHERE official",
            )
            .await
            .unwrap();
    });
    // Listed as the operator wrote it, and offline: the bridge already
    // running here has no `metadata`, so it sends no heartbeat.
    let entry = listed(&http, origin).expect("listed once approved");
    assert_eq!(entry.metadata.name, "Baylee");
    assert!(!entry.online);
    let metadata = GatewayMetadata {
        name: "Synthetic official gateway".into(),
        version: "fixture".into(),
        website: format!("{origin}/"),
    };
    let config = config_file(&serde_json::json!({
        "endpoint": endpoint.origin(), "bridge": bridge, "origin": origin, "metadata": metadata,
    }));
    let gateway = common::spawn_gateway_with(
        "central-official",
        &[
            (
                "BAYLEE_CENTRAL_CONFIG_FILE",
                config.0.to_string_lossy().into_owned(),
            ),
            // Blank: the default terms, which the bridge's other gateway
            // publishes too, so the two agree.
            ("BAYLEE_TERMS_PATH", String::new()),
        ],
    );
    // The first heartbeat goes out at start; Central marks the entry online.
    let deadline = Instant::now() + Duration::from_secs(20);
    let entry = loop {
        if let Some(entry) = listed(&http, origin).filter(|entry| entry.online) {
            break entry;
        }
        assert!(
            Instant::now() < deadline,
            "the official gateway's heartbeat never arrived"
        );
        std::thread::sleep(Duration::from_millis(100));
    };
    assert_eq!(entry.metadata.name, "Synthetic official gateway");
    assert_eq!(entry.metadata.version, env!("CARGO_PKG_VERSION"));
    assert_eq!(entry.metadata.website, format!("{origin}/"));
    drop(gateway);
    drop(config);
    // Gone again, for what runs after this.
    runtime
        .block_on(
            operator.execute_unprepared("DELETE FROM central_identity.gateways WHERE official"),
        )
        .unwrap();
}
