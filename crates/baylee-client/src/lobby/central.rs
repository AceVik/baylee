//! Native login precedes gateway selection. Account secrets live on one bounded worker.
#![forbid(unsafe_code)]

use super::{Field, Lobby, LobbyEvent, LobbyState, Mailbox, Registration, Reply, http};
use baylee_account::{Account, Endpoint, Error};
use baylee_client_core::{i18n::Lang, prefs::Preferences};
use std::sync::{Arc, Mutex, mpsc};

type Job = Box<dyn FnOnce(&mut Account) + Send>;
type SaveResult = Arc<Mutex<Option<Result<(), Error>>>>;

struct PendingSave {
    gateways: Vec<String>,
    result: SaveResult,
}

/// Handle to a serial worker; the UI cannot read or print its tokens or keys.
#[derive(Clone)]
pub(crate) struct Handle(mpsc::SyncSender<Job>);

impl Handle {
    fn start(mut account: Account) -> Result<Self, Error> {
        let (send, receive) = mpsc::sync_channel::<Job>(4);
        std::thread::Builder::new()
            .name("central-account".into())
            .spawn(move || {
                while let Ok(job) = receive.recv() {
                    job(&mut account);
                }
                let _ = account.logout();
            })
            .map_err(|_| Error::Unavailable)?;
        Ok(Self(send))
    }

    pub(crate) fn save(
        &self,
        preferences: Option<serde_json::Value>,
        gateways: Option<Vec<String>>,
        done: impl FnOnce(Result<(), Error>) + Send + 'static,
    ) -> bool {
        self.0
            .try_send(Box::new(move |account| {
                done(account.save(preferences, gateways));
            }))
            .is_ok()
    }
}

pub(super) struct State {
    endpoint: Result<Endpoint, Error>,
    local_gateways: Vec<String>,
    pub(super) handle: Option<Handle>,
    saved_gateways: Vec<String>,
    saving: Option<PendingSave>,
    sync_failed: bool,
    chat_unlock: Option<crate::social::vault_access::Login>,
}

pub(super) struct Restored {
    handle: Handle,
    gateways: Vec<String>,
    preferences: Preferences,
    chat_unlock: Option<crate::social::vault_access::Login>,
}

pub(super) fn enable(state: &mut LobbyState) {
    let endpoint = endpoint();
    state.central = Some(State {
        endpoint,
        local_gateways: std::mem::take(&mut state.gateways),
        handle: None,
        saved_gateways: Vec::new(),
        saving: None,
        sync_failed: false,
        chat_unlock: None,
    });
    state.gateway_selected = false;
    state.gateway_cursor = None;
    state.probes.clear();
    state.lobby.set_gateway_ready(true);
    state.lobby.set_registration(Registration::Off);
    state.lobby.set_guests_enabled(false);
}

fn endpoint() -> Result<Endpoint, Error> {
    // Development is explicit and loopback-only; a bad override never falls back to live.
    #[cfg(feature = "dev-control")]
    if let Some(origin) = std::env::var_os("BAYLEE_CENTRAL_DEV_ENDPOINT") {
        return Endpoint::development(origin.to_str().ok_or(Error::Rejected)?);
    }
    Ok(Endpoint::live())
}

pub(super) fn login(state: &mut LobbyState, mailbox: &Mailbox, username: String, password: String) {
    state.lobby.clear_credentials();
    let Some(central) = &state.central else {
        return;
    };
    let endpoint = central.endpoint.clone();
    let gateways = central.local_gateways.clone();
    let target = mailbox.clone();
    let epoch = state.gateway_epoch;
    let already_signed_in = central.handle.is_some();
    let password = zeroize::Zeroizing::new(password);
    let launched = std::thread::Builder::new()
        .name("central-login".into())
        .spawn(move || {
            let result = (|| {
                if already_signed_in {
                    return Err(Error::Rejected);
                }
                let username = baylee_protocol::names::username(&username)
                    .map_err(|_| Error::Rejected)?
                    .key;
                let endpoint = endpoint?;
                let account =
                    Account::login(endpoint.clone(), username, password.to_string(), gateways)?;
                let chat_unlock = endpoint.legacy_origin().map(|gateway| {
                    crate::social::vault_access::Login::central(gateway, password.to_string())
                });
                let Ok(preferences) = serde_json::from_value(account.data().preferences.clone())
                else {
                    let _ = account.logout();
                    return Err(Error::Rejected);
                };
                let gateways = account.data().gateways.clone();
                Ok(Restored {
                    handle: Handle::start(account)?,
                    gateways,
                    preferences,
                    chat_unlock,
                })
            })();
            if let Ok(mut messages) = target.0.lock() {
                messages.push(Reply::Remote(epoch, Box::new(Reply::Central(result))));
            }
        });
    if launched.is_err() {
        state
            .lobby
            .apply(LobbyEvent::Failed(message(state.lobby.lang()).into()));
    }
}

