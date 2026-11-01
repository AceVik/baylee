//! The gateway's link to Baylee Central: the operator's file
//! (`BAYLEE_CENTRAL_CONFIG_FILE`, the official gateway's migration bridge or
//! a registered credential pair), or a token an admin pasted into the console
//! (`link.rs`, baylee-central `docs/gateway-registry.md` §"Gateway tokens").
//! Secrets are never logged.
#![forbid(unsafe_code)]

use crate::Shared;
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use baylee_account::{CentralHttp, Endpoint, Token, wire};
use serde::Deserialize;
use std::{io::Read, sync::Arc, time::Duration};
use zeroize::Zeroizing;

mod link;
mod routes;
pub(crate) use link::{console_routes, restore};
pub(crate) use routes::router;

pub(crate) struct Bridge {
    http: CentralHttp,
    credential: Token,
    heartbeat: Option<Token>,
    metadata: Option<baylee_account::gateway::GatewayMetadata>,
    legacy: bool,
    workers: Arc<tokio::sync::Semaphore>,
    pending: Arc<tokio::sync::Semaphore>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Settings {
    /// Exactly "live" or an explicitly selected numeric-loopback development origin.
    endpoint: String,
    #[serde(default)]
    bridge: Option<Token>,
    #[serde(default)]
    origin: Option<String>,
    #[serde(default)]
    credential: Option<Token>,
    #[serde(default)]
    heartbeat: Option<Token>,
    #[serde(default)]
    metadata: Option<baylee_account::gateway::GatewayMetadata>,
}

/// The link as the gateway holds it: the bridge in use, if any. One from the
/// operator's file is fixed for the process; a pasted token's can be
/// replaced or removed while the gateway runs, so every user asks for the
/// bridge each time ([`Link::get`]) rather than keeping one.
pub(crate) struct Link {
    slot: std::sync::RwLock<Option<Arc<Bridge>>>,
    /// The operator's file set it; the console shows it and cannot change it.
    file: bool,
    /// Where a pasted token talks to: `BAYLEE_CENTRAL_ENDPOINT`, `live`
    /// unless an explicitly selected numeric-loopback development origin.
    endpoint: Endpoint,
}

impl Link {
    /// The operator's file, if `BAYLEE_CENTRAL_CONFIG_FILE` names one; else
    /// empty until [`restore`] reads a pasted token from the database.
    pub(crate) fn from_env() -> Result<Self, ()> {
        let endpoint = match std::env::var("BAYLEE_CENTRAL_ENDPOINT") {
            Err(_) => Endpoint::live(),
            Ok(value) if value == "live" => Endpoint::live(),
            Ok(value) => Endpoint::development(&value).map_err(|_| ())?,
        };
        let bridge = Bridge::from_env()?;
        Ok(Self {
            file: bridge.is_some(),
            slot: std::sync::RwLock::new(bridge),
            endpoint,
        })
    }

    /// The bridge in use now, if any.
    pub(crate) fn get(&self) -> Option<Arc<Bridge>> {
        self.slot
            .read()
            .map_or(None, |slot| slot.as_ref().map(Arc::clone))
    }

    pub(crate) fn is_some(&self) -> bool {
        self.get().is_some()
    }

    /// Whether the operator's file set the link (the console cannot change it).
    pub(crate) const fn fixed_by_file(&self) -> bool {
        self.file
    }

    /// Use `bridge` from now on: a pasted token's, or none. A request that
    /// holds the earlier bridge finishes with it.
    fn set(&self, bridge: Option<Arc<Bridge>>) {
        if let Ok(mut slot) = self.slot.write() {
            *slot = bridge;
        }
    }
}

impl Bridge {
    fn from_env() -> Result<Option<Arc<Self>>, ()> {
        let Some(path) = std::env::var_os("BAYLEE_CENTRAL_CONFIG_FILE") else {
            return Ok(None);
        };
        let mut bytes = Zeroizing::new(Vec::new());
        std::fs::File::open(path)
            .map_err(|_| ())?
            .take(4097)
            .read_to_end(&mut bytes)
            .map_err(|_| ())?;
        if bytes.len() > 4096 {
            return Err(());
        }
        let settings: Settings = serde_json::from_slice(&bytes).map_err(|_| ())?;
        let endpoint = if settings.endpoint == "live" {
            Endpoint::live()
        } else {
            Endpoint::development(&settings.endpoint).map_err(|_| ())?
        };
        let (credential, heartbeat, metadata, legacy) = settings.validate(&endpoint)?;
        Ok(Some(Arc::new(Self {
            http: CentralHttp::new(endpoint),
            credential,
            heartbeat,
            metadata,
            legacy,
            workers: Arc::new(tokio::sync::Semaphore::new(4)),
            pending: Arc::new(tokio::sync::Semaphore::new(64)),
        })))
    }

