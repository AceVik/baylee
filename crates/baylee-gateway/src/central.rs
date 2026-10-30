//! Opt-in migration bridge. Secrets are read from an operator-owned file and never logged.
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

mod routes;
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

impl Bridge {
    pub(crate) fn from_env() -> Result<Option<Arc<Self>>, ()> {
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
            if let Some(origin) = &self.origin {
                baylee_account::gateway::validate_origin(
                    origin,
                    endpoint.legacy_origin().is_none(),
                )
                .map_err(|_| ())?;
                if endpoint
                    .legacy_origin()
                    .is_some_and(|expected| expected != origin)
                {
                    return Err(());
                }
            }
            // The official gateway's directory entry, sent with the bridge
            // credential. Its website lies on the bridge's origin: the compiled
            // legacy origin live, the configured `origin` in development.
            let metadata = match self.metadata {
                None => None,
                Some(mut metadata) => {
                    let origin = endpoint
                        .legacy_origin()
                        .map(str::to_owned)
                        .or(self.origin)
                        .ok_or(())?;
                    metadata.version = env!("CARGO_PKG_VERSION").into();
                    metadata.validate(&origin).map_err(|_| ())?;
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
    /// Beats for a registered gateway, and for the official one when its
    /// bridge has `metadata`; nothing without metadata.
    pub(crate) fn start(bridge: Option<&Arc<Bridge>>) -> Option<Self> {
        let bridge = bridge.filter(|bridge| bridge.metadata.is_some())?.clone();
        let (stop, mut stopped) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                tokio::select! {
                    _ = &mut stopped => return,
                    _ = interval.tick() => {}
                }
                let Some(metadata) = bridge.metadata.clone() else {
                    return;
                };
                // Network errors contain no operator-supplied diagnostics and never alter
                // authorization: directory liveness expires centrally if heartbeats stop.
                let _ = bridge
                    .exchange(wire::Request::GatewayHeartbeat { metadata })
                    .await;
            }
        });
        Some(Self {
            stop: Some(stop),
            task,
        })
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
            let bridge = state
                .central
                .as_ref()
                .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
            let Some(standing) = authorize(
                state,
                bridge,
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
    if state.central.is_none() {
        return;
    }
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
    let Some(bridge) = state.central.as_ref() else {
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
    let Some(bridge) = state.central.as_ref().filter(|bridge| !bridge.legacy) else {
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
    if state.central.is_none() {
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
        let live = Endpoint::live();
        let (_, heartbeat, metadata, legacy) = official("https://baylee.acevik.de/")
            .validate(&live)
            .unwrap();
        assert!(legacy);
        assert!(
            heartbeat.is_none(),
            "the bridge beats with its own credential"
        );
        let metadata = metadata.unwrap();
        assert_eq!(metadata.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(metadata.website, "https://baylee.acevik.de/");
        let mut named = official("https://baylee.acevik.de/status");
        named.origin = Some("https://baylee.acevik.de".into());
        assert!(named.validate(&live).is_ok());
        // Without metadata the bridge is as before: no heartbeat at all.
        let mut plain = official("https://baylee.acevik.de/");
        plain.metadata = None;
        let (_, _, metadata, legacy) = plain.validate(&live).unwrap();
        assert!(legacy && metadata.is_none());
        // A website elsewhere, or a registered gateway's credentials beside the bridge.
        assert!(
            official("https://gateway.example/")
                .validate(&live)
                .is_err()
        );
        let mut with_credential = official("https://baylee.acevik.de/");
        with_credential.credential = Some(Token::generate().unwrap());
        assert!(with_credential.validate(&live).is_err());
        let mut with_heartbeat = official("https://baylee.acevik.de/");
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
