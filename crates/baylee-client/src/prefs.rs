//! The account's preferences, and the two places they are kept.
//!
//! ```text
//!   gateway  GET /settings ─┐                     ┌─ PUT /settings
//!                           v                     │
//!                         Prefs  ── every edit ───┴──> local file / localStorage
//! ```
//!
//! Keys and standing orders follow the **account**, so a player who rebinds
//! confirm at home finds it rebound at a friend's table. That is the whole
//! reason the gateway stores them at all — and it stores them as an opaque
//! blob, because it links neither the client's brain nor the engine.
//!
//! The local copy is not a cache of that; it is what an *unsigned-in* client
//! uses. "Play the house AI offline" needs a keymap too, and a lobby that has
//! not been signed into yet has no account to ask. Signing in replaces the
//! local copy with the account's, which is the only ordering that does not
//! quietly upload one machine's defaults over a player's real bindings.
//!
//! Writes are debounced: a settings screen where every keystroke costs a
//! request would send a dozen while a player drags one slider.

use baylee_client_core::automation::PhaseOrders;
use baylee_client_core::prefs::{AutoRules, Keymap, Preferences};
use bevy::prelude::*;
use std::sync::{Arc, Mutex};

/// The document the local copy is stored under.
const LOCAL: &str = "preferences.json";

/// How long an edit sits before it is written back, in seconds.
///
/// Long enough that dragging a slider or holding a key sends one request,
/// short enough that a player who closes the settings screen and quits has
/// already had their change saved.
const DEBOUNCE: f32 = 1.5;

/// Where a `GET /settings` leaves its answer.
///
/// A slot rather than a channel for the same reason as the card text: a
/// `Receiver` is not `Sync`, a Bevy resource must be, and there is only ever
/// one answer in flight.
type Slot = Arc<Mutex<Option<Preferences>>>;

/// The account's preferences, as this client holds them.
#[derive(Resource, Default)]
pub struct Prefs {
    /// The preferences themselves.
    value: Preferences,
    /// Gateway base URL and account bearer token, once signed in.
    account: Option<(String, String)>,
    /// A `GET /settings` in flight.
    loading: Option<Slot>,
    /// Whether an edit is waiting to be written back.
    dirty: bool,
    /// Seconds left before the pending edit is written back.
    save_in: f32,
}

impl Prefs {
    /// The preferences a client starts with: whatever this machine last saw.
    #[must_use]
    pub fn local() -> Self {
        let prefs = crate::settings::store::read_named(LOCAL)
            .map(|text| Preferences::from_json(&text))
            .unwrap_or_default();
        Self {
            value: prefs,
            ..Self::default()
        }
    }

    /// The keymap, for resolving this frame's key presses.
    #[must_use]
    pub const fn keymap(&self) -> &Keymap {
        &self.value.keymap
    }

    /// What the client may answer without asking.
    #[must_use]
    pub const fn auto(&self) -> &AutoRules {
        &self.value.auto
    }

    /// The phase rail.
    #[must_use]
    pub const fn orders(&self) -> &PhaseOrders {
        &self.value.orders
    }

    /// The phase rail's *keyboard selection*, which is not a preference.
    ///
    /// Moving the highlight around the rail is navigation, not a setting, and
    /// routing it through [`Prefs::edit`] would post the whole blob to the
    /// gateway on every press of `⇧S`.
    pub const fn rail_cursor(&mut self) -> &mut PhaseOrders {
        &mut self.value.orders
    }

    /// Everything, read-only.
    #[must_use]
    pub const fn all(&self) -> &Preferences {
        &self.value
    }