pub(super) fn receive(
    state: &mut LobbyState,
    prefs: &mut crate::prefs::Prefs,
    mailbox: &Mailbox,
    result: Result<Restored, Error>,
) {
    match result {
        Ok(restored) => {
            let Some(central) = &mut state.central else {
                return;
            };
            central.saved_gateways.clone_from(&restored.gateways);
            central.handle = Some(restored.handle.clone());
            central.chat_unlock = restored.chat_unlock;
            prefs.attach_central(restored.handle, restored.preferences);
            let lang = state.lobby.lang();
            let username = state.lobby.field(Field::Username).to_string();
            state.lobby = Lobby::new();
            state.lobby.set_lang(lang);
            state.lobby.set_field(Field::Username, &username);
            state.lobby.set_gateway_ready(false);
            state.gateway_selected = false;
            state.gateways = restored.gateways;
            state.gateway_cursor = (!state.gateways.is_empty()).then_some(0);
            for url in state.unasked_gateways() {
                http::probe_gateway(url, mailbox);
            }
        }
        Err(_) => {
            state
                .lobby
                .apply(LobbyEvent::Failed(message(state.lobby.lang()).into()));
        }
    }
}

pub(super) fn admit(state: &mut LobbyState, mailbox: &Mailbox) {
    let Some(handle) = state.central.as_ref().and_then(|s| s.handle.clone()) else {
        return;
    };
    let gateway = state.gateway.clone();
    let mut chat_unlock = state
        .central
        .as_mut()
        .and_then(|s| s.chat_unlock.take())
        .filter(|login| login.fresh_for(&gateway));
    let epoch = state.gateway_epoch;
    let target = mailbox.clone();
    let lang = state.lobby.lang();
    state.lobby.say(if lang == Lang::De {
        "Verbindung zum Gateway wird hergestellt …"
    } else {
        "Connecting to gateway …"
    });
    let accepted = handle
        .0
        .try_send(Box::new(move |account| {
            let admission = account.admit(&gateway);
            let stale = admission
                .as_ref()
                .ok()
                .and_then(|value| value.get("terms_stale"))
                .and_then(serde_json::Value::as_bool);
            let event = match admission {
                Ok(value) => match (
                    value.get("token").and_then(serde_json::Value::as_str),
                    value.get("username").and_then(serde_json::Value::as_str),
                ) {
                    (Some(token), Some(username)) => LobbyEvent::LoggedIn {
                        token: token.into(),
                        username: Some(username.into()),
                    },
                    _ => LobbyEvent::Failed(message(lang).into()),
                },
                Err(_) => LobbyEvent::Failed(message(lang).into()),
            };
            if let Ok(mut messages) = target.0.lock() {
                if let Some(stale) = stale {
                    messages.push(Reply::Remote(
                        epoch,
                        Box::new(Reply::Terms(super::front::terms::TermsReply::Stale(stale))),
                    ));
                }
                if let LobbyEvent::LoggedIn { token, .. } = &event
                    && let Some(mut login) = chat_unlock.take()
                {
                    login.accept(token);
                    messages.push(Reply::Remote(epoch, Box::new(Reply::VaultLogin(login))));
                }
                messages.push(Reply::Remote(epoch, Box::new(Reply::Event(event))));
            }
        }))
        .is_ok();
    if !accepted {
        state.lobby.apply(LobbyEvent::Failed(message(lang).into()));
    }
}

