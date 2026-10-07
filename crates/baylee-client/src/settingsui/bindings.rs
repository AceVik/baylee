//! Settings › Controls (`KEYBOARD.md` §5): the shell's shortcuts and the
//! table's keys, each with its key, a press that waits for the next key,
//! and a reset where it differs from the default.
//!
//! A shortcut onto a key another live action holds is refused with the
//! holder's name (`ShellKeymap::bind`), and a second request takes it
//! (Q19: refuse, then take); the table's keymap keeps its own rule.

use baylee_client_core::i18n::{Lang, Phrase};
use baylee_client_core::prefs::{Action, Chord, Keymap};
use baylee_client_core::settings_map::Row;
use baylee_client_core::shellkeys::{Refused, ShellAction, ShellKeymap};
use bevy::prelude::*;

use crate::lobby::{Press, SettingsPress};
use crate::shellkit::controls::Weight;

use super::View;
use super::rows::Out;

/// How an action's bindings read on one line.
pub(crate) fn chords_of(keymap: &Keymap, action: Action, lang: Lang) -> String {
    let chords = keymap.chords(action);
    if chords.is_empty() {
        // Unbinding is allowed — a pointer reaches everything — so this is a
        // state to name, not an error to hide.
        return Phrase::Unbound.text(lang).to_string();
    }
    chords
        .iter()
        .map(Chord::display)
        .collect::<Vec<_>>()
        .join("  /  ")
}

/// A shortcut's key as its cap reads.
fn shell_key(map: &ShellKeymap, action: ShellAction, lang: Lang) -> String {
    map.hint(
        action,
        crate::shellkit::keys::mac(),
        &baylee_client_core::shellkeys::Learnt::default(),
    )
    .unwrap_or_else(|| Phrase::Unbound.text(lang).to_string())
}

/// The refusal a shortcut's rebind met, in words.
pub(crate) fn refusal(refused: Refused, lang: Lang) -> String {
    match refused {
        Refused::Fixed => Phrase::KeyFixed.text(lang).to_string(),
        Refused::Held(holder) => Phrase::KeyHeldBy.fill(lang, &[holder.label().text(lang)]),
    }
}

/// The section: shortcuts, then the table's keys, then Reset all.
pub(crate) fn controls(out: &mut Out, view: &View) {
    let lang = out.lang;
    let prefs = view.prefs;
    let capturing = view.state.settings_capturing();
    let mut lines = Vec::new();
    for (i, action) in ShellAction::ALL.into_iter().enumerate() {
        let waiting = view.state.shell_capturing() == Some(action);
        let text = if waiting {
            Phrase::PressAKey.text(lang).to_string()
        } else {
            shell_key(&prefs.shell_keys, action, lang)
        };
        let key = out.button_item(
            "shell-keys",
            i,
            &text,
            if waiting {
                Weight::Primary
            } else {
                Weight::Secondary
            },
            Press::Settings(SettingsPress::RebindShell(action)),
        );
        let mut controls = vec![key];
        if prefs.shell_keys.chords(action) != action.defaults().as_slice() {
            controls.push(out.button_item(
                "shell-reset",
                i,
                Phrase::Reset.text(lang),
                Weight::Ghost,
                Press::Settings(SettingsPress::ResetShell(action)),
            ));
        }
        let help = view
            .state
            .shell_refusal()
            .filter(|(a, ..)| *a == action)
            .map(|(_, refused)| refusal(refused, lang));
        if help.is_some() {
            controls.push(out.button(
                "take",
                Phrase::KeyTake.text(lang),
                Weight::Primary,
                Press::Settings(SettingsPress::TakeShell),
            ));
        }
        let line = out.line(action.label().text(lang), help.as_deref(), &controls);
        lines.push(line);
    }
    out.block(Row::ShellKeys, &lines);
    let mut lines = Vec::new();
    let mut group = None;
    for (i, action) in Action::ALL.into_iter().enumerate() {
        if group != Some(action.group()) {
            group = Some(action.group());
            lines.push(out.words(action.group().text(lang), true));
        }
        let waiting = capturing == Some(action);
        let text = if waiting {
            Phrase::PressAKey.text(lang).to_string()
        } else {
            chords_of(&prefs.keymap, action, lang)
        };
        let key = out.button_item(
            "table-keys",
            i,
            &text,
            if waiting {
                Weight::Primary
            } else {
                Weight::Secondary
            },
            Press::Settings(SettingsPress::Rebind(action)),
        );
        let mut controls = vec![key];
        if prefs.keymap.chords(action) != Keymap::standard().chords(action) {
            controls.push(out.button_item(
                "table-reset",
                i,
                Phrase::Reset.text(lang),
                Weight::Ghost,
                Press::Settings(SettingsPress::ResetBinding(action)),
            ));
        }
        lines.push(out.line(action.label().text(lang), None, &controls));
    }
    let all = out.button(
        "reset-all",
        Phrase::ResetAll.text(lang),
        Weight::Secondary,
        Press::Settings(SettingsPress::ResetAllBindings),
    );
    lines.push(all);
    out.block(Row::TableKeys, &lines);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bound_action_reads_as_its_keys_and_an_unbound_one_says_so() {
        let mut keymap = Keymap::standard();
        assert_eq!(chords_of(&keymap, Action::Confirm, Lang::En), "Space");
        assert_eq!(chords_of(&keymap, Action::NumberUp, Lang::En), "↑  /  →");
        keymap.bind(Action::Confirm, vec![]);
        assert_eq!(chords_of(&keymap, Action::Confirm, Lang::En), "unbound");
    }

    #[test]
    fn a_refusal_names_the_holder() {
        let said = refusal(Refused::Held(ShellAction::Search), Lang::En);
        assert!(
            said.contains(ShellAction::Search.label().text(Lang::En)),
            "{said}"
        );
    }
}
