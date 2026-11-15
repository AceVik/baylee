//! Dedicated role boundaries and immutable bootstrap content on real PostgreSQL.
#![forbid(unsafe_code)]
mod support;

use baylee_protocol::execution::Identity;
use baylee_tower::v2::package::Build;
use baylee_tower_store::{Error, Role, Store};
use sea_orm::ConnectionTrait;
use support::{Sandbox, publication};
use uuid::Uuid;

#[tokio::test]
async fn only_live_dedicated_roles_can_advertise_or_publish_house_opponents() {
    let sandbox = Sandbox::open().await;
    let store = sandbox.store.clone();
    let db = sandbox.db.clone();
    let schema = sandbox.schema.clone();
    let observed = tokio::spawn(async move {
        let published = publication();
        let cohort = store.publish(&published).await.unwrap();
        let host_id = store
            .provision_operator("synthetic host", Role::Host, [1; 32])
            .await
            .unwrap();
        assert_eq!(host_id.get_version_num(), 7);
        assert!(matches!(
            store.authenticate_operator([0; 32]).await,
            Err(Error::NotFound)
        ));
        let host = store.authenticate_operator([1; 32]).await.unwrap();
        assert_eq!(host.id(), host_id);
        assert_eq!(host.role(), Role::Host);
        assert!(
            !store
                .host_supports(host_id, &published.cohort.executor)
                .await
                .unwrap()
        );
        let identities = [published.cohort.executor.clone()];
        store
            .advertise(&host, &identities, &identities)
            .await
            .unwrap();
        assert!(store.host_supports(host_id, &identities[0]).await.unwrap());
        let mut other = identities[0].clone();
        other.build.push('x');
        assert!(!store.host_supports(host_id, &other).await.unwrap());
        assert!(
            store
                .advertise(&host, &[identities[0].clone(), identities[0].clone()], &[])
                .await
                .is_err()
        );
        let build = Build {
            formation: None,
            deck: published.starters[0].deck.clone(),
            active: Vec::new(),
            potions: [0; 2],
        };
        assert_eq!(
            store.publish_house(&host, cohort, 1, 1, &build).await,
            Err(Error::NotFound)
        );
        publication_roles(&store, cohort, &build, &identities).await;
        let mut updates = Vec::new();
        for _ in 0..4 {
            let store = store.clone();
            let host = host.clone();
            let identities = identities.clone();
            updates.push(tokio::spawn(async move {
                store.advertise(&host, &identities, &identities).await
            }));
        }
        for update in updates {
            update.await.unwrap().unwrap();
        }
        db.execute_unprepared(&format!(
            "UPDATE \"{schema}\".operators SET enabled=false WHERE id='{host_id}'"
        ))
        .await
        .unwrap();
        assert!(matches!(
            store.authenticate_operator([1; 32]).await,
            Err(Error::NotFound)
        ));
        assert_eq!(
            store.advertise(&host, &identities, &identities).await,
            Err(Error::NotFound)
        );
        assert_eq!(
            store.host_supports(host_id, &identities[0]).await,
            Err(Error::NotFound)
        );
    })
    .await;
    sandbox.close().await;
    observed.expect("operator scenario completed without assertion failure");
}

async fn publication_roles(store: &Store, cohort: Uuid, build: &Build, identities: &[Identity]) {
    for (index, role) in [
        Role::Verifier,
        Role::Supervisor,
        Role::Publisher,
        Role::Courier,
    ]
    .into_iter()
    .enumerate()
    {
        let digest = [u8::try_from(index + 2).unwrap(); 32];
        store
            .provision_operator("synthetic role", role, digest)
            .await
            .unwrap();
        let actor = store.authenticate_operator(digest).await.unwrap();
        assert_eq!(
            store.advertise(&actor, identities, identities).await,
            Err(Error::NotFound)
        );
        let result = store.publish_house(&actor, cohort, 1, 1, build).await;
        if role == Role::Publisher {
            result.unwrap();
            store
                .publish_house(&actor, cohort, 1, 1, build)
                .await
                .unwrap();
            let mut changed = build.clone();
            changed.potions = [1, 0];
            assert_eq!(
                store.publish_house(&actor, cohort, 1, 1, &changed).await,
                Err(Error::Conflict)
            );
            assert!(
                store
                    .publish_house(&actor, cohort, 13, 1, build)
                    .await
                    .is_err()
            );
        } else {
            assert_eq!(result, Err(Error::NotFound));
        }
    }
}
