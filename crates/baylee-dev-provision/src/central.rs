//! A fresh local Central: schema, runtime grants, the Tower award issuer, the
//! deployment's own keys and one synthetic administrator account.
//!
//! The stack's supervisor has already made the three roles and the empty
//! database (`central_migrator` owns it). What comes back are the secrets the
//! Central service must be started with: an account is bound to the OPAQUE
//! setup it was enrolled under, so the keys are made here, once, and handed
//! on.
use anyhow::{Context as _, Result, anyhow, bail};
use central_api::{
    Endpoint, Token,
    wire::{Request, Response},
};
use central_crypto::{
    Context,
    jwt::SigningKey,
    opaque::{Enrollment, server::ServerKeys},
    proof::{self, Work},
    vault::{MasterKey, ObjectContext, RecoveryCode},
};
use central_identity::{IdentityService, TokenKeys};
use central_postgres::{
    AccountStore, DevConfig, LegacyStore, appoint_admin, connect_dev, provision_tower_award_issuer,
};
use sea_orm::{ConnectionTrait as _, DatabaseConnection, DbBackend, Statement};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Central's `ops/grants.sql` at the pinned revision, the copy the gateway's
/// integration tests apply. The supervisor refuses a checkout whose own file
/// differs.
const GRANTS: &str = include_str!("../../baylee-gateway/tests/central/runtime-grants.sql");

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Input {
    postgres_port: u16,
    database: String,
    migrator_password: String,
    runtime_password: String,
    legacy_password: String,
    /// Central's native endpoint, `http://127.0.0.1:<port>`.
    endpoint: String,
    /// The one gateway this Central bridges.
    gateway: String,
    /// The Tower authority's origin, whose awards Central accepts.
    tower: String,
    /// The Tower authority's award credential, 64 lowercase hex digits.
    issuer: String,
    account: Account,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Account {
    username: String,
    display_name: String,
    password: String,
}

/// What the Central service's development settings take, as it reads them.
#[derive(Serialize)]
pub(crate) struct Reply {
    opaque_setup: Vec<u8>,
    bridge: Vec<u8>,
    token_seed: Vec<u8>,
    /// The applied migrations, for the supervisor's comparison with the
    /// checkout whose service will read this database.
    migrations: Vec<String>,
}

pub(crate) async fn run(input: Input) -> Result<Reply> {
    let role = |user: &str, password: &str| {
        DevConfig::new(
            input.postgres_port,
            user,
            password.to_owned(),
            &input.database,
        )
        .map_err(|_| anyhow!("the database settings for {user} are not valid"))
    };
    let operator = connect_dev(role("central_migrator", &input.migrator_password)?)
        .await
        .map_err(|_| anyhow!("central_migrator cannot connect"))?;
    central_migration::apply(&operator)
        .await
        .context("applying Central's migrations")?;
    operator
        .execute_unprepared(GRANTS)
        .await
        .map_err(|_| anyhow!("applying Central's runtime grants failed"))?;
    let issuer = Token::from_hex(&input.issuer)
        .map_err(|_| anyhow!("the Tower award credential is not 64 hex digits"))?;
    provision_tower_award_issuer(&operator, &input.tower, &issuer, true)
        .await
        .map_err(|error| anyhow!("provisioning the Tower award issuer: {error}"))?;
    let migrations = applied(&operator).await?;

    let endpoint = Endpoint::development(&input.endpoint)
        .map_err(|_| anyhow!("the Central endpoint is not a loopback development origin"))?;
    let keys = ServerKeys::generate();
    let opaque_setup = keys.to_secret_bytes().to_vec();
    let bridge = Token::generate().map_err(|_| anyhow!("no randomness for the bridge"))?;
    let seed =
        SigningKey::generate_seed().map_err(|_| anyhow!("no randomness for the token key"))?;
    let tokens = TokenKeys::new(SigningKey::from_seed(&seed), Vec::new())
        .map_err(|_| anyhow!("the token key was refused"))?;

    let store = AccountStore::connect_dev(role("central_runtime", &input.runtime_password)?)
        .await
        .map_err(|_| anyhow!("central_runtime cannot connect"))?;
    let legacy = LegacyStore::connect_dev(role("central_legacy", &input.legacy_password)?)
        .await
        .map_err(|_| anyhow!("central_legacy cannot connect"))?;
    let service = IdentityService::new(
        store.clone(),
        &endpoint,
        keys,
        &bridge,
        &input.gateway,
        tokens,
    )
    .map_err(|_| anyhow!("the identity service refused its settings"))?
    .with_tower(Some(&input.tower))
    .map_err(|_| anyhow!("the identity service refused the Tower origin"))?
    // Lowered as the gateway's integration tests lower it: one synthetic
    // account on a loopback database, not a registration to be priced.
    .with_registration_challenge(0, proof::MIN_MEMORY, Duration::ZERO)
    .with_legacy(legacy.clone());
    enroll(&service, &endpoint, &input.account, &input.gateway).await?;
    drop(service);
    let _ = legacy.close().await;
    let _ = store.close().await;

    appoint_admin(&operator, &input.account.username)
        .await
        .map_err(|error| anyhow!("appointing the administrator: {error}"))?;
    let _ = operator.close().await;
    Ok(Reply {
        opaque_setup,
        bridge: bridge.to_secret_bytes().to_vec(),
        token_seed: seed.to_vec(),
        migrations,
    })
}

/// The migration history, which Central keeps in `public`.
async fn applied(operator: &DatabaseConnection) -> Result<Vec<String>> {
    let rows = operator
        .query_all_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT version FROM public.seaql_migrations ORDER BY version",
        ))
        .await
        .map_err(|_| anyhow!("reading the migration history failed"))?;
    rows.iter()
        .map(|row| {
            row.try_get::<String>("", "version")
                .map_err(|_| anyhow!("a migration's name is not text"))
        })
        .collect()
}

