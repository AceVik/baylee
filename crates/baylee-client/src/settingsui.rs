//! The settings screen: keys, automation, and the phase rail.
//!
//! It lives beside the lobby rather than inside a duel for one reason — this
//! is where the account is. Everything on this screen belongs to the account
//! and travels with it (`docs/protocol.md` §"Client preferences"), and a
//! player who has not signed in still gets to change all of it, they just
//! keep it on this machine.
//!
//! Its own module rather than another few hundred lines of `lobby.rs`, which
//! is already the largest file in the crate. It borrows that module's
//! `Metrics`, `Press` and widget helpers, so the screen looks like every
//! other screen without a second copy of any of them.

use crate::hud::{UiFonts, palette, tf};
use crate::lobby::{Metrics, Press, button, chip, heading, panel, row};
use baylee_client_core::automation::{RAIL_ROWS, RailSide};
use baylee_client_core::prefs::{Action, AutoRule, Chord, Keymap, Preferences};
use bevy::prelude::*;
use bevy::ui::{percent, px};

/// Draws the whole screen under `root`.
///
/// `capturing` is the action waiting for a key, if any: its row reads
/// "press a key…" and the next keystroke binds it.
pub(crate) fn screen(
    commands: &mut Commands,
    root: Entity,
    prefs: &Preferences,
    capturing: Option<Action>,
    signed_in: bool,
    fonts: &UiFonts,
    metrics: Metrics,
) {
    let header = row(commands, metrics, true);
    let title = heading(commands, fonts, metrics, "Settings");
    let back = button(
        commands,
        fonts,
        metrics,
        "Back",
        Press::CloseSettings,
        palette::PANEL_LIT,
        true,
    );
    commands.entity(header).add_children(&[back, title]);
    // Where these are kept is not a detail a player should have to guess at:
    // one of the two lines below is always true, and which one decides
    // whether their keys are on this laptop or on their account.
    let note = commands
        .spawn((
            Text::new(if signed_in {
                "Saved to your account — these travel with you to any table."
            } else {
                "Saved on this computer. Sign in and they follow your account."
            }),
            tf(fonts, metrics.small),
            TextColor(palette::MUTED),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(header).add_child(note);
    commands.entity(root).add_child(header);

    let columns = commands
        .spawn((
            Node {
                width: percent(100),
                flex_grow: 1.0,
                flex_direction: if metrics.frame == crate::lobby::Frame::Phone {
                    FlexDirection::Column
                } else {
                    FlexDirection::Row
                },
                column_gap: px(metrics.gap),
                row_gap: px(metrics.gap),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(root).add_child(columns);

    let keys = keymap_panel(commands, &prefs.keymap, capturing, fonts, metrics);
    let rules = automation_panel(commands, prefs, fonts, metrics);
    commands.entity(columns).add_children(&[keys, rules]);
}

/// The keymap, grouped the way [`Action::group`] groups it.
fn keymap_panel(
    commands: &mut Commands,
    keymap: &Keymap,
    capturing: Option<Action>,
    fonts: &UiFonts,
    metrics: Metrics,
) -> Entity {
    let column = panel(commands, metrics, percent(100), 1.0);
    let title = heading(commands, fonts, metrics, "Keys");
    commands.entity(column).add_child(title);

    let mut group = "";
    for action in Action::ALL {
        if action.group() != group {
            group = action.group();
            let label = commands
                .spawn((
                    Text::new(group),
                    tf(fonts, metrics.small),
                    TextColor(palette::ACCENT),
                    Pickable::IGNORE,
                ))
                .id();
            commands.entity(column).add_child(label);
        }
        let line = binding_row(commands, action, keymap, capturing, fonts, metrics);
        commands.entity(column).add_child(line);
    }

    let reset = button(
        commands,
        fonts,
        metrics,
        "Reset every key",
        Press::ResetAllBindings,
        palette::PANEL_LIT,
        true,
    );
    commands.entity(column).add_child(reset);
    column
}

/// One action and the chord (or chords) bound to it.
fn binding_row(
    commands: &mut Commands,
    action: Action,
    keymap: &Keymap,
    capturing: Option<Action>,
    fonts: &UiFonts,
    metrics: Metrics,
) -> Entity {
    let line = row(commands, metrics, false);
    let label = commands
        .spawn((
            Text::new(action.label()),
            tf(fonts, metrics.text),
            TextColor(palette::INK),
            Node {
                flex_grow: 1.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(line).add_child(label);

    // While a row is capturing, the whole panel is still drawn — a player who
    // starts a rebinding and changes their mind can press escape, or click
    // anything else, without losing the screen.
    let waiting = capturing == Some(action);
    let text = if waiting {
        "press a key…".to_string()
    } else {
        chords_of(keymap, action)
    };
    let key = chip(
        commands,
        fonts,
        metrics,
        &text,
        Press::Rebind(action),
        waiting,
    );
    commands.entity(line).add_child(key);

    // Only offered where it would do something: a row already on its default
    // has nothing to put back, and a reset button that never changes anything
    // is a button that teaches a player to distrust the others.
    if keymap.chords(action) != Keymap::standard().chords(action) {
        let reset = chip(
            commands,
            fonts,
            metrics,
            "↺",
            Press::ResetBinding(action),
            false,
        );
        commands.entity(line).add_child(reset);
    }
    line
}

/// How an action's bindings read on one line.
fn chords_of(keymap: &Keymap, action: Action) -> String {
    let chords = keymap.chords(action);
    if chords.is_empty() {
        // Unbinding is allowed — a pointer reaches everything — so this is a
        // state to name, not an error to hide.
        return "unbound".to_string();
    }
    chords
        .iter()
        .map(Chord::display)
        .collect::<Vec<_>>()
        .join("  /  ")
}

/// The automation switches, and the phase rail underneath them.
fn automation_panel(
    commands: &mut Commands,
    prefs: &Preferences,
    fonts: &UiFonts,
    metrics: Metrics,
) -> Entity {
    let column = panel(commands, metrics, percent(100), 1.0);
    let title = heading(commands, fonts, metrics, "Automation");
    commands.entity(column).add_child(title);

    for rule in AutoRule::ALL {
        let line = row(commands, metrics, false);
        let text = commands
            .spawn((
                Node {
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
                Pickable::IGNORE,
                children![
                    (
                        Text::new(rule.label()),
                        tf(fonts, metrics.text),
                        TextColor(palette::INK),
                    ),
                    (
                        Text::new(rule.detail()),
                        tf(fonts, metrics.small),
                        TextColor(palette::MUTED),
                    )
                ],
            ))
            .id();
        let on = rule.get(&prefs.auto);
        let switch = chip(
            commands,
            fonts,
            metrics,
            if on { "on" } else { "off" },
            Press::ToggleAuto(rule),
            on,
        );
        commands.entity(line).add_children(&[text, switch]);
        commands.entity(column).add_child(line);
    }

    let rail = heading(commands, fonts, metrics, "Where to stop");
    commands.entity(column).add_child(rail);
    let explain = commands
        .spawn((
            Text::new(
                "A red step is one the client passes for you. Nothing is red \
                 until you make it red.",
            ),
            tf(fonts, metrics.small),
            TextColor(palette::MUTED),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(column).add_child(explain);

    for side in RailSide::BOTH {
        let label = commands
            .spawn((
                Text::new(match side {
                    RailSide::Mine => "Your turns",
                    RailSide::Theirs => "Opponents' turns",
                }),
                tf(fonts, metrics.small),
                TextColor(palette::ACCENT),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(column).add_child(label);
        let strip = row(commands, metrics, true);
        for step in RAIL_ROWS {
            let skipped = prefs.orders.is_skipped(side, step);
            let button = chip(
                commands,
                fonts,
                metrics,
                step.name(),
                Press::ToggleRail(side, step),
                skipped,
            );
            commands.entity(strip).add_child(button);
        }
        commands.entity(column).add_child(strip);
    }
    column
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The screen lists one row per action and one switch per rule. Both
    /// lists come from the `ALL` constants, so a new action that nobody added
    /// there is an action nobody can rebind — which is invisible until a
    /// player goes looking for it.
    #[test]
    fn every_action_and_every_rule_has_a_row() {
        assert_eq!(Action::ALL.len(), 25);
        assert_eq!(AutoRule::ALL.len(), 4);
        for action in Action::ALL {
            assert!(!action.label().is_empty());
            assert!(!action.group().is_empty());
        }
        for rule in AutoRule::ALL {
            assert!(!rule.label().is_empty());
            assert!(!rule.detail().is_empty());
        }
        for step in RAIL_ROWS {
            assert!(!step.name().is_empty());
        }
    }

    /// Actions are drawn under group headings, and the heading only changes
    /// when the group does — so `ALL` has to keep each group's actions
    /// together or the same heading appears three times.
    #[test]
    fn the_action_list_keeps_each_group_in_one_run() {
        let mut seen: Vec<&str> = Vec::new();
        for action in Action::ALL {
            if seen.last() != Some(&action.group()) {
                assert!(
                    !seen.contains(&action.group()),
                    "{} is split into two runs of the list",
                    action.group()
                );
                seen.push(action.group());
            }
        }
    }

    #[test]
    fn a_bound_action_reads_as_its_keys_and_an_unbound_one_says_so() {
        let mut keymap = Keymap::standard();
        assert_eq!(chords_of(&keymap, Action::Confirm), "Enter");
        assert_eq!(chords_of(&keymap, Action::NumberUp), "↑  /  →");
        keymap.bind(Action::Confirm, vec![]);
        assert_eq!(chords_of(&keymap, Action::Confirm), "unbound");
    }
}
