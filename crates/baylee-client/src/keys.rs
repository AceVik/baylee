//! The table between a [`Chord`] and whatever this window system calls a key.
//!
//! `baylee-client-core` names a key with a string — `"KeyW"`, `"Space"`,
//! `"ArrowUp"` — because a stored keymap has to outlive both the renderer and
//! the machine it was bound on. This module is the only place that knows those
//! names are Bevy's [`KeyCode`]s, which keeps the keymap itself renderer-free
//! and testable.
//!
//! Names are *physical*, not typed characters: a player who binds the key
//! right of `A` finds it right of `A` on a German keyboard too, rather than
//! wherever that layout happens to put an `S`.

use baylee_client_core::prefs::{Action, Chord, Keymap};
use bevy::prelude::*;

/// Every key a chord may name, with the stable name it is stored under.
///
/// Not exhaustive over `KeyCode` on purpose: a keymap is only useful over keys
/// a player can reliably press and a settings screen can print. Modifiers are
/// absent because they are *part of* a chord, never its subject.
const TABLE: &[(&str, KeyCode)] = &[
    ("KeyA", KeyCode::KeyA),
    ("KeyB", KeyCode::KeyB),
    ("KeyC", KeyCode::KeyC),
    ("KeyD", KeyCode::KeyD),
    ("KeyE", KeyCode::KeyE),
    ("KeyF", KeyCode::KeyF),
    ("KeyG", KeyCode::KeyG),
    ("KeyH", KeyCode::KeyH),
    ("KeyI", KeyCode::KeyI),
    ("KeyJ", KeyCode::KeyJ),
    ("KeyK", KeyCode::KeyK),
    ("KeyL", KeyCode::KeyL),
    ("KeyM", KeyCode::KeyM),
    ("KeyN", KeyCode::KeyN),
    ("KeyO", KeyCode::KeyO),
    ("KeyP", KeyCode::KeyP),
    ("KeyQ", KeyCode::KeyQ),
    ("KeyR", KeyCode::KeyR),
    ("KeyS", KeyCode::KeyS),
    ("KeyT", KeyCode::KeyT),
    ("KeyU", KeyCode::KeyU),
    ("KeyV", KeyCode::KeyV),
    ("KeyW", KeyCode::KeyW),
    ("KeyX", KeyCode::KeyX),
    ("KeyY", KeyCode::KeyY),
    ("KeyZ", KeyCode::KeyZ),
    ("Digit0", KeyCode::Digit0),
    ("Digit1", KeyCode::Digit1),
    ("Digit2", KeyCode::Digit2),
    ("Digit3", KeyCode::Digit3),
    ("Digit4", KeyCode::Digit4),
    ("Digit5", KeyCode::Digit5),
    ("Digit6", KeyCode::Digit6),
    ("Digit7", KeyCode::Digit7),
    ("Digit8", KeyCode::Digit8),
    ("Digit9", KeyCode::Digit9),
    ("Space", KeyCode::Space),
    ("Enter", KeyCode::Enter),
    ("Escape", KeyCode::Escape),
    ("Tab", KeyCode::Tab),
    ("Backspace", KeyCode::Backspace),
    ("Delete", KeyCode::Delete),
    ("Home", KeyCode::Home),
    ("End", KeyCode::End),
    ("PageUp", KeyCode::PageUp),
    ("PageDown", KeyCode::PageDown),
    ("ArrowUp", KeyCode::ArrowUp),
    ("ArrowDown", KeyCode::ArrowDown),
    ("ArrowLeft", KeyCode::ArrowLeft),
    ("ArrowRight", KeyCode::ArrowRight),
    ("Minus", KeyCode::Minus),
    ("Equal", KeyCode::Equal),
    ("BracketLeft", KeyCode::BracketLeft),
    ("BracketRight", KeyCode::BracketRight),
    ("Semicolon", KeyCode::Semicolon),
    ("Quote", KeyCode::Quote),
    ("Comma", KeyCode::Comma),
    ("Period", KeyCode::Period),
    ("Slash", KeyCode::Slash),
    ("Backslash", KeyCode::Backslash),
    ("Backquote", KeyCode::Backquote),
    ("F1", KeyCode::F1),
    ("F2", KeyCode::F2),
    ("F3", KeyCode::F3),
    ("F4", KeyCode::F4),
    ("F5", KeyCode::F5),
    ("F6", KeyCode::F6),
    ("F7", KeyCode::F7),
    ("F8", KeyCode::F8),
    ("F9", KeyCode::F9),
    ("F10", KeyCode::F10),
    ("F11", KeyCode::F11),
    ("F12", KeyCode::F12),
];

