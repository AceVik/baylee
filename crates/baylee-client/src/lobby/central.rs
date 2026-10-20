//! Native login precedes gateway selection. Account secrets live on one bounded worker.
#![forbid(unsafe_code)]

use super::{Field, Lobby, LobbyEvent, LobbyState, Mailbox, Registration, Reply, http};
use baylee_account::{Account, Endpoint, Error};
use baylee_client_core::{Phrase, i18n::Lang, prefs::Preferences};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc,
};

type Job = Box<dyn FnOnce(&mut Account) + Send>;
type SaveResult = Arc<Mutex<Option<Result<(), Error>>>>;

struct PendingSave {
    gateways: Vec<String>,
    result: SaveResult,
}

/// Handle to a serial worker; the UI cannot read or print its tokens or keys.
#[derive(Clone)]
pub(crate) struct Handle {
    send: mpsc::SyncSender<Job>,
    active: Arc<AtomicBool>,
}

impl Handle {
    #[cfg(test)]
    pub(crate) fn unavailable_for_test() -> Self {
        let (send, receive) = mpsc::sync_channel(1);
        drop(receive);
        Self {
            send,
            active: Arc::new(AtomicBool::new(true)),
        }
    }
    pub(crate) fn is_active(&self) -> bool {
        self.active.load(Ordering::Acquire)
    }
    /// Enqueue bounded account work without exposing SDK keys or tokens to the UI.
    pub(crate) fn submit(&self, operation: impl FnOnce(&mut Account) + Send + 'static) -> bool {
        self.active.load(Ordering::Acquire) && self.send.try_send(Box::new(operation)).is_ok()
    }

    pub(crate) fn chat_history(
        &self,
        device: baylee_account::chat::ChatDevice,
    ) -> Result<std::collections::BTreeMap<String, Vec<u8>>, Error> {
        self.call(move |account| account.read_chat_history(&device))
    }
    /// Blocking adapter for an existing background job; never called by a render system.
    pub(crate) fn chat_devices(
        &self,
        gateway: String,
        tag: String,
    ) -> Result<Vec<baylee_account::chat::ChatDevice>, Error> {
        self.call(move |account| account.chat_devices(&gateway, &tag))
    }

    pub(crate) fn archive_chat_device(
        &self,
        device: baylee_account::chat::ChatDevice,
    ) -> Result<(), Error> {
        self.call(move |account| account.archive_chat_device(&device))
    }

    pub(crate) fn retire_chat_device(
        &self,
        device: baylee_account::chat::ChatDevice,
    ) -> Result<(), Error> {
        self.call(move |account| account.retire_chat_device(&device))
    }

    pub(crate) fn remember_chat_installation(
        &self,
        device: baylee_account::chat::ChatDevice,
        installation: String,
    ) -> Result<(), Error> {
        self.call(move |account| account.remember_chat_installation(&device, installation))
    }

    pub(crate) fn upload_chat_history(
        &self,
        device: baylee_account::chat::ChatDevice,
        files: std::collections::BTreeMap<String, Vec<u8>>,
    ) -> Result<(), Error> {
        self.call(move |account| account.upload_chat_history(&device, &files))
    }

    fn call<T: Send + 'static>(
        &self,
        operation: impl FnOnce(&mut Account) -> Result<T, Error> + Send + 'static,
    ) -> Result<T, Error> {
        let (send, receive) = mpsc::sync_channel(1);
        if !self.active.load(Ordering::Acquire) {
            return Err(Error::Rejected);
        }
        self.send
            .try_send(Box::new(move |account| {
                let _ = send.send(operation(account));
            }))
            .map_err(|_| Error::Busy)?;
        receive.recv().map_err(|_| Error::Unavailable)?
    }

    fn start(mut account: Account) -> Result<Self, Error> {
        let (send, receive) = mpsc::sync_channel::<Job>(4);
        let active = Arc::new(AtomicBool::new(true));
        let worker_active = active.clone();
        std::thread::Builder::new()
            .name("central-account".into())
            .spawn(move || {
                let interval = std::time::Duration::from_secs(10);
                let mut checked = std::time::Instant::now();
                loop {
                    if checked.elapsed() >= interval {
                        if matches!(account.check_active(), Err(Error::Rejected)) {
                            worker_active.store(false, Ordering::Release);
                            break;
                        }
                        checked = std::time::Instant::now();
                    }
                    match receive.recv_timeout(interval.saturating_sub(checked.elapsed())) {
                        Ok(job) => job(&mut account),
                        Err(mpsc::RecvTimeoutError::Timeout) => {}
                        Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    }
                }
                let _ = account.logout();
            })
            .map_err(|_| Error::Unavailable)?;
        Ok(Self { send, active })
    }

    pub(crate) fn save(
        &self,
        preferences: Option<serde_json::Value>,
        gateways: Option<Vec<String>>,
        done: impl FnOnce(Result<(), Error>) + Send + 'static,
    ) -> bool {
        if !self.active.load(Ordering::Acquire) {
            return false;
        }
        self.send
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
    chat_unlock: Arc<Mutex<Option<crate::social::vault_access::Login>>>,
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
        chat_unlock: Arc::new(Mutex::new(None)),
    });
    state.gateway_selected = false;
    state.gateway_cursor = None;
    state.probes.clear();
    state.lobby.set_gateway_ready(true);
    state.lobby.set_registration(Registration::Off);
    state.lobby.set_guests_enabled(false);
}

