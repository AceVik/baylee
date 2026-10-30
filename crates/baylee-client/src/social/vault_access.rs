//! Account-password handoff and off-thread vault startup. A failed unlock is
//! never the absence of keys. Guests and browser tabs use memory only.

use baylee_client_core::lobby::LobbyRequest;
#[cfg(not(target_arch = "wasm32"))]
use zeroize::Zeroizing;

use super::{SocialHost, social};
use crate::lobby::LobbyState;

/// Short-lived authentication material. Never serialized or printed.
pub(crate) struct Login {
    gateway: String,
    token: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    password: Zeroizing<String>,
    #[cfg(not(target_arch = "wasm32"))]
    previous: Option<Zeroizing<String>>,
    created: web_time::Instant,
}

impl std::fmt::Debug for Login {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Login([redacted])")
    }
}

impl Login {
    /// One local-only handoff after Central verified the password. Never used for gateway HTTP.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn central(gateway: &str, password: String) -> Self {
        Self {
            gateway: gateway.into(),
            token: None,
            password: Zeroizing::new(password),
            previous: None,
            created: web_time::Instant::now(),
        }
    }

    /// Recent, and for `gateway`: the live gateway's former address is the
    /// live gateway (`gateway_list::moved`).
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn fresh_for(&self, gateway: &str) -> bool {
        use baylee_client_core::lobby::gateway_list::moved;
        self.created.elapsed().as_secs() <= 120
            && moved(self.gateway.trim_end_matches('/')) == moved(gateway.trim_end_matches('/'))
    }
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn from_request(_gateway: &str, _request: &LobbyRequest) -> Option<Self> {
        None
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn from_request(gateway: &str, request: &LobbyRequest) -> Option<Self> {
        let (password, previous) = match request {
            LobbyRequest::LogIn { password, .. } => (password, None),
            LobbyRequest::ChangePassword {
                password,
                new_password,
                ..
            } => (new_password, Some(Zeroizing::new(password.clone()))),
            _ => return None,
        };
        Some(Self {
            gateway: gateway.trim_end_matches('/').to_string(),
            token: None,
            password: Zeroizing::new(password.clone()),
            previous,
            created: web_time::Instant::now(),
        })
    }

    pub(crate) fn accept(&mut self, token: &str) {
        self.token = Some(token.to_string());
    }
}

pub(crate) fn discard_stale_login(lobby: &mut LobbyState) {
    if lobby.vault_login.as_ref().is_some_and(|l| {
        l.created.elapsed().as_secs() > 120
            || l.gateway != lobby.gateway.trim_end_matches('/')
            || l.token
                .as_deref()
                .is_some_and(|t| Some(t) != lobby.lobby.token())
    }) {
        lobby.vault_login = None;
    }
}

/// The worker's lifecycle, with passwords owned by its job rather than ECS.
#[derive(Default)]
pub(crate) struct Access {
    pub(crate) prefs: social::ChatPrefs,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) attempted: bool,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) error: Option<Error>,
    #[cfg(not(target_arch = "wasm32"))]
    central: Option<crate::lobby::central::Handle>,
    #[cfg(not(target_arch = "wasm32"))]
    sync: Option<std::sync::Arc<native::central::SyncState>>,
    #[cfg(not(target_arch = "wasm32"))]
    ack: AckGate,
    #[cfg(not(target_arch = "wasm32"))]
    job: Option<Job>,
}

impl Access {
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn clear_sync(&mut self) {
        self.sync = None;
        self.ack = AckGate::default();
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn snapshot_owner(&self) -> Option<std::sync::Arc<native::central::SyncState>> {
        self.sync.clone()
    }

    /// Return true when a Central chat ACK was retained until its manifest is durable.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn defer_ack(&mut self, upto: u64) -> bool {
        let Some(sync) = &self.sync else {
            return false;
        };
        self.ack.hold(upto, sync.generation());
        sync.queue(false);
        true
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn ready_ack(&self) -> Option<u64> {
        self.ack.ready(self.sync.as_ref()?.acknowledged()?)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn sent_ack(&mut self, upto: u64) {
        self.ack.sent(upto);
    }

    pub(crate) fn sync_status(&self) -> Option<SyncStatus> {
        #[cfg(not(target_arch = "wasm32"))]
        return self.sync.as_ref().map(|sync| sync.status());
        #[cfg(target_arch = "wasm32")]
        None
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn is_central(&self) -> bool {
        self.central.is_some()
    }
}

/// An ACK covers every completed disk effect before it, including the full device state.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Default)]
struct AckGate(Option<(u64, u64)>);

#[cfg(not(target_arch = "wasm32"))]
impl AckGate {
    fn hold(&mut self, upto: u64, generation: u64) {
        if self.0.is_none_or(|(prior, _)| upto >= prior) {
            self.0 = Some((upto, generation));
        }
    }
    fn ready(&self, uploaded: u64) -> Option<u64> {
        self.0
            .filter(|(_, generation)| *generation <= uploaded)
            .map(|(upto, _)| upto)
    }
    fn sent(&mut self, upto: u64) {
        if self.0.is_some_and(|(pending, _)| pending == upto) {
            self.0 = None;
        }
    }
}

/// Local progress, never a claim that an offline device's pending writes were uploaded.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(target_arch = "wasm32", allow(dead_code))] // browser chat has no Central archive worker
pub(crate) enum SyncStatus {
    Pending,
    Failed,
    Saved,
}

impl SyncStatus {
    pub(crate) const fn phrase(self) -> social::ChatPhrase {
        match self {
            Self::Pending => social::ChatPhrase::ArchivePending,
            Self::Failed => social::ChatPhrase::ArchiveFailed,
            Self::Saved => social::ChatPhrase::ArchiveSaved,
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn follow(_host: &mut SocialHost, lobby: &mut LobbyState) {
    // Browser chat remains explicitly ephemeral; no password is kept for it.
    lobby.vault_login = None;
}

#[cfg(not(target_arch = "wasm32"))]
use native::Job;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use native::{Choice, Error};

#[cfg(not(target_arch = "wasm32"))]
mod native;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) use native::{begin, follow};

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    #[test]
    fn delivery_ack_waits_for_its_complete_snapshot_and_survives_failed_send() {
        let mut gate = AckGate::default();
        gate.hold(10, 5);
        assert_eq!(
            gate.ready(4),
            None,
            "failed/unacknowledged upload cannot discard the inbox"
        );
        assert_eq!(gate.ready(5), Some(10));
        assert_eq!(
            gate.ready(5),
            Some(10),
            "socket failure retains the pending ACK"
        );
        gate.hold(11, 6);
        gate.sent(10);
        assert_eq!(
            gate.ready(5),
            None,
            "an older completed upload cannot release a newer delivery"
        );
        assert_eq!(gate.ready(6), Some(11));
        gate.sent(11);
        assert_eq!(gate.ready(u64::MAX), None);
    }
    #[test]
    fn only_auth_requests_supply_redacted_one_shot_credentials() {
        assert!(Login::from_request("https://example.test", &LobbyRequest::ListDecks).is_none());
        let mut login = Login::from_request(
            "https://example.test/",
            &LobbyRequest::LogIn {
                username: "alice".into(),
                password: "secret".into(),
            },
        )
        .unwrap();
        assert!(login.token.is_none());
        assert!(!format!("{login:?}").contains("secret"));
        login.accept("accepted-token");
        assert_eq!(login.token.as_deref(), Some("accepted-token"));
        assert_eq!(login.gateway, "https://example.test");
    }
}