/// The key a stored name refers to, or `None` for a name this build has never
/// heard of — a keymap from a newer client must not stop the older one from
/// starting.
#[must_use]
pub fn key_code(name: &str) -> Option<KeyCode> {
    TABLE.iter().find(|(n, _)| *n == name).map(|(_, k)| *k)
}

/// The stable name a key is stored under.
#[must_use]
pub fn key_name(code: KeyCode) -> Option<&'static str> {
    TABLE.iter().find(|(_, k)| *k == code).map(|(n, _)| *n)
}

/// Which modifiers are held right now.
fn modifiers(keys: &ButtonInput<KeyCode>) -> (bool, bool, bool, bool) {
    (
        keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]),
        keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]),
        keys.any_pressed([KeyCode::AltLeft, KeyCode::AltRight]),
        keys.any_pressed([KeyCode::SuperLeft, KeyCode::SuperRight]),
    )
}

/// Whether the modifiers held are *exactly* the ones a chord asks for.
///
/// Exactly, not "at least": `W` and `⇧W` are two different bindings, and a
/// player holding shift to move the rail selection must not also walk the card
/// cursor. Every "if not shift" guard the old input code was littered with is
/// this one comparison instead.
fn modifiers_match(chord: &Chord, held: (bool, bool, bool, bool)) -> bool {
    (chord.shift, chord.ctrl, chord.alt, chord.meta) == held
}

/// The keymap, resolved against this frame's key state.
///
/// Constructed per system rather than stored, so a rebinding takes effect on
/// the next frame with nothing to invalidate.
pub struct Binds<'a> {
    /// This frame's key state.
    keys: &'a ButtonInput<KeyCode>,
    /// The player's bindings.
    map: &'a Keymap,
    /// The modifiers held, computed once.
    held: (bool, bool, bool, bool),
}

impl<'a> Binds<'a> {
    /// Resolves a keymap against the current key state.
    #[must_use]
    pub fn new(keys: &'a ButtonInput<KeyCode>, map: &'a Keymap) -> Self {
        Self {
            keys,
            map,
            held: modifiers(keys),
        }
    }

    /// Whether an action's key went down this frame.
    #[must_use]
    pub fn just_pressed(&self, action: Action) -> bool {
        self.matching(action, ButtonInput::just_pressed)
    }

    /// Whether an action's key is held.
    #[must_use]
    pub fn pressed(&self, action: Action) -> bool {
        self.matching(action, ButtonInput::pressed)
    }

    /// Shared body of the two above.
    fn matching(
        &self,
        action: Action,
        test: impl Fn(&ButtonInput<KeyCode>, KeyCode) -> bool,
    ) -> bool {
        self.map.chords(action).iter().any(|chord| {
            modifiers_match(chord, self.held)
                && key_code(&chord.key).is_some_and(|code| test(self.keys, code))
        })
    }
}

/// The actions whose chords went down this frame.
///
/// Resolved once and then carried as a small array of flags, so a system can
/// hold `&mut Prefs` while still knowing what was pressed — and so the keymap
/// is walked once a frame rather than once per question asked of it.
#[derive(Clone, Copy)]
pub struct Fired([bool; Action::ALL.len()]);

impl Fired {
    /// Resolves every action against this frame's key state.
    #[must_use]
    pub fn of(keys: &ButtonInput<KeyCode>, map: &Keymap) -> Self {
        let binds = Binds::new(keys, map);
        let mut flags = [false; Action::ALL.len()];
        for (slot, action) in Action::ALL.into_iter().enumerate() {
            flags[slot] = binds.just_pressed(action);
        }
        Self(flags)
    }

    /// The same flags, built from actions rather than from keys.
    ///
    /// Not everything that fires an action has a chord behind it: a button in
    /// the prompt bar, a touch target on a phone, and a test all name the
    /// *action*. This is how they enter the pipeline the keyboard already
    /// uses, instead of every handler growing a second way in.
    #[must_use]
    pub fn of_actions(actions: &[Action]) -> Self {
        let mut flags = [false; Action::ALL.len()];
        for action in actions {
            if let Some(slot) = Action::ALL.iter().position(|a| a == action) {
                flags[slot] = true;
            }
        }
        Self(flags)
    }

    /// Whether an action fired.
    #[must_use]
    pub fn has(&self, action: Action) -> bool {
        Action::ALL
            .iter()
            .position(|a| *a == action)
            .is_some_and(|slot| self.0[slot])
    }