/// The website's registration, spoken to the service in this process: the
/// proof, the OPAQUE enrollment, and the sealed first document naming the
/// stack's gateway.
async fn enroll(
    service: &IdentityService,
    endpoint: &Endpoint,
    account: &Account,
    gateway: &str,
) -> Result<()> {
    let ask = |request: Request| async {
        service
            .handle_browser(request, None)
            .await
            .map(|answer| answer.response)
            .map_err(|error| anyhow!("Central refused a registration step: {error}"))
    };
    let Response::RegisterWork {
        nonce,
        parts,
        difficulty,
        memory,
        passes,
        ..
    } = ask(Request::RegisterChallengeFor { signals: 0 }).await?
    else {
        bail!("Central offered no registration proof");
    };
    let counters = proof::solve(
        &nonce,
        Work {
            memory,
            passes,
            parts,
            difficulty,
        },
    )
    .map_err(|_| anyhow!("the registration proof could not be solved"))?;
    let Response::Enrollment { identity, ticket } = ask(Request::Register {
        username: account.username.clone(),
        display_name: Some(account.display_name.clone()),
        nonce,
        counters,
    })
    .await?
    else {
        bail!("Central did not open an enrollment for the synthetic account");
    };
    let context = Context::new(endpoint.issuer(), identity.account, identity.epoch)
        .map_err(|_| anyhow!("the enrollment context was refused"))?;
    let (enrollment, message) = Enrollment::start(account.password.clone(), context)
        .map_err(|_| anyhow!("the OPAQUE enrollment could not start"))?;
    // The ticket is spent twice and is not `Clone`.
    let again = Token::from_hex(&ticket.hex()).map_err(|_| anyhow!("the ticket is not a token"))?;
    let Response::EnrollmentChallenge { message } = ask(Request::EnrollmentStart {
        ticket: again,
        message,
    })
    .await?
    else {
        bail!("Central did not answer the enrollment");
    };
    let finished = enrollment
        .finish(&message)
        .map_err(|_| anyhow!("the OPAQUE enrollment could not finish"))?;
    let master = MasterKey::generate().map_err(|_| anyhow!("no randomness for the master key"))?;
    let wrapped = finished
        .wrapping_key
        .wrap(&master)
        .map_err(|_| anyhow!("wrapping the master key failed"))?;
    let recovery = RecoveryCode::generate()
        .and_then(|code| code.wrap(context, &master))
        .map_err(|_| anyhow!("making the recovery wrapper failed"))?;
    let object = ObjectContext::new(context, identity.account, 1)
        .map_err(|_| anyhow!("the document's context was refused"))?;
    let document = master
        .seal_object(object, &first_document(gateway))
        .map_err(|_| anyhow!("sealing the first document failed"))?;
    let done = ask(Request::EnrollmentFinish {
        ticket,
        record: finished.upload,
        wrapped_master: wrapped.as_bytes().to_vec(),
        document,
        recovery: Some(recovery.as_bytes().to_vec()),
    })
    .await?;
    if !matches!(done, Response::Done) {
        bail!("Central did not finish the enrollment");
    }
    Ok(())
}

/// The plaintext the website seals as an account's object 1, naming the one
/// gateway the account plays on.
pub(crate) fn first_document(gateway: &str) -> Vec<u8> {
    serde_json::json!({
        "version": 1,
        "gateways": [gateway],
        "preferences": {},
        "legacy": null,
        "chat": { "devices": [], "installations": [] },
    })
    .to_string()
    .into_bytes()
}