#[cfg_attr(not(feature = "dev-control"), allow(clippy::unnecessary_wraps))] // The dev override can fail.
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
                messages.push(Reply::Remote(
                    epoch,
                    Box::new(Reply::Central(result.map(Box::new))),
                ));
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
    result: Result<Box<Restored>, Error>,
) {
    match result {
        Ok(restored) => {
            let restored = *restored;
            let Some(central) = &mut state.central else {
                return;
            };
            central.saved_gateways.clone_from(&restored.gateways);
            central.handle = Some(restored.handle.clone());
            central.chat_unlock = Arc::new(Mutex::new(restored.chat_unlock));
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
    let chat_unlock = state.central.as_ref().map(|s| s.chat_unlock.clone());
    let epoch = state.gateway_epoch;
    let target = mailbox.clone();
    let lang = state.lobby.lang();
    state.lobby.say(if lang == Lang::De {
        "Verbindung zum Gateway wird hergestellt …"
    } else {
        "Connecting to gateway …"
    });
    let accepted = handle
        .send
        .try_send(Box::new(move |account| {
            let admission = account.admit(&gateway);
            let stale = admission.as_ref().ok().and_then(|value| value.terms_stale);
            let central_stale = admission
                .as_ref()
                .ok()
                .and_then(|value| value.central_terms_stale)
                == Some(true);
            let event = match admission {
                Ok(mut value) => LobbyEvent::LoggedIn {
                    token: std::mem::take(&mut value.token),
                    username: value.username.take(),
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
                // Central's own terms come first: their sheet, then the
                // gateway's after a fresh admission.
                if central_stale {
                    messages.push(Reply::Remote(
                        epoch,
                        Box::new(Reply::Terms(super::front::terms::TermsReply::CentralStale)),
                    ));
                }
                if let LobbyEvent::LoggedIn { token, .. } = &event
                    && let Some(mut login) = chat_unlock
                        .as_ref()
                        .and_then(|slot| slot.lock().ok().and_then(|mut value| value.take()))
                        .filter(|login| login.fresh_for(&gateway))
                {
                    login.accept(token);
                    messages.push(Reply::Remote(epoch, Box::new(Reply::VaultLogin(login))));
                }
                let landed = matches!(event, LobbyEvent::LoggedIn { .. });
                messages.push(Reply::Remote(epoch, Box::new(Reply::Event(event))));
                if landed {
                    messages.push(Reply::Remote(
                        epoch,
                        Box::new(Reply::Terms(super::front::terms::TermsReply::CentralBound)),
                    ));
                }
            }
        }))
        .is_ok();
    if !accepted {
        state.lobby.apply(LobbyEvent::Failed(message(lang).into()));
    }
}

pub(super) fn sync_gateways(
    mut state: bevy::prelude::ResMut<LobbyState>,
    mut prefs: bevy::prelude::ResMut<crate::prefs::Prefs>,
) {
    if state
        .central_handle()
        .is_some_and(|handle| !handle.active.load(Ordering::Acquire))
    {
        logout(&mut state, &mut prefs);
        return;
    }
    let Some(central) = &state.central else {
        return;
    };
    if let Ok(mut login) = central.chat_unlock.lock()
        && login
            .as_ref()
            .is_some_and(|login| !login.fresh_for("https://baylee.acevik.de"))
    {
        *login = None;
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
    Phrase::CentralSignInFailed.text(lang)
}

fn sync_message(lang: Lang) -> &'static str {
    Phrase::CentralSyncFailed.text(lang)
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
    fn central_account_deletion_never_sends_a_password_to_the_gateway() {
        let sink = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        sink.set_nonblocking(true).unwrap();
        let mut state = LobbyState::from_settings(crate::settings::ClientSettings::default());
        enable(&mut state);
        state.gateway = format!("http://{}", sink.local_addr().unwrap());
        let mailbox = Mailbox::default();
        http::dispatch(
            &mut state,
            &mailbox,
            Some(LobbyRequest::DeleteAccount {
                password: Some("synthetic central-only password".into()),
            }),
        );
        assert!(
            matches!(sink.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock)
        );
        assert!(mailbox.0.lock().unwrap().is_empty());
        assert!(state.central_pending());
        assert!(state.lobby.status().contains("Central verification"));
    }

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
            handle: Handle {
                send,
                active: Arc::new(AtomicBool::new(true)),
            },
            gateways: vec![source.clone()],
            preferences: Preferences::default(),
            chat_unlock: None,
        };
        receive(
            &mut state,
            &mut prefs,
            &Mailbox::default(),
            Ok(Box::new(restored)),
        );
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
