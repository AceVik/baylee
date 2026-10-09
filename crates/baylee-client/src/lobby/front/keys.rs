//! The front door's keyboard (`KEYBOARD.md` §1.3, §7.2): the `TabOrder`
//! tables its faces are built from, and the bridge between the kit's focus
//! walker and the lobby's own text fields.
//!
//! The faces are the first lobby screen to wear the kit's [`Stop`]s. Their
//! text fields stay the lobby's (a [`TextBuffer`] with a caret, masking,
//! paste and the browser's `<input>` on wasm); each field's box carries a
//! [`Stop`] and a [`LobbyField`], and two systems keep the two notions of
//! focus one:
//!
//! - [`kit_to_lobby`]: focus walked onto a field puts the lobby's caret
//!   there (the text selected, as a browser does); focus on a control —
//!   a button, the eye — parks the caret, so a typed letter goes nowhere.
//! - [`lobby_to_kit`]: the lobby placing its caret (a click on a field, a
//!   refusal sending it to the missing one, a face opening) moves the
//!   kit's focus onto that field, ring hidden.
//!
//! Enter or Space on a focused control is [`activate_by_key`]: the
//! control's own `Press`, through the same door a click takes. Enter in a
//! field stays the lobby's (it submits the face).
//!
//! [`TextBuffer`]: baylee_client_core::textbuf::TextBuffer

use crate::shellkit::focus::{Activated, Remembered, Stop, TabOrder};
use baylee_client_core::lobby::{Face, Field, Screen};
use bevy::input_focus::{FocusCause, InputFocus};
use bevy::prelude::*;

use super::super::press::Cx;
use super::super::{LobbyState, Mailbox, Press, Scrolled};

// The text row and the colophon end every face (§3, principle 8):
// `lang` (a radio group), `music`, `settings`, `offline`, `about`, then the
// colophon's `source` link and its `notice` (the full notice, on the
// one-line colophon).

const GATEWAY_STOPS: &[&str] = &[
    "address", "save", "lang", "music", "settings", "offline", "about", "source", "notice",
];
const SIGN_IN_STOPS: &[&str] = &[
    "back", "retry", "username", "password", "eye", "submit", "create", "guest", "lang", "music",
    "settings", "offline", "about", "source", "notice",
];
const CREATE_STOPS: &[&str] = &[
    "back",
    "username",
    "display",
    "password",
    "eye",
    "again",
    "eye-again",
    "key",
    "submit",
    "lang",
    "music",
    "settings",
    "offline",
    "about",
    "source",
    "notice",
];
const GUEST_STOPS: &[&str] = &[
    "back",
    "guest-name",
    "key",
    "submit",
    "lang",
    "music",
    "settings",
    "offline",
    "about",
    "source",
    "notice",
];

/// The gateway picker: the address (its list is the saved gateways, walked
/// with the arrows), Check / Save, the text row, the colophon.
pub(crate) const GATEWAY: TabOrder = TabOrder {
    name: "front-gateway",
    stops: GATEWAY_STOPS,
    modal: false,
};
/// The sign-in face.
pub(crate) const SIGN_IN: TabOrder = TabOrder {
    name: "front-sign-in",
    stops: SIGN_IN_STOPS,
    modal: false,
};
/// The create-account face.
pub(crate) const CREATE: TabOrder = TabOrder {
    name: "front-create",
    stops: CREATE_STOPS,
    modal: false,
};
/// The guest's face.
pub(crate) const GUEST: TabOrder = TabOrder {
    name: "front-guest",
    stops: GUEST_STOPS,
    modal: false,
};
/// The terms sheet: the text (a focusable scroll region), then its
/// answers; a guest's question replaces Not now and Accept with Stay and
/// Sign out.
pub(crate) const TERMS: TabOrder = TabOrder {
    name: "terms",
    stops: &[
        "text", "retry", "not-now", "decline", "accept", "stay", "leave",
    ],
    modal: true,
};
/// The About sheet.
pub(crate) const ABOUT: TabOrder = TabOrder {
    name: "about",
    stops: &["text", "source", "close"],
    modal: true,
};

/// The table of the face on show.
#[must_use]
pub(crate) fn table_of(state: &LobbyState) -> &'static TabOrder {
    if !state.lobby.gateway_chosen() {
        return &GATEWAY;
    }
    match state.lobby.face() {
        Face::SignIn => &SIGN_IN,
        Face::Create => &CREATE,
        Face::Guest => &GUEST,
    }
}

/// A stop of the face on show.
#[must_use]
pub(crate) fn stop(state: &LobbyState, id: &'static str) -> Stop {
    Stop::new(table_of(state).name, id)
}

/// The stop a lobby field's box carries on the face on show.
#[must_use]
pub(crate) fn field_stop(state: &LobbyState, field: Field) -> Option<Stop> {
    let id = match field {
        Field::Gateway => "address",
        Field::Username => "username",
        Field::Password => "password",
        Field::DisplayName => "display",
        Field::PasswordAgain => "again",
        Field::InviteKey => "key",
        Field::GuestName => "guest-name",
        _ => return None,
    };
    Some(stop(state, id))
}

/// A lobby text field's box: the field it types into.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct LobbyField(pub(crate) Field);

/// Whether the lobby's front door is the screen on show (and nothing
/// stands over it that owns the keys).
fn at_the_door(state: &LobbyState) -> bool {
    matches!(state.lobby.screen(), Screen::SignIn { .. })
        && !state.settings.is_open()
        && state.confirmation.is_none()
}

