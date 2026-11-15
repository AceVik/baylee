#![forbid(unsafe_code)]
use baylee_core::ids::CardIndex;
use baylee_protocol::execution::{Identity, Variant};
use baylee_tower::{
    cards::{Card, Catalog, Collection, Stack},
    v2::{generation, package::Cohort},
};
use baylee_tower_store::{Publication, Store};
use sea_orm::{ConnectOptions, ConnectionTrait, Database, DatabaseConnection};
use std::time::Duration;
use uuid::Uuid;

pub struct Sandbox {
    pub store: Store,
    pub db: DatabaseConnection,
    pub schema: String,
}

impl Sandbox {
    pub async fn open() -> Self {
        let connection = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://baylee:baylee@127.0.0.1:5432/baylee".into());
        let parsed = url::Url::parse(&connection)
            .map_err(|_| ())
            .expect("valid local test database URL");
        let local = match parsed.host() {
            Some(url::Host::Domain(host)) => {
                host == "localhost"
                    || host
                        .parse::<std::net::IpAddr>()
                        .is_ok_and(|ip| ip.is_loopback())
            }
            Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
            Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
            None => false,
        };
        assert!(
            local
                && parsed.query().is_none()
                && parsed.fragment().is_none()
                && matches!(parsed.scheme(), "postgres" | "postgresql"),
            "tests require an explicit loopback PostgreSQL URL without host overrides"
        );
        let mut options = ConnectOptions::new(connection);
        options
            .max_connections(4)
            .min_connections(1)
            .sqlx_logging(false)
            .connect_timeout(Duration::from_secs(5))
            .acquire_timeout(Duration::from_secs(5));
        let db = Database::connect(options)
            .await
            .map_err(|_| ())
            .expect("local synthetic PostgreSQL database is reachable");
        let schema = format!("t_tower_{}", Uuid::now_v7().simple());
        let store = Store::new(db.clone(), &schema).unwrap();
        if let Err(error) = store.migrate().await {
            let _ = db
                .execute_unprepared(&format!("DROP SCHEMA IF EXISTS \"{schema}\" CASCADE"))
                .await;
            db.close()
                .await
                .map_err(|_| ())
                .expect("failed fixture pool closes");
            panic!("isolated Tower migration failed: {error}");
        }
        Self { store, db, schema }
    }

    pub async fn close(self) {
        let result = self
            .db
            .execute_unprepared(&format!("DROP SCHEMA \"{}\" CASCADE", self.schema))
            .await;
        let closed = self.db.close().await;
        result.map_err(|_| ()).expect("owned test schema removed");
        closed.map_err(|_| ()).expect("owned test pool closed");
    }
}

pub fn publication() -> Publication {
    let mut cards = vec![Card {
        card: CardIndex::new(0),
        price: 0,
        basic_land: true,
        run: None,
    }];
    cards.extend((1..=8).map(|id| Card {
        card: CardIndex::new(id),
        price: 2,
        basic_land: false,
        run: None,
    }));
    let catalog = Catalog { cards };
    let starter = Collection {
        formation: None,
        owned: Vec::new(),
        deck: vec![Stack {
            card: CardIndex::new(0),
            copies: 40,
        }],
    };
    let mut identity = Identity::native(Variant::Tower, "database-test", "synthetic-registry", 21);
    identity.rules = "tower-4".into();
    Publication {
        cohort: Cohort::for_generation(identity, &catalog, 3).unwrap(),
        catalog,
        starters: vec![starter.clone(), starter.clone(), starter],
    }
}

#[allow(dead_code)] // only the retained-run integration binaries use this fixture
pub fn retained_publication() -> Publication {
    let mut publication = publication();
    let mut identity = publication.cohort.executor;
    identity.rules = "tower-2".into();
    identity.ai = "tower-house-19".into();
    publication.cohort =
        Cohort::for_generation(identity, &publication.catalog, generation::original()).unwrap();
    publication
}