    /// A pasted gateway token's bridge: one token serves both its players'
    /// sign-ins and the heartbeat, for what its scopes allow; `metadata` is
    /// the directory entry it beats with, on the token's origin.
    fn from_token(
        endpoint: Endpoint,
        token: &Token,
        metadata: baylee_account::gateway::GatewayMetadata,
    ) -> Arc<Self> {
        Arc::new(Self {
            http: CentralHttp::new(endpoint),
            credential: Token::from_secret_bytes(token.to_secret_bytes()),
            heartbeat: Some(Token::from_secret_bytes(token.to_secret_bytes())),
            metadata: Some(metadata),
            legacy: false,
            workers: Arc::new(tokio::sync::Semaphore::new(4)),
            pending: Arc::new(tokio::sync::Semaphore::new(64)),
        })
    }

    /// Whether this is the official gateway's bridge, the one credential
    /// Central takes a report straight from a client under.
    pub(crate) const fn official(&self) -> bool {
        self.legacy
    }

    /// Files a report's metadata with Central under this gateway's own
    /// credential (`report::central`), blocking.
    pub(crate) fn file_report(
        &self,
        intake: &baylee_account::reports::ReportIntake,
    ) -> Result<[u8; 16], baylee_account::Error> {
        self.http.file_report(intake, &self.credential)
    }

    /// Sends a part `report` announced, under the same credential, blocking.
    pub(crate) fn put_report_part(
        &self,
        report: &[u8; 16],
        part: baylee_account::reports::Part,
        gzip: &[u8],
    ) -> Result<(), baylee_account::Error> {
        self.http
            .put_report_part(report, part, gzip, &self.credential)
    }

    async fn exchange(
        self: &Arc<Self>,
        request: wire::Request,
    ) -> Result<wire::Response, StatusCode> {
        // A small bounded queue absorbs simultaneous socket rechecks. Failing the
        // fifth request immediately would disconnect healthy clients under bursts;
        // neither queue capacity nor network stalls may grow blocking work without bound.
        let pending = self
            .pending
            .clone()
            .try_acquire_owned()
            .map_err(|_| StatusCode::TOO_MANY_REQUESTS)?;
        let permit =
            tokio::time::timeout(Duration::from_secs(2), self.workers.clone().acquire_owned())
                .await
                .map_err(|_| StatusCode::TOO_MANY_REQUESTS)?
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        let bridge = self.clone();
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            let _pending = pending;
            // A registered gateway beats with its heartbeat token; the official
            // gateway's bridge beats with the bridge credential, for the
            // official directory entry alone (baylee-central
            // `docs/gateway-registry.md` §"Discovery, heartbeats and administrators").
            let credential =
                if matches!(&request, wire::Request::GatewayHeartbeat { .. }) && !bridge.legacy {
                    bridge.heartbeat.as_ref().ok_or(StatusCode::UNAUTHORIZED)?
                } else {
                    &bridge.credential
                };
            bridge
                .http
                .exchange(&request, Some(credential))
                .map_err(|error| match error {
                    baylee_account::Error::Conflict => StatusCode::CONFLICT,
                    baylee_account::Error::Busy => StatusCode::TOO_MANY_REQUESTS,
                    baylee_account::Error::Rejected => StatusCode::UNAUTHORIZED,
                    baylee_account::Error::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
                    // A gateway's own credential is no account session; Central
                    // never answers it so, and a 403 is the honest reading.
                    baylee_account::Error::TermsRequired => StatusCode::FORBIDDEN,
                })
        })
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
    }
}

