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
    job: Option<Job>,
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
