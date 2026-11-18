//! A playable Tower cohort for the stack's own Tower engine: the schema, the
//! publication bound to that engine's identity, the five operators the
//! authority and the gateway need, and the house's stand-in build for every
//! stage and starter.
//!
//! This is what the operator commands of `docs/tower-operations.md` do one
//! process at a time; a cohort's six hundred house builds make that too slow
//! for a start.
use anyhow::{Context as _, Result, anyhow};
use baylee_tower::v2::{GENERATOR, MAX_STAGES, package::Cohort};
use baylee_tower_service::Artifact;
use baylee_tower_store::{Publication, Role, Store};
use central_api::Token;
use sea_orm::{ConnectOptions, Database};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, time::Duration};
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Input {
    database_url: String,
    schema: String,
    /// The staged Tower engine, absolute; its `--metadata` is the identity.
    engine: PathBuf,
    /// The gateway allowed to host this cohort's games.
    gateway: String,
    /// Operator credentials, 64 lowercase hex digits each.
    host: String,
    verifier: String,
    supervisor: String,
    publisher: String,
    courier: String,
}

#[derive(Serialize)]
pub(crate) struct Reply {
    cohort: Uuid,
    starters: usize,
    stages: u8,
}

pub(crate) async fn run(input: Input) -> Result<Reply> {
    let artifact = Artifact::probe(&input.engine)
        .await
        .map_err(|error| anyhow!("{} is no Tower engine: {error}", input.engine.display()))?;
    let content = baylee_tower_content::runic().context("building the Tower content")?;
    let publication = Publication {
        cohort: Cohort::for_generation(artifact.identity().clone(), &content.catalog, GENERATOR)
            .map_err(|error| anyhow!("the engine's identity makes no cohort: {error:?}"))?,
        catalog: content.catalog,
        starters: content.starters,
    };

    let mut options = ConnectOptions::new(input.database_url);
    options
        .max_connections(4)
        .min_connections(1)
        .sqlx_logging(false)
        .connect_timeout(Duration::from_secs(5));
    let db = Database::connect(options)
        .await
        .map_err(|_| anyhow!("the Tower database cannot be reached"))?;
    let store = Store::new(db.clone(), &input.schema).context("selecting the Tower schema")?;
    store
        .migrate()
        .await
        .context("migrating the Tower schema")?;
    let cohort = store
        .publish(&publication)
        .await
        .context("publishing the cohort")?;

    let operator = |name: &'static str, role: Role, credential: &str| {
        let token = Token::from_hex(credential)
            .map_err(|_| anyhow!("the {name} credential is not 64 hex digits"));
        let store = &store;
        async move {
            let token = token?;
            let id = store
                .provision_operator(name, role, token.digest())
                .await
                .with_context(|| format!("provisioning the {name}"))?;
            Ok::<_, anyhow::Error>((id, token))
        }
    };
    let (host, _) = operator("local stack gateway", Role::Host, &input.host).await?;
    store
        .approve_host(host, &input.gateway, true)
        .await
        .context("approving the gateway as a host")?;
    operator("local stack verifier", Role::Verifier, &input.verifier).await?;
    operator(
        "local stack supervisor",
        Role::Supervisor,
        &input.supervisor,
    )
    .await?;
    operator("local stack courier", Role::Courier, &input.courier).await?;
    let (_, publisher) =
        operator("local stack publisher", Role::Publisher, &input.publisher).await?;
    let publisher = store
        .authenticate_operator(publisher.digest())
        .await
        .context("authenticating the publisher")?;

    for stage in 1..=MAX_STAGES {
        for (index, starter) in publication.starters.iter().enumerate() {
            let archetype = u8::try_from(index + 1).context("more starters than archetypes")?;
            let build = baylee_tower_content::development_house(starter, stage);
            store
                .publish_house(&publisher, cohort, stage, archetype, &build)
                .await
                .with_context(|| format!("publishing the house at stage {stage}"))?;
        }
    }
    let _ = db.close().await;
    Ok(Reply {
        cohort,
        starters: publication.starters.len(),
        stages: MAX_STAGES,
    })
}