/// Where the kit's focus and the lobby's caret are one: the front door, and
/// the settings screen (its search is a lobby field, owner 09.10.2026),
/// unless a confirmation stands over it.
fn bridged(state: &LobbyState) -> bool {
    at_the_door(state)
        || (state.settings.is_open()
            && state.confirmation.is_none()
            && state.lobby.deleting_account().is_none())
}

/// The stop the two notions of focus last agreed on. [`kit_to_lobby`]
/// acts only when the kit's focus stands on another stop: not when the
/// lobby moved it there itself ([`lobby_to_kit`]), and not when a rebuild
/// drew the same stop anew and the walker followed it (the lobby may have
/// moved its caret again since, and would be overruled).
#[derive(Resource, Default, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct MovedByTheLobby(Option<Stop>);

/// Focus walked onto a field takes the lobby's caret there; focus on any
/// other front-door stop parks it.
pub(crate) fn kit_to_lobby(
    focus: Res<InputFocus>,
    mut moved: ResMut<MovedByTheLobby>,
    fields: Query<&LobbyField>,
    stops: Query<&Stop>,
    mut state: ResMut<LobbyState>,
) {
    if !focus.is_changed() || !bridged(&state) {
        return;
    }
    let Some(entity) = focus.get() else {
        return;
    };
    let Ok(stop) = stops.get(entity) else {
        return;
    };
    if moved.0 == Some(*stop) {
        return;
    }
    moved.0 = Some(*stop);
    if let Ok(LobbyField(field)) = fields.get(entity) {
        if state.lobby.focus() != *field || state.lobby.caret_parked() {
            state.lobby.focus_on(*field);
            state.lobby.select_all();
        }
    } else if (stop.table.starts_with("front-")
        || stop.table == crate::settingsui::keys::SETTINGS.name)
        && !state.lobby.caret_parked()
    {
        state.lobby.park_caret();
    }
}

/// The lobby placing its caret moves the kit's focus onto that field.
pub(crate) fn lobby_to_kit(
    state: Res<LobbyState>,
    fields: Query<(Entity, &LobbyField, &Stop)>,
    mut focus: ResMut<InputFocus>,
    mut remembered: ResMut<Remembered>,
    mut moved: ResMut<MovedByTheLobby>,
) {
    // A parked caret is the walker's business (focus stands on a control).
    if !bridged(&state) || state.terms.up() || state.lobby.caret_parked() {
        return;
    }
    let want = state.lobby.focus();
    let Some((entity, _, stop)) = fields.iter().find(|(_, f, _)| f.0 == want) else {
        return;
    };
    if focus.get() != Some(entity) {
        focus.set(entity, FocusCause::Navigated);
    }
    if moved.0 != Some(*stop) {
        moved.0 = Some(*stop);
    }
    if remembered.0 != Some(*stop) {
        remembered.0 = Some(*stop);
    }
}

/// A parked caret the lobby has since placed again: the placement wins.
pub(crate) fn unpark_on_placement(mut state: ResMut<LobbyState>, mut seen: Local<Option<u64>>) {
    let epoch = state.lobby.focus_epoch();
    if *seen != Some(epoch) {
        *seen = Some(epoch);
        if state.lobby.caret_parked() {
            let field = state.lobby.focus();
            state.lobby.focus_on(field);
        }
    }
}

/// The tables whose Enter and Space are [`activate_by_key`]'s: the front
/// door's faces and sheets, and the settings screen with its profile sheet.
/// Play, Decks and the room are `focusing::keys_press`', the builder its own.
pub(in crate::lobby) fn ours(table: &str) -> bool {
    [
        GATEWAY.name,
        SIGN_IN.name,
        CREATE.name,
        GUEST.name,
        TERMS.name,
        ABOUT.name,
        crate::settingsui::keys::SETTINGS.name,
        crate::settingsui::keys::PROFILE_SHEET.name,
    ]
    .contains(&table)
}

/// Enter or Space on a focused front-door or settings control (not a
/// field): the control's own press.
#[allow(clippy::too_many_arguments)] // a Bevy system: the press's resources ride along
pub(in crate::lobby) fn activate_by_key(
    mut activated: MessageReader<Activated>,
    presses: Query<&Press>,
    fields: Query<(), With<LobbyField>>,
    mut state: ResMut<LobbyState>,
    mut prefs: ResMut<crate::prefs::Prefs>,
    mut scrolled: ResMut<Scrolled>,
    mailbox: Res<Mailbox>,
    mut settings: Option<ResMut<crate::settings::ClientSettings>>,
) {
    for hit in activated.read() {
        if !hit.by_key || fields.contains(hit.entity) || !ours(hit.stop.table) {
            continue;
        }
        let Ok(press) = presses.get(hit.entity) else {
            continue;
        };
        let cx = Cx {
            state: &mut state,
            prefs: &mut prefs,
            scrolled: &mut scrolled,
            mailbox: &mailbox,
            settings: &mut settings,
        };
        super::super::clicks::run(*press, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// No table names a stop twice, and every front-door face ends in the
    /// text row and the colophon (principle 8: the notices travel with the
    /// door).
    #[test]
    fn every_face_ends_in_the_text_row_and_the_colophon() {
        for table in [&GATEWAY, &SIGN_IN, &CREATE, &GUEST, &TERMS, &ABOUT] {
            let named: Vec<&str> = table.stops.to_vec();
            let mut unique = named.clone();
            unique.sort_unstable();
            unique.dedup();
            assert_eq!(unique.len(), named.len(), "{}: a stop twice", table.name);
            if table.name.starts_with("front-") {
                for id in ["lang", "music", "settings", "offline", "about", "source"] {
                    assert!(named.contains(&id), "{} lacks {id}", table.name);
                }
            }
        }
    }
}
