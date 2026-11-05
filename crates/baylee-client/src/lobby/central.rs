//! Baylee Central sign-in, for a gateway that takes it (`central` in its
//! `GET /auth/config`): choosing one turns the account form into Central's,
//! and the sign-in admits the player to that gateway. Every other gateway
//! signs people in itself. Account secrets live on one bounded worker.
#![forbid(unsafe_code)]

use super::{Field, Lobby, LobbyEvent, LobbyState, Mailbox, Registration, Reply, http};
use baylee_account::{Account, Endpoint, Error};
use baylee_client_core::{Phrase, i18n::Lang, lobby::gateway_list, prefs::Preferences};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc,
};

pub(crate) type Job = Box<dyn FnOnce(&mut Account) + Send>;
type SaveResult = Arc<Mutex<Option<Result<(), Error>>>>;

/// How often the account worker renews its Central session: well inside the
/// 30 minutes one renewal grants.
const RENEW_EVERY: std::time::Duration = std::time::Duration::from_mins(10);
/// How soon a failed renewal is tried again (a Central restarting, or not
/// renewing yet).
const RENEW_RETRY: std::time::Duration = std::time::Duration::from_mins(1);

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
    /// A handle whose jobs the test receives, instead of a worker.
    #[cfg(test)]
    pub(crate) fn queued_for_test() -> (Self, mpsc::Receiver<Job>) {
        let (send, receive) = mpsc::sync_channel(4);
        let handle = Self {
            send,
            active: Arc::new(AtomicBool::new(true)),
        };
        (handle, receive)
    }
    pub(crate) fn is_active(&self) -> bool {
        self.active.load(Ordering::Acquire)
    }
    /// Enqueue bounded account work without exposing SDK keys or tokens to the UI.
    pub(crate) fn submit(&self, operation: impl FnOnce(&mut Account) + Send + 'static) -> bool {
        self.active.load(Ordering::Acquire) && self.send.try_send(Box::new(operation)).is_ok()
    }

    pub(crate) fn chat_identity(&self) -> Result<baylee_account::chat::DeviceKey, Error> {
        self.call(|account| account.chat_identity())
    }

    pub(crate) fn chat_history(
        &self,
        device: baylee_account::chat::ChatDevice,
    ) -> Result<std::collections::BTreeMap<String, Vec<u8>>, Error> {
        self.call(move |account| account.read_chat_history(&device))
    }
    /// Blocking adapter for an existing background job; never called by a render system.
    /// The live gateway's records are filed under the address its keys are
    /// bound to (`gateway_list::keyed`), by every client before and after its move.
    pub(crate) fn chat_devices(
        &self,
        gateway: String,
        tag: String,
    ) -> Result<Vec<baylee_account::chat::ChatDevice>, Error> {
        let gateway = gateway_list::keyed(&gateway).to_string();
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

    /// The key of the local chat vault with `scope`, from the account's
    /// master key (`docs/password-vault.md` §"The account key").
    pub(crate) fn local_vault_key(
        &self,
        scope: [u8; 32],
    ) -> Result<zeroize::Zeroizing<[u8; 32]>, Error> {
        self.call(move |account| account.local_vault_key(&scope))
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
        let (send, receive) = mpsc::sync_channel::<Job>(64);
        let active = Arc::new(AtomicBool::new(true));
        let worker_active = active.clone();
        std::thread::Builder::new()
            .name("central-account".into())
            .spawn(move || {
                let interval = std::time::Duration::from_secs(10);
                let mut checked = std::time::Instant::now();
                let mut renew_at = std::time::Instant::now() + RENEW_EVERY;
                loop {
                    if checked.elapsed() >= interval {
                        if matches!(account.check_active(), Err(Error::Rejected)) {
                            worker_active.store(false, Ordering::Release);
                            break;
                        }
                        checked = std::time::Instant::now();
                    }
                    // The session, and every game admitted from it, lasts while
                    // the client runs (Central caps it at 12 hours). A failed
                    // renewal is tried again a minute later and ends nothing:
                    // the check above says whether the session lives.
                    if std::time::Instant::now() >= renew_at {
                        let next = if account.renew().is_ok() {
                            RENEW_EVERY
                        } else {
                            RENEW_RETRY
                        };
                        renew_at = std::time::Instant::now() + next;
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

#[allow(clippy::struct_excessive_bools)]
pub(crate) struct State {
    endpoint: Result<Endpoint, Error>,
    pub(super) local_gateways: Vec<String>,
    pub(super) handle: Option<Handle>,
    saved_gateways: Vec<String>,
    saving: Option<PendingSave>,
    sync_failed: bool,
    /// The gateway that asked for Central sign-in, admitted to once it is
    /// done; `None` when Central was turned on without one.
    bound: Option<String>,
    /// That gateway's own guest door, and who it lets in new, when it takes
    /// guests too (`auth_mode` `mixed`): Central's form offers it beside the
    /// sign-in ([`guest_instead`]).
    guest_door: Option<Registration>,
    /// A `baylee:` link opened this sign-in (`lobby::links`): `‹` goes back
    /// to the device's gateways, as from a gateway that asked for it.
    linked: bool,
    /// Central's own terms wait for the signed-in account: their sheet is
    /// up before any gateway, nothing is admitted or synced (Central
    /// refuses tickets and writes until they are accepted).
    pub(super) terms_pending: bool,
    /// A Central ticket is being exchanged with the chosen gateway.
    pub(super) admitting: bool,
    /// The Central account's `DisplayName#Tag` (or username), shown in the
    /// Lobby profile pill even before a gateway is selected.
    pub(crate) profile_handle: Option<String>,
}

pub(super) struct Restored {
    handle: Handle,
    gateways: Vec<String>,
    preferences: Preferences,
    /// The sign-in found Central's terms in force unaccepted (and a gateway
    /// account's move, if any, waiting for them).
    terms_pending: bool,
    profile_handle: Option<String>,
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
        bound: None,
        guest_door: None,
        linked: false,
        terms_pending: false,
        admitting: false,
        profile_handle: None,
    });
    state.gateway_selected = false;
    state.left_central_bound = false;
    state.gateway_cursor = None;
    state.probes.clear();
    state.lobby.set_gateway_ready(true);
    state.lobby.set_registration(Registration::Off);
    state.lobby.set_guests_enabled(false);
}

impl State {
    /// Whether `‹` leads back from this sign-in to the device's gateways:
    /// a gateway asked for it, or a link opened it.
    pub(super) fn has_way_back(&self) -> bool {
        self.bound.is_some() || self.linked
    }
}

/// A `baylee:` link asks for Central's sign-in (`lobby::links`): the form
/// the official gateway would open, with `‹` back to the device's own
/// gateways and no gateway entered afterwards; the account's list is shown
/// instead. Nothing when Central is on already.
pub(super) fn from_link(state: &mut LobbyState) {
    if state.central.is_some() {
        return;
    }
    let probes = std::mem::take(&mut state.probes);
    enable(state);
    state.probes = probes;
    if let Some(central) = &mut state.central {
        central.linked = true;
    }
}

/// The chosen gateway takes Central sign-ins: the account form becomes
/// Central's, and the sign-in admits the player to that gateway. A gateway
/// that takes guests too keeps its guest door on that form. Nothing when
/// Central is on already, or a session on the gateway is.
pub(super) fn bound(state: &mut LobbyState) {
    if state.central.is_some() || !state.gateway_selected || state.lobby.token().is_some() {
        return;
    }
    let gateway = state.gateway.clone();
    let guest_door = state
        .lobby
        .guests_enabled()
        .then(|| state.lobby.registration());
    let probes = std::mem::take(&mut state.probes);
    enable(state);
    state.probes = probes;
    if let Some(central) = &mut state.central {
        central.bound = Some(gateway);
        central.guest_door = guest_door;
    }
    if guest_door.is_some() {
        state.lobby.set_guests_enabled(true);
    }
}

/// Play as guest from Central's form that a gateway taking guests too
/// asked for: Central is off again, the device's own gateways are back, and
/// that gateway stays chosen with its guest door as it offered it, so the
/// guest comes in as on any gateway that takes guests. `false`, and nothing
/// changed, when the form offers no guest door.
pub(super) fn guest_instead(state: &mut LobbyState) -> bool {
    let Some(registration) = state
        .central
        .as_ref()
        .filter(|central| central.handle.is_none() && central.bound.is_some())
        .and_then(|central| central.guest_door)
    else {
        return false;
    };
    let Some(central) = state.central.take() else {
        return false;
    };
    state.gateways = central.local_gateways;
    state.gateway_selected = true;
    state.lobby.set_gateway_ready(true);
    state.lobby.set_registration(registration);
    state.lobby.set_guests_enabled(true);
    true
}

/// Back from Central's sign-in that a gateway (or a link) asked for:
/// Central is off again and the device's own gateways are back. Central
/// turned on without either has no way back.
pub(super) fn leave(state: &mut LobbyState) {
    if state.central.as_ref().is_some_and(State::has_way_back) {
        close(state);
    }
}

/// Central off, the device's own gateways back, the gateway form shown.
fn close(state: &mut LobbyState) {
    let Some(central) = state.central.take() else {
        return;
    };
    state.gateway_epoch = state.gateway_epoch.wrapping_add(1);
    state.gateways = central.local_gateways;
    state.gateway_selected = false;
    state.left_central_bound = true;
    state.gateway_cursor = (!state.gateways.is_empty()).then_some(0);
    state.lobby.set_gateway_ready(false);
}

/// How many old local chat vaults a Central sign-in tries to open with its
/// password, one Argon2id run each (`attach_account_keys`).
const ATTACH_LIMIT: usize = 8;

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
    let gateways = gateway_list::filed(&central.local_gateways);
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
                let live = endpoint == Endpoint::live();
                let account = Account::login(
                    endpoint.clone(),
                    username.clone(),
                    password.to_string(),
                    gateways,
                )?;
                // The password's last use: every old local chat vault it
                // opens gains the account's wrapper, so no later chat start
                // needs it (`docs/password-vault.md` §"The account key").
                // Only the live Central's: a development one would wrap the
                // device's real vaults under its own issuer's key.
                if live && let Some(root) = crate::social::config_dir() {
                    baylee_client_core::social::vault::password::attach_account_keys(
                        &root,
                        &password,
                        |scope| account.local_vault_key(scope.as_bytes()).ok(),
                        ATTACH_LIMIT,
                    );
                }
                drop(password);
                let Ok(preferences) = serde_json::from_value(account.data().preferences.clone())
                else {
                    let _ = account.logout();
                    return Err(Error::Rejected);
                };
                let gateways = account.data().gateways.clone();
                // Central's own terms come before any gateway: a sign-in
                // that finds them unaccepted shows them first.
                let terms_pending = account.central_terms_open();
                let profile_handle = account.profile_handle().or_else(|| Some(username.clone()));
                Ok(Restored {
                    handle: Handle::start(account)?,
                    gateways,
                    preferences,
                    terms_pending,
                    profile_handle,
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
            central.terms_pending = restored.terms_pending;
            central.profile_handle.clone_from(&restored.profile_handle);
            prefs.attach_central(restored.handle, restored.preferences);
            let lang = state.lobby.lang();
            let username = state.lobby.field(Field::Username).to_string();
            let me_handle = restored.profile_handle.or_else(|| {
                let trimmed = username.trim();
                (!trimmed.is_empty()).then(|| trimmed.to_string())
            });
            state.lobby = Lobby::new();
            state.lobby.set_lang(lang);
            state.lobby.set_field(Field::Username, &username);
            if let Some(handle) = me_handle {
                state.lobby.set_me(baylee_client_core::lobby::strips::Me {
                    handle,
                    guest: false,
                });
            }
            state.lobby.set_gateway_ready(false);
            state.lobby.set_registration(Registration::Off);
            state.lobby.set_guests_enabled(false);
            state.gateway_selected = false;
            state.left_central_bound = false;
            state.gateway_filter = baylee_client_core::lobby::gateway_list::GatewayFilter::All;
            state.gateway_sort = baylee_client_core::lobby::gateway_list::GatewaySort::LastUsed;
            state.gateway_collapsed = [false, true, true];
            // The document names the live gateway by its first address
            // (`gateway_list::filed`); this client shows it by today's.
            state.gateways = gateway_list::renamed(restored.gateways);
            let visible = state.visible_gateway_indices();
            state.gateway_cursor = visible.first().copied();
            for url in state.unasked_gateways() {
                http::probe_gateway(url, mailbox);
            }
            // The gateway that asked for this sign-in is where the player
            // was going: it joins the account's list and is entered, after
            // Central's own terms where they wait (`front::terms`).
            let bound = state
                .central
                .as_ref()
                .and_then(|central| central.bound.clone());
            if let Some(gateway) = bound {
                if !state.gateways.contains(&gateway) {
                    state.gateways.push(gateway.clone());
                }
                if let Some(index) = state.gateways.iter().position(|g| g == &gateway)
                    && state.select_gateway(index)
                {
                    http::probe_registration(state, mailbox);
                    if !restored.terms_pending {
                        admit(state, mailbox);
                    }
                }
            }
        }
        Err(_) => {
            state
                .lobby
                .apply(LobbyEvent::Failed(message(state.lobby.lang()).into()));
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn accept_guest_terms_if_needed(base: &str, token: &str) {
    #[derive(serde::Deserialize)]
    struct TermsDoc {
        version: String,
    }
    let get = ehttp::Request::get(format!("{base}/terms"));
    let Ok(resp) = crate::transport::fetch_blocking(&get) else {
        return;
    };
    if !(200..300).contains(&resp.status) {
        return;
    }
    let Ok(doc) = serde_json::from_slice::<TermsDoc>(&resp.bytes) else {
        return;
    };
    let Ok(body) = serde_json::to_vec(&serde_json::json!({ "terms_version": doc.version })) else {
        return;
    };
    let mut post = ehttp::Request::post(format!("{base}/account/terms"), body);
    post.headers.insert("Content-Type", "application/json");
    post.headers
        .insert("Authorization", format!("Bearer {token}"));
    let _ = crate::transport::fetch_blocking(&post);
}

#[cfg(not(target_arch = "wasm32"))]
fn sanitize_guest_display_name(raw: &str) -> String {
    let filtered: String = raw
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .take(16)
        .collect();
    let trimmed = filtered.trim_matches(['_', '-']);
    if (3..=16).contains(&trimmed.len()) {
        trimmed.to_string()
    } else {
        "Player".to_string()
    }
}

#[cfg(not(target_arch = "wasm32"))]
enum GuestFallbackError {
    NoGuests,
    Unreachable,
    Other,
}

#[cfg(not(target_arch = "wasm32"))]
fn admit_guest_fallback(
    gateway: &str,
    display_name: &str,
    lang: &str,
    kept: Option<baylee_client_core::lobby::KeptGuest>,
) -> Result<baylee_client_core::lobby::KeptGuest, GuestFallbackError> {
    #[derive(serde::Deserialize)]
    struct GuestBody {
        token: String,
        handle: String,
        #[serde(default)]
        terms_stale: Option<bool>,
    }
    let base = gateway.trim_end_matches('/');
    if base.is_empty() {
        return Err(GuestFallbackError::Unreachable);
    }
    if let Some(kept) = kept {
        let mut check = ehttp::Request::get(format!("{base}/me"));
        check
            .headers
            .insert("Authorization", format!("Bearer {}", kept.token));
        if let Ok(resp) = crate::transport::fetch_blocking(&check) {
            if (200..300).contains(&resp.status) {
                return Ok(kept);
            }
            if resp.status == 403 {
                accept_guest_terms_if_needed(base, &kept.token);
                if let Ok(retry) = crate::transport::fetch_blocking(&check)
                    && (200..300).contains(&retry.status)
                {
                    return Ok(kept);
                }
            }
        }
    }
    let clean_name = sanitize_guest_display_name(display_name);
    let post_guest = |payload: serde_json::Value| -> Result<ehttp::Response, GuestFallbackError> {
        let bytes = serde_json::to_vec(&payload).map_err(|_| GuestFallbackError::Other)?;
        let mut req = ehttp::Request::post(format!("{base}/auth/guest"), bytes);
        req.headers.insert("Content-Type", "application/json");
        crate::transport::fetch_blocking(&req).map_err(|_| GuestFallbackError::Unreachable)
    };
    let mut response = post_guest(serde_json::json!({
        "display_name": clean_name,
        "lang": lang,
    }))?;
    if response.status == 400 {
        response = post_guest(serde_json::json!({ "lang": lang }))?;
    }
    if !(200..300).contains(&response.status) {
        if response.status == 403
            && response
                .text()
                .is_some_and(|t| t.contains("takes no guests"))
        {
            return Err(GuestFallbackError::NoGuests);
        }
        return Err(GuestFallbackError::Other);
    }
    let guest: GuestBody =
        serde_json::from_slice(&response.bytes).map_err(|_| GuestFallbackError::Other)?;
    if guest.terms_stale == Some(true) {
        accept_guest_terms_if_needed(base, &guest.token);
    }
    Ok(baylee_client_core::lobby::KeptGuest {
        token: guest.token,
        handle: guest.handle,
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn gateway_reachable(gateway: &str) -> bool {
    let base = gateway.trim_end_matches('/');
    if base.is_empty() {
        return false;
    }
    let req = ehttp::Request::get(format!("{base}/info"));
    crate::transport::fetch_blocking(&req).is_ok_and(|resp| (200..500).contains(&resp.status))
}

#[cfg(not(target_arch = "wasm32"))]
fn fallback_event(
    gateway: &str,
    display_name: &str,
    lang_code: &str,
    lang: Lang,
    kept_guest: Option<baylee_client_core::lobby::KeptGuest>,
) -> LobbyEvent {
    match admit_guest_fallback(gateway, display_name, lang_code, kept_guest) {
        Ok(guest) => LobbyEvent::GuestIn(guest),
        Err(GuestFallbackError::NoGuests) => LobbyEvent::Failed(Phrase::NoGuests.text(lang).into()),
        Err(GuestFallbackError::Unreachable) => {
            LobbyEvent::Failed(Phrase::GatewayNotAnswering.text(lang).into())
        }
        Err(GuestFallbackError::Other) => {
            if gateway_reachable(gateway) {
                LobbyEvent::Failed(message(lang).into())
            } else {
                LobbyEvent::Failed(Phrase::GatewayNotAnswering.text(lang).into())
            }
        }
    }
}

pub(super) fn admit(state: &mut LobbyState, mailbox: &Mailbox) {
    let Some(handle) = state.central.as_ref().and_then(|s| s.handle.clone()) else {
        return;
    };
    if let Some(central) = &mut state.central {
        central.admitting = true;
    }
    let gateway = state.gateway.clone();
    let epoch = state.gateway_epoch;
    let target = mailbox.clone();
    let lang = state.lobby.lang();
    let lang_code = state.lang.clone();
    let kept_guest = state.guests.get(&gateway).cloned();
    let display_name = state
        .central
        .as_ref()
        .and_then(|c| c.profile_handle.as_deref())
        .or_else(|| state.lobby.me().map(|m| m.handle.as_str()))
        .and_then(|h| h.split('#').next())
        .map(str::trim)
        .filter(|s| s.len() >= 2)
        .unwrap_or("Player")
        .to_string();
    state.lobby.tell(Phrase::GatewayConnecting, &[]);
    let fallback_gateway = gateway.clone();
    let fallback_name = display_name.clone();
    let fallback_lang = lang_code.clone();
    let fallback_kept = kept_guest.clone();
    let fallback_target = target.clone();
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
                Err(_) => fallback_event(&gateway, &display_name, &lang_code, lang, kept_guest),
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
                let central_bound = matches!(event, LobbyEvent::LoggedIn { .. });
                messages.push(Reply::Remote(epoch, Box::new(Reply::Event(event))));
                if central_bound {
                    messages.push(Reply::Remote(
                        epoch,
                        Box::new(Reply::Terms(super::front::terms::TermsReply::CentralBound)),
                    ));
                }
            }
        }))
        .is_ok();
    if !accepted {
        let _ = std::thread::Builder::new()
            .name("baylee-guest-admit".into())
            .spawn(move || {
                let event = fallback_event(
                    &fallback_gateway,
                    &fallback_name,
                    &fallback_lang,
                    lang,
                    fallback_kept,
                );
                if let Ok(mut messages) = fallback_target.0.lock() {
                    messages.push(Reply::Remote(epoch, Box::new(Reply::Event(event))));
                }
            });
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
    let Some(handle) = &central.handle else {
        return;
    };
    // Central refuses writes until its own terms are accepted.
    if central.sync_failed || central.terms_pending {
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
    let gateways = gateway_list::filed(&state.gateways);
    if central.saved_gateways == gateways {
        return;
    }
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

/// Whether Central's own terms wait for the signed-in account, before any
/// gateway: their sheet stands until they are accepted or put off.
pub(super) fn terms_pending(state: &LobbyState) -> bool {
    state
        .central
        .as_ref()
        .is_some_and(|central| central.terms_pending)
}

/// Central's own terms accepted (and a move that waited for them done):
/// nothing waits any more, and the account's list syncs again.
pub(super) fn terms_accepted(state: &mut LobbyState) {
    if let Some(central) = &mut state.central {
        central.terms_pending = false;
    }
}

/// Terms accepted at Central, its own or a gateway's: the chosen gateway is
/// signed in to afresh, which finds them accepted. Nothing to enter when no
/// gateway is chosen yet.
pub(super) fn readmit(state: &mut LobbyState, mailbox: &Mailbox) {
    if state.gateway_selected {
        admit(state, mailbox);
    }
}

/// A gateway account's move, held for Central's terms, failed once they
/// were accepted: signed out of Central as a failed sign-in is, and said
/// so. Signing in again resumes the move.
pub(super) fn migration_failed(state: &mut LobbyState, prefs: &mut crate::prefs::Prefs) {
    logout(state, prefs);
    state
        .lobby
        .apply(LobbyEvent::Failed(message(state.lobby.lang()).into()));
}

/// No gateway entered, signed in at Central: the account's list stands, with
/// the caret in it.
pub(super) fn back_to_list(state: &mut LobbyState) {
    state.gateway_selected = false;
    state.lobby.set_gateway_ready(false);
}

pub(super) fn leave_gateway_to_central(state: &mut LobbyState, mailbox: &Mailbox) {
    let flush = state.lobby.flush_delete();
    super::http::dispatch(state, mailbox, flush);
    state.offline = None;
    let out = state.lobby.sign_out();
    super::http::dispatch(state, mailbox, out);
    state.gateway_epoch = state.gateway_epoch.wrapping_add(1);
    state.gateway_selected = false;
    state.left_central_bound = false;
    state.hub = super::Hub::Play;
    state.lobby.set_gateway_ready(false);
    state.lobby.set_registration(Registration::Off);
    state.lobby.set_guests_enabled(false);
    if let Some(handle) = state
        .central
        .as_ref()
        .and_then(|c| c.profile_handle.clone())
        .or_else(|| {
            let u = state.lobby.field(Field::Username).trim();
            (!u.is_empty()).then(|| u.to_string())
        })
    {
        state.lobby.set_me(baylee_client_core::lobby::strips::Me {
            handle,
            guest: false,
        });
    }
    let visible = state.visible_gateway_indices();
    state.gateway_cursor = visible.first().copied();
}

pub(super) fn logout(state: &mut LobbyState, prefs: &mut crate::prefs::Prefs) {
    state.gateway_epoch = state.gateway_epoch.wrapping_add(1);
    let lang = state.lobby.lang();
    state.lobby = Lobby::new();
    state.lobby.set_lang(lang);
    state.offline = None;
    state.local_decks = None;
    prefs.detach_central();
    close(state);
}

/// Central signed in with `gateways` as the account's list, as a restore
/// leaves it, for tests of what follows a sign-in.
#[cfg(test)]
pub(super) fn restored_for_test(state: &mut LobbyState, gateways: Vec<String>) {
    if state.central.is_none() {
        enable(state);
    }
    let (send, _receive) = mpsc::sync_channel::<Job>(4);
    let restored = Restored {
        handle: Handle {
            send,
            active: Arc::new(AtomicBool::new(true)),
        },
        gateways,
        preferences: Preferences::default(),
        terms_pending: false,
        profile_handle: None,
    };
    receive(
        state,
        &mut crate::prefs::Prefs::default(),
        &Mailbox::default(),
        Ok(Box::new(restored)),
    );
}

/// Dev-control helper to inspect the post-sign-in Central Lobby & Gateway Hub
/// without requiring live Central credentials.
#[cfg(feature = "dev-control")]
pub(crate) fn demo_central_lobby(state: &mut LobbyState) {
    use baylee_client_core::lobby::gateway_info::{GatewayInfo, Probe};
    if state.central.is_none() {
        enable(state);
    }
    let urls = vec![
        "http://127.0.0.1:28766".to_string(),
        "https://sanctuary.baylee.app".to_string(),
        "https://legacy.community.example".to_string(),
    ];
    state.probes.insert(
        "http://127.0.0.1:28766".into(),
        Probe::Known(GatewayInfo {
            name: Some("Local Dev Gateway".into()),
            version: "0.1.0-beta.9c".into(),
            protocol_version: baylee_protocol::PROTOCOL_VERSION,
            view_version: baylee_view::VIEW_VERSION,
            games_running: Some(0),
            rooms_waiting: Some(1),
            players_online: Some(3),
            ..GatewayInfo::default()
        }),
    );
    state.probes.insert(
        "https://sanctuary.baylee.app".into(),
        Probe::Known(GatewayInfo {
            name: Some("Baylee Sanctuary".into()),
            version: "0.1.0-beta.9c".into(),
            protocol_version: baylee_protocol::PROTOCOL_VERSION,
            view_version: baylee_view::VIEW_VERSION,
            games_running: Some(5),
            rooms_waiting: Some(3),
            players_online: Some(14),
            ..GatewayInfo::default()
        }),
    );
    state.probes.insert(
        "https://legacy.community.example".into(),
        Probe::Known(GatewayInfo {
            name: Some("Old Guard Arena".into()),
            version: "0.1.0-beta.3b".into(),
            protocol_version: baylee_protocol::PROTOCOL_VERSION.saturating_sub(1),
            view_version: baylee_view::VIEW_VERSION.saturating_sub(1),
            games_running: Some(2),
            rooms_waiting: Some(1),
            players_online: Some(4),
            ..GatewayInfo::default()
        }),
    );
    let (send, _receive) = mpsc::sync_channel::<Job>(4);
    let restored = Restored {
        handle: Handle {
            send,
            active: Arc::new(AtomicBool::new(true)),
        },
        gateways: urls,
        preferences: Preferences::default(),
        terms_pending: false,
        profile_handle: Some("AceVik#0007".to_string()),
    };
    receive(
        state,
        &mut crate::prefs::Prefs::default(),
        &Mailbox::default(),
        Ok(Box::new(restored)),
    );
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
        let device = state.gateways.clone();
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
            terms_pending: false,
            profile_handle: None,
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
        assert!(
            state.central.is_none(),
            "Central is off once its session ends"
        );
        assert!(!state.lobby.gateway_chosen());
        assert!(
            state.gateway_epoch > previous,
            "late gateway replies must be invalidated"
        );
        assert_eq!(
            state.gateways, device,
            "the device's own gateways, not the account's"
        );
    }

    #[test]
    fn a_central_sign_in_a_gateway_asked_for_enters_that_gateway() {
        let mut state = LobbyState::from_settings(crate::settings::ClientSettings::default());
        // An address no request can be built from, as the lobby's headless
        // tests use: entering the gateway asks its `/auth/config`.
        state.gateway.clear();
        state.gateway_selected = true;
        bound(&mut state);
        assert!(state.central_bound());
        let (send, jobs) = mpsc::sync_channel::<Job>(4);
        let mut prefs = crate::prefs::Prefs::default();
        let other = "http://127.0.0.1:28766".to_string();
        state.probes.insert(other.clone(), Probe::Older);
        let restored = Restored {
            handle: Handle {
                send,
                active: Arc::new(AtomicBool::new(true)),
            },
            gateways: vec![other.clone()],
            preferences: Preferences::default(),
            terms_pending: false,
            profile_handle: None,
        };
        receive(
            &mut state,
            &mut prefs,
            &Mailbox::default(),
            Ok(Box::new(restored)),
        );
        assert!(state.central_ready());
        assert!(state.lobby.gateway_chosen(), "the gateway is entered");
        assert_eq!(state.gateway, "");
        assert_eq!(state.gateways, vec![other, String::new()]);
        assert!(jobs.try_recv().is_ok(), "and admitted to");
    }

    /// Central's own terms wait for the account (the owner's sign-in,
    /// 22.10.2026): the gateway is chosen but not entered, the account's
    /// list is not written while Central refuses writes, and accepting the
    /// terms enters the gateway.
    #[test]
    fn central_terms_that_wait_come_before_the_gateway() {
        use bevy::ecs::system::RunSystemOnce as _;
        let mut state = LobbyState::from_settings(crate::settings::ClientSettings::default());
        state.gateway.clear();
        state.gateway_selected = true;
        bound(&mut state);
        let (send, jobs) = mpsc::sync_channel::<Job>(4);
        let restored = Restored {
            handle: Handle {
                send,
                active: Arc::new(AtomicBool::new(true)),
            },
            gateways: Vec::new(),
            preferences: Preferences::default(),
            terms_pending: true,
            profile_handle: None,
        };
        let mailbox = Mailbox::default();
        let mut prefs = crate::prefs::Prefs::default();
        receive(&mut state, &mut prefs, &mailbox, Ok(Box::new(restored)));
        assert!(state.lobby.gateway_chosen(), "the gateway is chosen");
        assert!(terms_pending(&state));
        assert!(jobs.try_recv().is_err(), "but not entered");
        assert_ne!(
            state.central.as_ref().unwrap().saved_gateways,
            state.gateways,
            "the gateway joined the account's list"
        );
        let mut world = bevy::prelude::World::new();
        world.insert_resource(state);
        world.insert_resource(prefs);
        world.run_system_once(sync_gateways).unwrap();
        assert!(jobs.try_recv().is_err(), "which is not written yet");
        let mut state = world.resource_mut::<LobbyState>();
        terms_accepted(&mut state);
        assert!(!terms_pending(&state));
        readmit(&mut state, &mailbox);
        assert!(jobs.try_recv().is_ok(), "accepted, the gateway is entered");
    }

    /// An account's list from before the live gateway's move (or sealed by
    /// the website, which names its first address) shows the live gateway
    /// by today's address, and is not written back in the new spelling: an
    /// installed client on another device reads the list it always read.
    #[test]
    #[ignore = "the client still pins the old address; runs from the flip on (#402)"]
    fn the_account_s_list_shows_the_live_gateway_by_today_s_address() {
        use bevy::ecs::system::RunSystemOnce as _;
        let mut state = LobbyState::from_settings(crate::settings::ClientSettings::default());
        enable(&mut state);
        let other = "https://other.invalid".to_string();
        // Probed already: the test asks no gateway anything, the live one least.
        for url in [gateway_list::PINNED.to_string(), other.clone()] {
            state.probes.insert(url, Probe::Asking);
        }
        let (send, jobs) = mpsc::sync_channel::<Job>(4);
        let restored = Restored {
            handle: Handle {
                send,
                active: Arc::new(AtomicBool::new(true)),
            },
            gateways: vec![gateway_list::KEYED.into(), other.clone()],
            preferences: Preferences::default(),
            terms_pending: false,
            profile_handle: None,
        };
        let mailbox = Mailbox::default();
        let mut prefs = crate::prefs::Prefs::default();
        receive(&mut state, &mut prefs, &mailbox, Ok(Box::new(restored)));
        assert_eq!(
            state.gateways,
            vec![gateway_list::PINNED.to_string(), other]
        );
        assert!(mailbox.0.lock().unwrap().is_empty(), "nothing was probed");
        let mut world = bevy::prelude::World::new();
        world.insert_resource(state);
        world.insert_resource(prefs);
        world.run_system_once(sync_gateways).unwrap();
        assert!(jobs.try_recv().is_err(), "the list is not rewritten");
    }

    #[test]
    fn central_turned_on_without_a_gateway_has_no_way_back() {
        let mut state = LobbyState::from_settings(crate::settings::ClientSettings::default());
        enable(&mut state);
        assert!(!state.central_bound());
        leave(&mut state);
        assert!(state.central_pending());
    }
}