pub(super) fn sync_gateways(
    mut state: bevy::prelude::ResMut<LobbyState>,
    prefs: bevy::prelude::Res<crate::prefs::Prefs>,
) {
    let Some(central) = &state.central else {
        return;
    };
    if central
        .chat_unlock
        .as_ref()
        .is_some_and(|login| !login.fresh_for("https://baylee.acevik.de"))
    {
        if let Some(central) = &mut state.central {
            central.chat_unlock = None;
        }
        return;
    }
    let Some(handle) = &central.handle else {
        return;
    };
    if central.sync_failed {
        return;
    }
    if prefs.central_save_failed {
        if let Some(central) = &mut state.central {
            central.sync_failed = true;
        }
        let message = sync_message(state.lobby.lang());
        state.lobby.say(message);
        return;
    }
    if let Some(pending) = &central.saving {
        let result = pending
            .result
            .lock()
            .ok()
            .and_then(|mut result| result.take());
        if let Some(result) = result {
            let Some(central) = &mut state.central else {
                return;
            };
            let pending = central.saving.take();
            if result.is_err() {
                central.sync_failed = true;
                let message = sync_message(state.lobby.lang());
                state.lobby.say(message);
            } else if let Some(pending) = pending {
                central.saved_gateways = pending.gateways;
            }
        }
        return;
    }
    if central.saved_gateways == state.gateways {
        return;
    }
    let gateways = state.gateways.clone();
    let result = Arc::new(Mutex::new(None));
    let done = result.clone();
    if handle.save(None, Some(gateways.clone()), move |reply| {
        if let Ok(mut value) = done.lock() {
            *value = Some(reply);
        }
    }) && let Some(central) = &mut state.central
    {
        central.saving = Some(PendingSave { gateways, result });
    }
}

fn message(lang: Lang) -> &'static str {
    if lang == Lang::De {
        "Anmeldung oder Wiederherstellung fehlgeschlagen. Bitte Zugangsdaten und Verbindung prüfen."
    } else {
        "Sign-in or restore failed. Please check your credentials and connection."
    }
}

fn sync_message(lang: Lang) -> &'static str {
    if lang == Lang::De {
        "Kontodaten konnten nicht synchronisiert werden. Deine lokalen Änderungen bleiben erhalten. Bitte melde dich erneut an."
    } else {
        "Account synchronization failed. Your local changes are retained; please sign in again."
    }
}

pub(super) fn logout(state: &mut LobbyState, prefs: &mut crate::prefs::Prefs) {
    state.gateway_epoch = state.gateway_epoch.wrapping_add(1);
    let lang = state.lobby.lang();
    state.lobby = Lobby::new();
    state.lobby.set_lang(lang);
    state.offline = None;
    state.gateway_selected = false;
    prefs.detach_central();
    enable(state);
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_client_core::lobby::{LobbyRequest, gateway_info::Probe};

    #[test]
    fn central_login_hides_and_never_probes_device_gateways_before_restore() {
        let mut state = LobbyState::from_settings(crate::settings::ClientSettings::default());
        state.gateways = vec!["https://private-device-gateway.invalid".into()];
        enable(&mut state);
        assert!(state.central_pending());
        assert_eq!(state.gateways, Vec::<String>::new());
        assert_eq!(state.unasked_gateways(), Vec::<String>::new());
        assert!(!state.select_gateway(0));
        state.leave_gateway();
        assert!(
            state.lobby.gateway_chosen(),
            "back must not bypass central authentication"
        );
        state.lobby.set_field(Field::Username, "synthetic");
        state.lobby.set_field(Field::Password, "synthetic password");
        assert!(matches!(
            state.lobby.submit(),
            Some(LobbyRequest::LogIn { .. })
        ));
    }

    #[test]
    fn restored_gateways_replace_local_defaults_and_signout_closes_account_access() {
        let mut state = LobbyState::from_settings(crate::settings::ClientSettings::default());
        enable(&mut state);
        let source = "http://127.0.0.1:28766".to_string();
        // No real HTTP work: this synthetic public probe has already completed.
        state.probes.insert(source.clone(), Probe::Older);
        let (send, _receive) = mpsc::sync_channel::<Job>(4);
        let mut prefs = crate::prefs::Prefs::default();
        let restored = Restored {
            handle: Handle(send),
            gateways: vec![source.clone()],
            preferences: Preferences::default(),
            chat_unlock: None,
        };
        receive(&mut state, &mut prefs, &Mailbox::default(), Ok(restored));
        assert!(state.central_ready());
        assert!(!state.lobby.gateway_chosen());
        assert_eq!(state.gateways, vec![source]);
        let previous = state.gateway_epoch;
        logout(&mut state, &mut prefs);
        assert!(state.central_pending());
        assert!(
            state.gateway_epoch > previous,
            "late gateway replies must be invalidated"
        );
        assert_eq!(state.gateways, Vec::<String>::new());
    }
}