    /// Changes something, and schedules the write-back.
    pub const fn edit(&mut self) -> Edit<'_> {
        Edit { prefs: self }
    }

    /// Signs in: from here on the account's copy is the truth.
    ///
    /// Called by whatever owns the sign-in — the lobby, or an embedding
    /// application that already has an account. Idempotent, so a lobby that
    /// calls it every frame while signed in costs one request in total.
    pub fn attach(&mut self, gateway: &str, token: &str) {
        if self
            .account
            .as_ref()
            .is_some_and(|(g, t)| g == gateway && t == token)
        {
            return;
        }
        self.account = Some((gateway.to_string(), token.to_string()));
        // Anything this machine had queued belongs to nobody now: the account
        // is about to say what the preferences are.
        self.dirty = false;
        self.save_in = 0.0;
        let slot: Slot = Arc::default();
        let target = Arc::clone(&slot);
        let mut request = ehttp::Request::get(format!("{gateway}/settings"));
        request
            .headers
            .insert("authorization", format!("Bearer {token}"));
        ehttp::fetch(request, move |result| {
            let prefs = match result {
                Ok(response) if response.ok => response.text().map(Preferences::from_json),
                Ok(response) => {
                    bevy::log::warn!(status = response.status, "settings request refused");
                    None
                }
                Err(err) => {
                    // Not worth a visible error: the client has a keymap, and
                    // the one it has works.
                    bevy::log::info!("settings unavailable: {err}");
                    None
                }
            };
            if let (Some(prefs), Ok(mut slot)) = (prefs, target.lock()) {
                *slot = Some(prefs);
            }
        });
        self.loading = Some(slot);
    }

    /// Signs out: the account's copy stays on the gateway, and this client
    /// keeps what it has, writing it only to this machine from here on.
    ///
    /// Idempotent, like [`Prefs::attach`], so a caller can follow the token
    /// every frame; a detach that changed nothing must not discard an edit
    /// still waiting on its debounce.
    pub fn detach(&mut self) {
        if self.account.is_none() {
            return;
        }
        self.account = None;
        self.loading = None;
    }

    /// Writes the local copy. Best effort — a read-only home directory is not
    /// worth interrupting a game for.
    fn store_locally(&self) {
        crate::settings::store::write_named(LOCAL, &self.value.to_json());
    }

    /// Sends the account's copy to the gateway.
    fn store_remotely(&self) {
        let Some((gateway, token)) = self.account.as_ref() else {
            return;
        };
        let body = self.value.to_json().into_bytes();
        let mut request = ehttp::Request::post(format!("{gateway}/settings"), body);
        request.method = "PUT".to_string();
        request.headers = ehttp::Headers::new(&[
            ("Accept", "application/json"),
            ("Content-Type", "application/json"),
            ("Authorization", &format!("Bearer {token}")),
        ]);
        ehttp::fetch(request, |result| match result {
            Ok(response) if response.ok => {}
            Ok(response) => bevy::log::warn!(status = response.status, "settings not saved"),
            Err(err) => bevy::log::info!("settings not saved: {err}"),
        });
    }
}

/// A borrow that marks the preferences dirty when it is dropped.
///
/// The point is that there is no way to change a preference without
/// scheduling the save — a `pub` field would let one caller forget, and the
/// symptom would be a setting that survives until the next restart and then
/// silently reverts.
pub struct Edit<'a> {
    /// The resource being edited.
    prefs: &'a mut Prefs,
}

impl std::ops::Deref for Edit<'_> {
    type Target = Preferences;

    fn deref(&self) -> &Self::Target {
        &self.prefs.value
    }
}

impl std::ops::DerefMut for Edit<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.prefs.value
    }
}

impl Drop for Edit<'_> {
    fn drop(&mut self) {
        self.prefs.dirty = true;
        self.prefs.save_in = DEBOUNCE;
    }
}

/// Installs [`Prefs`] and keeps it in sync.
///
/// Added by both the duel and the lobby, because both read the keymap and
/// only one of them is present in any given launch. `is_plugin_added` rather
/// than an unconditional `add_plugins`: Bevy panics on a duplicate plugin,
/// and "whoever gets there first" is exactly the semantics wanted here.
pub struct PrefsPlugin;

impl Plugin for PrefsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Prefs::local())
            .add_systems(Update, sync);
    }
}

/// Adds [`PrefsPlugin`] unless another plugin already did.
pub(crate) fn install(app: &mut App) {
    if !app.is_plugin_added::<PrefsPlugin>() {
        app.add_plugins(PrefsPlugin);
    }
}

