//! The reviewed runtime grants (`runtime_grants.sql`) carry a whole run's
//! runtime path, and refuse every administrative write.
use super::{Sandbox, entropy, operator, publication};
use baylee_tower::v2::{
    package::Build,
    run::{Command, Outcome, Phase, Settlement},
};
use baylee_tower_api::ranking::{Lane, Name};
use baylee_tower_store::{CONSENT_REVISION, Error, Reserve, Role, Start, Store};
use sea_orm::{ConnectOptions, ConnectionTrait as _, Database, DatabaseConnection};
use uuid::Uuid;

async fn runtime_connection(sandbox: &Sandbox, role: &str, password: &str) -> DatabaseConnection {
    let base = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://baylee:baylee@127.0.0.1:5432/baylee".into());
    let mut url = url::Url::parse(&base).unwrap();
    url.set_username(role).unwrap();
    url.set_password(Some(password)).unwrap();
    let mut options = ConnectOptions::new(url.as_str());
    options.max_connections(2).sqlx_logging(false);
    let _ = sandbox;
    Database::connect(options)
        .await
        .map_err(|_| ())
        .expect("the runtime login connects")
}

#[tokio::test]
async fn a_whole_run_plays_under_the_runtime_grants_and_administration_is_refused() {
    let sandbox = Sandbox::open().await;
    let role = format!("t_tower_runtime_{}", Uuid::now_v7().simple());
    let password = format!("{:032x}", Uuid::now_v7().as_u128());
    sandbox
        .db
        .execute_unprepared(&format!("CREATE ROLE \"{role}\" LOGIN PASSWORD '{password}'"))
        .await
        .map_err(|_| ())
        .expect("synthetic runtime login created");
    let owner = sandbox.store.clone();
    let observed = {
        let role = role.clone();
        let schema = sandbox.schema.clone();
        let runtime_db = runtime_connection(&sandbox, &role, &password).await;
        tokio::spawn(async move {
            // Before any grant the runtime reads nothing at all.
            let runtime = Store::new(runtime_db.clone(), &schema).unwrap();
            assert!(runtime.ready().await.is_err());
            owner.grant_runtime(&role).await.unwrap();
            // A renewal is exact and repeatable.
            owner.grant_runtime(&role).await.unwrap();
            runtime.ready().await.unwrap();
            runtime.runtime_least_privilege().await.unwrap();
            assert_eq!(
                owner.runtime_least_privilege().await,
                Err(Error::Privileged),
                "the schema owner never passes for a runtime role"
            );
            // A runtime role cannot renew or widen its own grants.
            assert!(runtime.grant_runtime(&role).await.is_err());

            // The owner publishes and provisions; the runtime serves.
            let published = publication();
            let cohort = owner.publish(&published).await.unwrap();
            let publisher = operator(&owner, Role::Publisher, 31).await;
            let host = operator(&owner, Role::Host, 32).await;
            let verifier = operator(&owner, Role::Verifier, 33).await;
            let courier = operator(&owner, Role::Courier, 34).await;
            owner
                .approve_host(host.id(), "http://127.0.0.1:29881", true)
                .await
                .unwrap();
            for (stage, archetype) in (1..=baylee_tower::v2::MAX_STAGES).flat_map(|stage| {
                (1..=u8::try_from(published.starters.len()).unwrap()).map(move |a| (stage, a))
            }) {
                let starter = &published.starters[usize::from(archetype - 1)];
                owner
                    .publish_house(
                        &publisher,
                        cohort,
                        stage,
                        archetype,
                        &Build {
                            formation: starter.formation.clone(),
                            deck: starter.deck.clone(),
                            active: Vec::new(),
                            potions: [0; 2],
                        },
                    )
                    .await
                    .unwrap();
            }
            let host = runtime.authenticate_operator([32; 32]).await.unwrap();
            let verifier_rt = runtime.authenticate_operator([33; 32]).await.unwrap();
            let courier_rt = runtime.authenticate_operator([34; 32]).await.unwrap();
            assert_eq!(verifier_rt.id(), verifier.id());
            assert_eq!(courier_rt.id(), courier.id());
            let executor = std::slice::from_ref(&published.cohort.executor);
            runtime.advertise(&host, executor, executor).await.unwrap();

            let account = Uuid::now_v7();
            let mut snapshot = runtime
                .start(
                    account,
                    Uuid::now_v7(),
                    Start {
                        cohort,
                        starter: 0,
                        consent: CONSENT_REVISION,
                    },
                    [9; 32],
                )
                .await
                .unwrap();
            // Verified defeats until the run ends: reservation, record,
            // seal, verification, settlement, reward claim, award outbox and
            // the personal best, every one as the runtime role.
            for duel in 0..12_u8 {
                if matches!(snapshot.state.phase, Phase::Exhausted | Phase::Conquered) {
                    break;
                }
                runtime
                    .reserve(
                        account,
                        Uuid::now_v7(),
                        Reserve {
                            run: snapshot.id,
                            revision: snapshot.state.revision,
                            host: host.id(),
                        },
                        entropy(40 + duel),
                        4096,
                    )
                    .await
                    .unwrap();
                let location = runtime.locate(account, snapshot.id).await.unwrap();
                assert_eq!(location.host.map(|host| host.id), Some(host.id()));
                let claimed = runtime
                    .claim(&host, snapshot.id, Uuid::now_v7(), 120_000)
                    .await
                    .unwrap();
                let ack = runtime
                    .append(&host, claimed.lease, claimed.prefix, b"synthetic record\n")
                    .await
                    .unwrap();
                runtime.seal(&host, claimed.lease, ack.prefix).await.unwrap();
                let job = runtime
                    .claim_verification(
                        &verifier_rt,
                        claimed.lease.reservation,
                        Uuid::now_v7(),
                        120_000,
                    )
                    .await
                    .unwrap();
                let report = Settlement {
                    binding: job.lease.binding,
                    outcome: Outcome::Defeat,
                    turns: 12,
                    used: [0; 2],
                };
                snapshot = runtime
                    .settle_verified(&verifier_rt, job.lease, &report, [50 + duel; 32])
                    .await
                    .unwrap();
                if snapshot.state.phase == Phase::Reward {
                    let reward = snapshot.state.reward.clone().unwrap();
                    let mut cards: Vec<_> = reward
                        .cards
                        .iter()
                        .copied()
                        .take(usize::from(reward.take))
                        .collect();
                    cards.sort_unstable();
                    snapshot = runtime
                        .command(
                            account,
                            Uuid::now_v7(),
                            snapshot.id,
                            snapshot.state.revision,
                            &Command::Claim { cards },
                        )
                        .await
                        .unwrap();
                }
            }
            assert_eq!(snapshot.state.phase, Phase::Exhausted);
            let award = runtime.claim_award(&courier_rt).await.unwrap().unwrap();
            runtime
                .acknowledge_award(&courier_rt, award.lease)
                .await
                .unwrap();
            let next = runtime.claim_award(&courier_rt).await.unwrap().unwrap();
            runtime.defer_award(&courier_rt, next.lease).await.unwrap();

            let name = Name {
                display: "Synthetic".into(),
                tag: 7,
            };
            let listing = runtime.listing(account, 0, true, Some(&name)).await.unwrap();
            let board = runtime.ranking(account, cohort, Lane::House).await.unwrap();
            assert!(board.best.is_some());
            runtime.ranking_identity(account, None).await.unwrap();
            runtime
                .listing(account, listing.revision + 1, false, None)
                .await
                .unwrap();
            runtime.withdraw_echoes(account).await.unwrap();
            runtime.runs(account, None, 8).await.unwrap();

            // Administration, through the store and in plain SQL.
            assert!(runtime.publish(&published).await.is_err());
            assert!(
                runtime
                    .provision_operator("forged", Role::Host, [90; 32])
                    .await
                    .is_err()
            );
            assert!(
                runtime
                    .approve_host(host.id(), "http://127.0.0.1:29882", true)
                    .await
                    .is_err()
            );
            assert!(runtime.revoke_operator(host.id()).await.is_err());
            assert!(runtime.pause_enrollment(cohort).await.is_err());
            assert!(
                runtime
                    .publish_house(
                        &runtime.authenticate_operator([31; 32]).await.unwrap(),
                        cohort,
                        2,
                        1,
                        &Build {
                            formation: published.starters[0].formation.clone(),
                            deck: published.starters[0].deck.clone(),
                            active: Vec::new(),
                            potions: [0; 2],
                        },
                    )
                    .await
                    .is_err()
            );
            for refused in [
                format!("UPDATE \"{schema}\".operators SET enabled=true"),
                format!("UPDATE \"{schema}\".operators SET credential_digest=credential_digest"),
                format!("UPDATE \"{schema}\".cohorts SET accepting=true"),
                format!("UPDATE \"{schema}\".host_locations SET development=true"),
                format!("UPDATE \"{schema}\".players SET account=account"),
                format!("UPDATE \"{schema}\".settled_duels SET report=report"),
                format!("DELETE FROM \"{schema}\".runs"),
                format!("DELETE FROM \"{schema}\".settled_duels"),
                format!("DELETE FROM \"{schema}\".players"),
                format!("TRUNCATE \"{schema}\".record_chunks"),
                format!("INSERT INTO \"{schema}\".schema_version SELECT * FROM \"{schema}\".schema_version"),
                format!("CREATE TABLE \"{schema}\".planted(x int)"),
            ] {
                assert!(
                    runtime_db.execute_unprepared(&refused).await.is_err(),
                    "the runtime role may not: {refused}"
                );
            }
            runtime_db.close().await.unwrap();
        })
        .await
    };
    // Reap the grants and the owned login before the schema, even on panic.
    sandbox
        .db
        .execute_unprepared(&format!("DROP OWNED BY \"{role}\"; DROP ROLE \"{role}\""))
        .await
        .map_err(|_| ())
        .expect("owned synthetic role removed");
    sandbox.close().await;
    observed.expect("the runtime grants carry a run and refuse administration");
}