impl Settings {
    fn validate(
        self,
        endpoint: &Endpoint,
    ) -> Result<
        (
            Token,
            Option<Token>,
            Option<baylee_account::gateway::GatewayMetadata>,
            bool,
        ),
        (),
    > {
        if let Some(bridge) = self.bridge {
            if self.credential.is_some() || self.heartbeat.is_some() {
                return Err(());
            }
            // Only the separately provisioned official migration bridge has legacy authority.
            // Synthetic loopback fixtures deliberately use their isolated bridge credential.
            // Live, the bridge is the official gateway's at any of its compiled
            // origins (`Endpoint::official_origins`: the new one, and the one
            // it moved from, docs/central-login.md §"The official gateway's aliases").
            let official = endpoint.official_origins();
            if let Some(origin) = &self.origin {
                baylee_account::gateway::validate_origin(
                    origin,
                    endpoint.legacy_origin().is_none(),
                )
                .map_err(|_| ())?;
                if !official.is_empty() && !official.contains(origin) {
                    return Err(());
                }
            }
            // The official gateway's directory entry, sent with the bridge
            // credential. Its website lies on the bridge's origin: the
            // configured `origin`, or live without one any of the official
            // gateway's, as Central checks it.
            let metadata = match self.metadata {
                None => None,
                Some(mut metadata) => {
                    let origins = self.origin.map_or(official, |origin| vec![origin]);
                    metadata.version = env!("CARGO_PKG_VERSION").into();
                    if !origins
                        .iter()
                        .any(|origin| metadata.validate(origin).is_ok())
                    {
                        return Err(());
                    }
                    Some(metadata)
                }
            };
            return Ok((bridge, None, metadata, true));
        }
        let (Some(origin), Some(credential), Some(heartbeat), Some(mut metadata)) =
            (self.origin, self.credential, self.heartbeat, self.metadata)
        else {
            return Err(());
        };
        baylee_account::gateway::validate_origin(&origin, endpoint.legacy_origin().is_none())
            .map_err(|_| ())?;
        if credential.digest() == heartbeat.digest() {
            return Err(());
        }
        metadata.version = env!("CARGO_PKG_VERSION").into();
        metadata.validate(&origin).map_err(|_| ())?;
        Ok((credential, Some(heartbeat), Some(metadata), false))
    }
}

/// Owned heartbeat task. Dropping it cancels periodic work; each blocking request
/// retains its permit until the bounded strict HTTP transport completes.
pub(crate) struct Heartbeat {
    stop: Option<tokio::sync::oneshot::Sender<()>>,
    task: tokio::task::JoinHandle<()>,
}

impl Heartbeat {
    /// Beats every 30 seconds with the bridge in use then: a registered
    /// gateway's, a pasted token's, and the official one's when its bridge
    /// has `metadata`; nothing while there is none or it has no metadata.
    pub(crate) fn start(state: Shared) -> Self {
        let (stop, mut stopped) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                tokio::select! {
                    _ = &mut stopped => return,
                    _ = interval.tick() => {}
                }
                let Some(bridge) = state.central.get() else {
                    continue;
                };
                let Some(metadata) = bridge.metadata.clone() else {
                    continue;
                };
                // Network errors contain no operator-supplied diagnostics and never alter
                // authorization: directory liveness expires centrally if heartbeats stop.
                let _ = bridge
                    .exchange(wire::Request::GatewayHeartbeat { metadata })
                    .await;
            }
        });
        Self {
            stop: Some(stop),
            task,
        }
    }
}

impl Drop for Heartbeat {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        self.task.abort();
    }
}