/// Takes delivery of the account's copy, and writes edits back once they
/// have stopped arriving.
pub fn sync(mut prefs: ResMut<Prefs>, time: Res<Time>) {
    if let Some(slot) = prefs.loading.clone()
        && let Some(arrived) = slot.lock().ok().and_then(|mut s| s.take())
    {
        prefs.loading = None;
        prefs.value = arrived;
        // The account's copy is now this machine's too, so an offline launch
        // starts where the player left off rather than at the defaults.
        prefs.store_locally();
        bevy::log::info!("preferences loaded from the gateway");
    }
    if !prefs.dirty {
        return;
    }
    prefs.save_in -= time.delta_secs();
    if prefs.save_in > 0.0 {
        return;
    }
    prefs.dirty = false;
    prefs.store_locally();
    prefs.store_remotely();
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_client_core::automation::{RailRow, RailSide};
    use baylee_client_core::prefs::{Action, Chord};

    #[test]
    fn an_edit_schedules_its_own_write_back() {
        let mut prefs = Prefs::default();
        assert!(!prefs.dirty, "nothing has changed yet");
        prefs
            .edit()
            .keymap
            .bind(Action::Confirm, vec![Chord::key("KeyZ")]);
        assert!(prefs.dirty, "an edit did not schedule a save");
        assert!(prefs.save_in > 0.0, "the save was not debounced");
        assert_eq!(
            prefs.keymap().chords(Action::Confirm),
            &[Chord::key("KeyZ")]
        );
    }

    /// Moving the rail highlight is navigation, not a preference. Routing it
    /// through `edit` would post the whole blob on every `⇧S`.
    #[test]
    fn walking_the_rail_does_not_schedule_a_request() {
        let mut prefs = Prefs::default();
        prefs.rail_cursor().move_selection(1);
        assert!(prefs.orders().selected().is_some());
        assert!(!prefs.dirty, "moving the highlight queued a save");
        // …but toggling a button is.
        prefs.edit().orders.toggle(RailSide::Mine, RailRow::Upkeep);
        assert!(prefs.dirty);
    }

    /// Signing in must not upload this machine's defaults over the account's
    /// real bindings — the account's copy is the one that wins.
    #[test]
    fn signing_in_drops_whatever_was_queued_locally() {
        let mut prefs = Prefs::default();
        prefs.edit().auto.skip_empty_blocks = true;
        assert!(prefs.dirty);
        prefs.attach("http://gateway.invalid", "tok");
        assert!(
            !prefs.dirty,
            "a queued local edit would have overwritten the account"
        );
        assert!(
            prefs.loading.is_some(),
            "signing in did not ask for the account's copy"
        );
    }

    #[test]
    fn attaching_the_same_account_twice_asks_once() {
        let mut prefs = Prefs::default();
        prefs.attach("http://gateway.invalid", "tok");
        let first = prefs.loading.clone();
        prefs.attach("http://gateway.invalid", "tok");
        assert!(
            Arc::ptr_eq(
                first.as_ref().expect("a request went out"),
                prefs.loading.as_ref().expect("still the same request")
            ),
            "a second attach with the same credentials started a second request"
        );
        // A different account is a different question, and does ask again.
        prefs.attach("http://gateway.invalid", "other");
        assert!(!Arc::ptr_eq(
            first.as_ref().expect("a request went out"),
            prefs.loading.as_ref().expect("a fresh request")
        ));
    }

    #[test]
    fn signing_out_keeps_the_preferences_but_stops_writing_to_the_account() {
        let mut prefs = Prefs::default();
        prefs.attach("http://gateway.invalid", "tok");
        prefs.edit().auto.skip_opponent_turns = true;
        prefs.detach();
        assert!(
            prefs.auto().skip_opponent_turns,
            "the player lost their settings"
        );
        assert!(prefs.account.is_none());
        prefs.store_remotely(); // a no-op, and must not panic
    }
}