    /// Whether nothing fired at all — the usual case, and worth an early
    /// return before a handler starts looking at the board.
    #[must_use]
    pub fn quiet(&self) -> bool {
        !self.0.iter().any(|f| *f)
    }
}

/// The chord a player just pressed, for a rebinding screen listening for one.
///
/// A modifier on its own is never a chord: a screen waiting for a binding
/// should sit still while shift goes down and take the key that follows.
#[must_use]
pub fn captured(keys: &ButtonInput<KeyCode>) -> Option<Chord> {
    let (shift, ctrl, alt, meta) = modifiers(keys);
    let code = keys.get_just_pressed().find(|c| key_name(**c).is_some())?;
    Some(Chord {
        key: key_name(*code)?.to_string(),
        shift,
        ctrl,
        alt,
        meta,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(down: &[KeyCode]) -> ButtonInput<KeyCode> {
        let mut keys = ButtonInput::default();
        for key in down {
            keys.press(*key);
        }
        keys
    }

    /// The stored keymap is only useful if every name in it resolves. A
    /// default binding this build cannot look up is a key that does nothing,
    /// with no error anywhere.
    #[test]
    fn every_default_binding_names_a_key_this_client_knows() {
        let map = Keymap::standard();
        for action in Action::ALL {
            for chord in map.chords(action) {
                assert!(
                    key_code(&chord.key).is_some(),
                    "{action:?} is bound to {:?}, which this client cannot resolve",
                    chord.key
                );
            }
        }
    }

    #[test]
    fn a_name_round_trips_through_the_table() {
        for (name, code) in TABLE {
            assert_eq!(key_code(name), Some(*code));
            assert_eq!(key_name(*code), Some(*name));
        }
    }

    #[test]
    fn a_binding_from_a_newer_client_is_ignored_rather_than_fatal() {
        assert_eq!(key_code("F24"), None);
        let mut map = Keymap::standard();
        map.bind(Action::Confirm, vec![Chord::key("MediaPlayPause")]);
        let keys = input(&[KeyCode::Enter]);
        assert!(!Binds::new(&keys, &map).just_pressed(Action::Confirm));
    }

    /// The bug this replaces: `W` and `⇧W` were told apart by an `if !shift`
    /// wrapped around half the handler, and every new binding had to remember
    /// to join in.
    #[test]
    fn a_modifier_makes_a_different_chord_not_an_extra_one() {
        let map = Keymap::standard();

        let keys = input(&[KeyCode::KeyW]);
        let binds = Binds::new(&keys, &map);
        assert!(binds.just_pressed(Action::CursorUp));
        assert!(!binds.just_pressed(Action::RailUp));

        let keys = input(&[KeyCode::ShiftLeft, KeyCode::KeyW]);
        let binds = Binds::new(&keys, &map);
        assert!(binds.just_pressed(Action::RailUp));
        assert!(
            !binds.just_pressed(Action::CursorUp),
            "holding shift walked the card cursor as well as the rail"
        );
    }

    #[test]
    fn either_side_of_a_modifier_does() {
        let map = Keymap::standard();
        for shift in [KeyCode::ShiftLeft, KeyCode::ShiftRight] {
            let keys = input(&[shift, KeyCode::KeyW]);
            assert!(Binds::new(&keys, &map).just_pressed(Action::RailUp));
        }
    }

    #[test]
    fn an_action_with_two_bindings_answers_to_both() {
        let map = Keymap::standard();
        for key in [KeyCode::ArrowUp, KeyCode::ArrowRight] {
            let keys = input(&[key]);
            assert!(Binds::new(&keys, &map).just_pressed(Action::NumberUp));
        }
    }

    #[test]
    fn a_rebinding_screen_waits_through_the_modifier_for_the_key() {
        let keys = input(&[KeyCode::ShiftLeft]);
        assert_eq!(captured(&keys), None, "shift alone was taken as a binding");
        let keys = input(&[KeyCode::ShiftLeft, KeyCode::KeyP]);
        assert_eq!(captured(&keys), Some(Chord::shift("KeyP")));
    }

    #[test]
    fn a_rebound_key_takes_effect_without_anything_being_rebuilt() {
        let mut map = Keymap::standard();
        map.bind(Action::Confirm, vec![Chord::key("KeyZ")]);
        let keys = input(&[KeyCode::KeyZ]);
        assert!(Binds::new(&keys, &map).just_pressed(Action::Confirm));
        let keys = input(&[KeyCode::Enter]);
        assert!(!Binds::new(&keys, &map).just_pressed(Action::Confirm));
    }
}