/// Resolve local authorization and require a fresh Central check for its gateway lease.
/// No Central account session or password is available to the gateway.
pub(crate) async fn resolve_digest(
    state: &Shared,
    digest: Vec<u8>,
) -> Result<Option<crate::store::Session>, StatusCode> {
    let binding = baylee_db::central::session_binding(&state.db, digest.clone())
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    match binding {
        baylee_db::central::SessionBinding::Local => {}
        baylee_db::central::SessionBinding::Rejected => return Ok(None),
        baylee_db::central::SessionBinding::Central(binding) => {
            if binding.purpose != "admit" {
                return Ok(None);
            }
            let bridge = state.central.get().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
            let Some(standing) = authorize(
                state,
                &bridge,
                binding.lease,
                binding.central,
                binding.expires_at,
            )
            .await?
            else {
                return Ok(None);
            };
            let session = crate::store::resolve_digest(&state.db, digest, crate::auth::now_secs())
                .await
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
            return Ok(session.map(|session| crate::store::Session {
                terms: standing,
                central: true,
                ..session
            }));
        }
    }
    let session = crate::store::resolve_digest(&state.db, digest, crate::auth::now_secs())
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    Ok(session)
}

/// Central's live check of an admission lease, with whether its account
/// accepted Central's terms in force and the version of this gateway's
/// terms Central confirmed (none confirmed: nothing to accept yet, so
/// nothing is enforced). `None` when the lease is no longer good or no
/// longer the one bound here. A renewed session's lease comes back with a
/// later expiry (never an earlier one), which is stored here too
/// ([`renewed`]).
pub(crate) async fn authorize(
    state: &Shared,
    bridge: &Arc<Bridge>,
    lease: [u8; 32],
    account: [u8; 16],
    expires: i64,
) -> Result<Option<crate::terms::Standing>, StatusCode> {
    let response = bridge
        .exchange(wire::Request::GatewayAuthorize {
            lease: Token::from_secret_bytes(Zeroizing::new(lease)),
            purpose: wire::TicketPurpose::Admit,
            terms: state.terms.confirmed(),
        })
        .await?;
    Ok(match response {
        wire::Response::GatewayAuthorization {
            identity,
            expires_at,
            central_terms,
            gateway_terms,
        } if identity.account == account && expires_at >= expires => {
            renewed(state, lease, expires, expires_at).await?;
            Some(crate::terms::Standing::of(central_terms, gateway_terms))
        }
        _ => None,
    })
}

/// Central renewed the session behind `lease` (baylee-central
/// `docs/operations.md` §"Sessions"): its later expiry becomes this
/// gateway's for the lease, which bounds the local session, so that lasts as
/// long as Central's. Nothing when it did not move.
async fn renewed(
    state: &Shared,
    lease: [u8; 32],
    expires: i64,
    expires_at: i64,
) -> Result<(), StatusCode> {
    if expires_at <= expires {
        return Ok(());
    }
    baylee_db::central::extend_session(&state.db, lease, expires_at)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)
}

/// Publish the terms in force to Central until it confirms them; only then
/// can players accept them there, and only then does the gateway enforce
/// them. Every minute, and at once after an admin published a version.
pub(crate) fn spawn_terms_publisher(state: Shared) {
    // a token pasted later links the gateway then, so the loop runs anyway
    tokio::spawn(async move {
        loop {
            publish_terms(&state).await;
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    });
}

/// One attempt; `true` when Central holds the terms in force (or there are
/// none, or no Central to hold them).
pub(crate) async fn publish_terms(state: &Shared) -> bool {
    let Some(terms) = state.terms.current() else {
        return true;
    };
    if state.terms.confirmed().is_some() {
        return true;
    }
    let Some(bridge) = state.central.get() else {
        return true;
    };
    let request = wire::Request::PublishGatewayTerms {
        version: terms.version.clone(),
        texts: terms.central_texts(),
    };
    match bridge.exchange(request).await {
        Ok(wire::Response::TermsPublished { digest }) if digest == terms.digest => {
            state.terms.confirm(&terms.central_version());
            tracing::info!(version = %terms.version, "terms of use published to Baylee Central");
            true
        }
        Err(StatusCode::CONFLICT) => {
            tracing::error!(
                version = %terms.version,
                "Baylee Central holds another text under this terms version; publish the \
                 changed terms under a new version"
            );
            false
        }
        Ok(_) => {
            tracing::error!("Baylee Central answered publishing the terms with something else");
            false
        }
        Err(status) => {
            tracing::warn!(%status, "publishing the terms of use to Baylee Central failed; retrying");
            false
        }
    }
}

/// Gateway administration accepts only a purpose-bound lease with current Central
/// membership. The audit actor comes from that lease, never a client-supplied name.
pub(crate) async fn admin_actor(
    state: &Shared,
    bearer: &str,
) -> Result<Option<String>, StatusCode> {
    let digest = crate::auth::token_digest(bearer);
    let binding = baylee_db::central::session_binding(&state.db, digest.clone())
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let baylee_db::central::SessionBinding::Central(binding) = binding else {
        return Ok(None);
    };
    if binding.purpose != "admin" {
        return Ok(None);
    }
    let Some(bridge) = state.central.get().filter(|bridge| !bridge.legacy) else {
        return Ok(None);
    };
    let response = bridge
        .exchange(wire::Request::GatewaySession {
            lease: Token::from_secret_bytes(Zeroizing::new(binding.lease)),
            purpose: wire::TicketPurpose::Admin,
        })
        .await?;
    let wire::Response::GatewaySession {
        identity,
        expires_at,
    } = response
    else {
        return Ok(None);
    };
    if identity.account != binding.central || expires_at < binding.expires_at {
        return Ok(None);
    }
    renewed(state, binding.lease, binding.expires_at, expires_at).await?;
    let Some(session) = crate::store::resolve_digest(&state.db, digest, crate::auth::now_secs())
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
    else {
        return Ok(None);
    };
    if crate::moderation::admit(&session).is_err() {
        return Ok(None);
    }
    Ok(Some(uuid::Uuid::from_bytes(binding.central).to_string()))
}

/// Bind a seat capability to the session which authorized its creation.
pub(crate) async fn seat_binding(
    state: &Shared,
    headers: &axum::http::HeaderMap,
) -> Result<Option<Vec<u8>>, (StatusCode, axum::Json<crate::ErrorBody>)> {
    let bearer = crate::bearer_token(headers)
        .ok_or_else(|| crate::err(StatusCode::UNAUTHORIZED, "missing session"))?;
    let digest = crate::auth::token_digest(bearer);
    match baylee_db::central::session_binding(&state.db, digest.clone()).await {
        Ok(baylee_db::central::SessionBinding::Local) => Ok(None),
        Ok(baylee_db::central::SessionBinding::Central(_)) => Ok(Some(digest)),
        Ok(baylee_db::central::SessionBinding::Rejected) => {
            Err(crate::err(StatusCode::UNAUTHORIZED, "expired session"))
        }
        Err(_) => Err(crate::err(
            StatusCode::SERVICE_UNAVAILABLE,
            "account authorization unavailable",
        )),
    }
}

/// Drain in-flight account requests while a Central admission fixes a link
/// or a projection: `central::routes` takes this barrier exclusively.
pub(crate) async fn barrier(State(state): State<Shared>, request: Request, next: Next) -> Response {
    if !state.central.is_some() {
        return next.run(request).await;
    }
    let Ok(_guard) =
        tokio::time::timeout(Duration::from_secs(5), state.central_barrier.read()).await
    else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registered() -> Settings {
        Settings {
            endpoint: "live".into(),
            bridge: None,
            origin: Some("https://gateway.example".into()),
            credential: Some(Token::generate().unwrap()),
            heartbeat: Some(Token::generate().unwrap()),
            metadata: Some(baylee_account::gateway::GatewayMetadata {
                name: "Synthetic gateway".into(),
                version: "ignored".into(),
                website: "https://gateway.example/".into(),
            }),
        }
    }

    #[test]
    fn gateway_credentials_are_separate_and_configuration_cannot_escalate_to_migration() {
        let endpoint = Endpoint::live();
        let (credential, heartbeat, metadata, legacy) = registered().validate(&endpoint).unwrap();
        assert!(!legacy);
        assert_ne!(credential.digest(), heartbeat.unwrap().digest());
        assert_eq!(metadata.unwrap().version, env!("CARGO_PKG_VERSION"));
        let mut ambiguous = registered();
        ambiguous.bridge = Some(Token::generate().unwrap());
        assert!(ambiguous.validate(&endpoint).is_err());
        let mut reused = registered();
        reused.heartbeat = Some(Token::from_secret_bytes(
            reused.credential.as_ref().unwrap().to_secret_bytes(),
        ));
        assert!(reused.validate(&endpoint).is_err());
        let mut wrong_origin = registered();
        wrong_origin.origin = Some("https://other.example".into());
        assert!(wrong_origin.validate(&endpoint).is_err());
        let wrong_bridge = Settings {
            endpoint: "live".into(),
            bridge: Some(Token::generate().unwrap()),
            origin: Some("https://gateway.example".into()),
            credential: None,
            heartbeat: None,
            metadata: None,
        };
        assert!(wrong_bridge.validate(&endpoint).is_err());
    }

    fn official(website: &str) -> Settings {
        Settings {
            endpoint: "live".into(),
            bridge: Some(Token::generate().unwrap()),
            origin: None,
            credential: None,
            heartbeat: None,
            metadata: Some(baylee_account::gateway::GatewayMetadata {
                name: "Synthetic official gateway".into(),
                version: "ignored".into(),
                website: website.into(),
            }),
        }
    }

    /// The official gateway's bridge may carry its directory entry, on the
    /// bridge's origin; never a registered gateway's credentials.
    #[test]
    fn the_bridge_takes_metadata_on_its_own_origin_and_no_registered_credential() {
        use central_api::{LEGACY_ORIGIN, OFFICIAL_ORIGIN};
        let live = Endpoint::live();
        let (_, heartbeat, metadata, legacy) = official(&format!("{OFFICIAL_ORIGIN}/"))
            .validate(&live)
            .unwrap();
        assert!(legacy);
        assert!(
            heartbeat.is_none(),
            "the bridge beats with its own credential"
        );
        let metadata = metadata.unwrap();
        assert_eq!(metadata.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(metadata.website, format!("{OFFICIAL_ORIGIN}/"));
        // A bridge file from before the move: its website on the old host.
        assert!(
            official(&format!("{LEGACY_ORIGIN}/"))
                .validate(&live)
                .is_ok()
        );
        // Named, the origin is either of the official gateway's, and the
        // website lies on that one.
        for origin in [OFFICIAL_ORIGIN, LEGACY_ORIGIN] {
            let mut named = official(&format!("{origin}/status"));
            named.origin = Some(origin.into());
            assert!(named.validate(&live).is_ok(), "{origin}");
        }
        let mut astray = official(&format!("{LEGACY_ORIGIN}/"));
        astray.origin = Some(OFFICIAL_ORIGIN.into());
        assert!(astray.validate(&live).is_err());
        let mut foreign = official("https://gateway.example/");
        foreign.origin = Some("https://gateway.example".into());
        assert!(foreign.validate(&live).is_err());
        // Without metadata the bridge is as before: no heartbeat at all.
        let mut plain = official(&format!("{OFFICIAL_ORIGIN}/"));
        plain.metadata = None;
        let (_, _, metadata, legacy) = plain.validate(&live).unwrap();
        assert!(legacy && metadata.is_none());
        // A website elsewhere, or a registered gateway's credentials beside the bridge.
        assert!(
            official("https://gateway.example/")
                .validate(&live)
                .is_err()
        );
        let mut with_credential = official(&format!("{OFFICIAL_ORIGIN}/"));
        with_credential.credential = Some(Token::generate().unwrap());
        assert!(with_credential.validate(&live).is_err());
        let mut with_heartbeat = official(&format!("{OFFICIAL_ORIGIN}/"));
        with_heartbeat.heartbeat = Some(Token::generate().unwrap());
        assert!(with_heartbeat.validate(&live).is_err());
        // In development the bridge's origin is the configured one: metadata
        // without it is refused, and metadata must lie on it.
        let development = Endpoint::development("http://127.0.0.1:9").unwrap();
        assert!(
            official("http://127.0.0.1:45125/")
                .validate(&development)
                .is_err()
        );
        let mut local = official("http://127.0.0.1:45125/");
        local.origin = Some("http://127.0.0.1:45125".into());
        assert!(local.validate(&development).unwrap().2.is_some());
        let mut foreign = official("http://127.0.0.1:45126/");
        foreign.origin = Some("http://127.0.0.1:45125".into());
        assert!(foreign.validate(&development).is_err());
    }
}
