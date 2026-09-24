//! The settings screen: keys, automation, and the standing orders.
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
use baylee_client_core::atmosphere::Atmosphere;
use baylee_client_core::automation::{RAIL_ROWS, RailPreset, RailSide};
use baylee_client_core::cue::Loudness;
use baylee_client_core::i18n::{Lang, Phrase};
use baylee_client_core::prefs::{Action, AutoRule, Chord, Keymap, Preferences};
use baylee_client_core::sky::SkyMode;
use bevy::prelude::*;
use bevy::ui::{percent, px};

/// Draws the whole screen under `root`.
///
/// `capturing` is the action waiting for a key, if any: its row reads
/// "press a key…" and the next keystroke binds it.
#[allow(clippy::too_many_arguments)] // one screen, drawn from everything it shows
pub(crate) fn screen(
    commands: &mut Commands,
    root: Entity,
    prefs: &Preferences,
    capturing: Option<Action>,
    signed_in: bool,
    lang: Lang,
    fonts: &UiFonts,
    metrics: Metrics,
) {
    let header = row(commands, metrics, true);
    let title = heading(commands, fonts, metrics, Phrase::SettingsTitle.text(lang));
    let back = button(
        commands,
        fonts,
        metrics,
        Phrase::Back.text(lang),
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
                Phrase::SettingsOnAccount.text(lang)
            } else {
                Phrase::SettingsOnDevice.text(lang)
            }),
            tf(fonts, metrics.small),
            TextColor(palette::MUTED),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(header).add_child(note);
    commands.entity(root).add_child(header);
    // The language sits above the two columns, because it is the one
    // setting on this screen that decides how the rest of it reads.
    let tongue = language_row(commands, lang, fonts, metrics);
    commands.entity(root).add_child(tongue);

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

    let keys = keymap_panel(commands, &prefs.keymap, capturing, lang, fonts, metrics);
    let rules = automation_panel(commands, prefs, lang, fonts, metrics);
    commands.entity(columns).add_children(&[keys, rules]);
}

/// The keymap, grouped the way [`Action::group`] groups it.
fn keymap_panel(
    commands: &mut Commands,
    keymap: &Keymap,
    capturing: Option<Action>,
    lang: Lang,
    fonts: &UiFonts,
    metrics: Metrics,
) -> Entity {
    let column = panel(commands, metrics, percent(100), 1.0);
    let title = heading(commands, fonts, metrics, Phrase::Keys.text(lang));
    commands.entity(column).add_child(title);

    let mut group = None;
    for action in Action::ALL {
        if group != Some(action.group()) {
            group = Some(action.group());
            let label = commands
                .spawn((
                    Text::new(action.group().text(lang)),
                    tf(fonts, metrics.small),
                    TextColor(palette::ACCENT),
                    Pickable::IGNORE,
                ))
                .id();
            commands.entity(column).add_child(label);
        }
        let line = binding_row(commands, action, keymap, capturing, lang, fonts, metrics);
        commands.entity(column).add_child(line);
    }

    let reset = button(
        commands,
        fonts,
        metrics,
        Phrase::ResetAll.text(lang),
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
    lang: Lang,
    fonts: &UiFonts,
    metrics: Metrics,
) -> Entity {
    let line = row(commands, metrics, false);
    let label = commands
        .spawn((
            Text::new(action.label().text(lang)),
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
        Phrase::PressAKey.text(lang).to_string()
    } else {
        chords_of(keymap, action, lang)
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
fn chords_of(keymap: &Keymap, action: Action, lang: Lang) -> String {
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

/// The automation switches, and the standing orders underneath them.
///
/// This is where both rows are seen **at once**. Every seat bar carries the
/// twelve steps of a turn, but only the row that turn belongs to — an order
/// about opponents' turns is invisible on your own bar until an opponent is
/// taking one. A player arranging stops wants the whole arrangement in front
/// of them, and that is a settings screen rather than a table.
fn automation_panel(
    commands: &mut Commands,
    prefs: &Preferences,
    lang: Lang,
    fonts: &UiFonts,
    metrics: Metrics,
) -> Entity {
    let column = panel(commands, metrics, percent(100), 1.0);
    let title = heading(commands, fonts, metrics, Phrase::Automation.text(lang));
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
                        Text::new(rule.label().text(lang)),
                        tf(fonts, metrics.text),
                        TextColor(palette::INK),
                    ),
                    (
                        Text::new(rule.detail().text(lang)),
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
            if on {
                Phrase::SwitchOn.text(lang)
            } else {
                Phrase::SwitchOff.text(lang)
            },
            Press::ToggleAuto(rule),
            on,
        );
        commands.entity(line).add_children(&[text, switch]);
        commands.entity(column).add_child(line);
    }

    spawn_ability_orders(commands, column, prefs, lang, fonts, metrics);

    let motion = motion_row(commands, prefs, lang, fonts, metrics);
    commands.entity(column).add_child(motion);
    let weather = sky_row(commands, prefs, lang, fonts, metrics);
    commands.entity(column).add_child(weather);
    let air = atmosphere_row(commands, prefs, lang, fonts, metrics);
    commands.entity(column).add_child(air);
    let loudness = sound_row(commands, prefs, lang, fonts, metrics);
    commands.entity(column).add_child(loudness);

    let rail = heading(commands, fonts, metrics, Phrase::WhereToStop.text(lang));
    commands.entity(column).add_child(rail);
    let explain = commands
        .spawn((
            Text::new(Phrase::RailExplain.text(lang)),
            tf(fonts, metrics.small),
            TextColor(palette::MUTED),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(column).add_child(explain);

    // The presets sit above the rail they write, because that is the order
    // they are used in: pick a starting point, then correct it by hand. A chip
    // is lit only while the rail still *is* that preset, so the first hand
    // correction puts both of them out and the rail stops claiming to be
    // something a player has since edited.
    for preset in RailPreset::ALL {
        let line = preset_row(commands, prefs, preset, lang, fonts, metrics);
        commands.entity(column).add_child(line);
    }

    for side in RailSide::BOTH {
        let label = commands
            .spawn((
                Text::new(
                    match side {
                        RailSide::Mine => Phrase::YourTurns,
                        RailSide::Theirs => Phrase::TheirTurns,
                    }
                    .text(lang),
                ),
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
                step.name().text(lang),
                Press::ToggleRail(side, step),
                skipped,
            );
            commands.entity(strip).add_child(button);
        }
        commands.entity(column).add_child(strip);
    }
    column
}

/// One rail preset: its name, what it stops at, and the button that writes it.
///
/// A button rather than a switch, because a preset is not a state the rail is
/// *in* — it writes twenty-four buttons and then has nothing more to do with
/// them. It is nonetheless drawn lit while the rail still matches, which is
/// the only honest way to answer "am I on competitive stops right now".
fn preset_row(
    commands: &mut Commands,
    prefs: &Preferences,
    preset: RailPreset,
    lang: Lang,
    fonts: &UiFonts,
    metrics: Metrics,
) -> Entity {
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
                    Text::new(preset.label().text(lang)),
                    tf(fonts, metrics.text),
                    TextColor(palette::INK),
                ),
                (
                    Text::new(preset.detail().text(lang)),
                    tf(fonts, metrics.small),
                    TextColor(palette::MUTED),
                )
            ],
        ))
        .id();
    let on = prefs.orders.is(preset);
    let use_it = chip(
        commands,
        fonts,
        metrics,
        Phrase::UsePreset.text(lang),
        Press::SetRail(preset),
        on,
    );
    commands.entity(line).add_children(&[text, use_it]);
    line
}

fn spawn_ability_orders(
    commands: &mut Commands,
    column: Entity,
    prefs: &Preferences,
    lang: Lang,
    fonts: &UiFonts,
    metrics: Metrics,
) {
    if !prefs.ability_orders.is_empty() {
        let reset = chip(
            commands,
            fonts,
            metrics,
            Phrase::StackResetRules.text(lang),
            Press::ResetAbilityOrders,
            false,
        );
        commands.entity(column).add_child(reset);
        for order in &prefs.ability_orders {
            let line = row(commands, metrics, false);
            let name = baylee_cards::by_index(order.ability.card).map_or("—", |card| card.name());
            let label = Phrase::StackRuleName.fill(
                lang,
                &[name, &(order.ability.index.saturating_add(1)).to_string()],
            );
            let text = heading(commands, fonts, metrics, &label);
            let reset = chip(
                commands,
                fonts,
                metrics,
                Phrase::Reset.text(lang),
                Press::ForgetAbility(order.ability),
                false,
            );
            commands.entity(line).add_children(&[text, reset]);
            commands.entity(column).add_child(line);
        }
    }
}

/// Which language the interface speaks, offered as one chip per language.
///
/// Every language names itself in its own words — a player looking for
/// German is looking for "Deutsch", not for the English word for it — which
/// is also why this row is legible whatever language it is currently in.
fn language_row(commands: &mut Commands, lang: Lang, fonts: &UiFonts, metrics: Metrics) -> Entity {
    let line = row(commands, metrics, true);
    let label = commands
        .spawn((
            Text::new(Phrase::Language.text(lang)),
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
    for offered in Lang::ALL {
        let pick = chip(
            commands,
            fonts,
            metrics,
            offered.name(),
            Press::PickLang(offered),
            offered == lang,
        );
        commands.entity(line).add_child(pick);
    }
    line
}

/// The one switch on this screen that is about the table rather than about
/// the game.
///
/// It sits here because this is where a player looks for it, and it travels
/// with the account for the same reason the keys do: a player who cannot read
/// a moving board cannot read one on any machine.
fn motion_row(
    commands: &mut Commands,
    prefs: &Preferences,
    lang: Lang,
    fonts: &UiFonts,
    metrics: Metrics,
) -> Entity {
    let line = row(commands, metrics, false);
    let label = commands
        .spawn((
            Node {
                flex_grow: 1.0,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            Pickable::IGNORE,
            children![
                (
                    Text::new(Phrase::HoldTheTableStill.text(lang)),
                    tf(fonts, metrics.text),
                    TextColor(palette::INK),
                ),
                (
                    Text::new(Phrase::HoldTheTableStillWhy.text(lang)),
                    tf(fonts, metrics.small),
                    TextColor(palette::MUTED),
                )
            ],
        ))
        .id();
    let switch = chip(
        commands,
        fonts,
        metrics,
        if prefs.reduce_motion {
            Phrase::SwitchOn.text(lang)
        } else {
            Phrase::SwitchOff.text(lang)
        },
        Press::ToggleMotion,
        prefs.reduce_motion,
    );
    commands.entity(line).add_children(&[label, switch]);
    line
}

/// What is behind the table, and it is weather rather than a rule.
///
/// It sits beside the motion switch because it is the other setting on this
/// screen that is about the *table* and not about the game. The line under it
/// says so out loud: Magic has a day/night designation (CR 731) and this is
/// emphatically not it, so a player must never look at a starfield and
/// wonder whether something on the board has transformed.
fn sky_row(
    commands: &mut Commands,
    prefs: &Preferences,
    lang: Lang,
    fonts: &UiFonts,
    metrics: Metrics,
) -> Entity {
    let line = row(commands, metrics, true);
    let label = commands
        .spawn((
            Node {
                flex_grow: 1.0,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            Pickable::IGNORE,
            children![
                (
                    Text::new(Phrase::Sky.text(lang)),
                    tf(fonts, metrics.text),
                    TextColor(palette::INK),
                ),
                (
                    Text::new(Phrase::SkyWhy.text(lang)),
                    tf(fonts, metrics.small),
                    TextColor(palette::MUTED),
                )
            ],
        ))
        .id();
    commands.entity(line).add_child(label);
    for offered in SkyMode::ALL {
        let pick = chip(
            commands,
            fonts,
            metrics,
            sky_name(offered).text(lang),
            Press::PickSky(offered),
            offered == prefs.sky,
        );
        commands.entity(line).add_child(pick);
    }
    line
}

/// The weather picker: a heading, a line of why, and three chips.
///
/// Directly under the sky's row, because the two are read together — the sky
/// is what is *behind* the table and this is what is in the air *over* it, and
/// a player who has just decided on stars is the player deciding whether snow
/// falls through them.
fn atmosphere_row(
    commands: &mut Commands,
    prefs: &Preferences,
    lang: Lang,
    fonts: &UiFonts,
    metrics: Metrics,
) -> Entity {
    let line = row(commands, metrics, true);
    let label = commands
        .spawn((
            Node {
                flex_grow: 1.0,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            Pickable::IGNORE,
            children![
                (
                    Text::new(Phrase::Atmosphere.text(lang)),
                    tf(fonts, metrics.text),
                    TextColor(palette::INK),
                ),
                (
                    Text::new(Phrase::AtmosphereWhy.text(lang)),
                    tf(fonts, metrics.small),
                    TextColor(palette::MUTED),
                )
            ],
        ))
        .id();
    commands.entity(line).add_child(label);
    for offered in Atmosphere::ALL {
        let pick = chip(
            commands,
            fonts,
            metrics,
            atmosphere_name(offered).text(lang),
            Press::PickAtmosphere(offered),
            offered == prefs.atmosphere,
        );
        commands.entity(line).add_child(pick);
    }
    line
}

/// The label a weather step is offered under.
///
/// A `match` here rather than a method on [`Atmosphere`], for the reason
/// [`sky_name`] gives: the enum lives in the renderer-free crate beside the
/// arithmetic, and `Phrase` is the interface's own vocabulary.
fn atmosphere_name(air: Atmosphere) -> Phrase {
    match air {
        Atmosphere::Off => Phrase::AtmosphereOff,
        Atmosphere::Soft => Phrase::AtmosphereSoft,
        Atmosphere::Full => Phrase::AtmosphereFull,
    }
}

/// The loudness picker: a heading, a line of why, and three chips.
///
/// Beside the sky's row and built the same way, because they are the same
/// kind of setting — something the table does that changes nothing about the
/// game, offered as a small closed set rather than as a number to tune.
fn sound_row(
    commands: &mut Commands,
    prefs: &Preferences,
    lang: Lang,
    fonts: &UiFonts,
    metrics: Metrics,
) -> Entity {
    let line = row(commands, metrics, true);
    let label = commands
        .spawn((
            Node {
                flex_grow: 1.0,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            Pickable::IGNORE,
            children![
                (
                    Text::new(Phrase::Sound.text(lang)),
                    tf(fonts, metrics.text),
                    TextColor(palette::INK),
                ),
                (
                    Text::new(Phrase::SoundWhy.text(lang)),
                    tf(fonts, metrics.small),
                    TextColor(palette::MUTED),
                )
            ],
        ))
        .id();
    commands.entity(line).add_child(label);
    for offered in Loudness::ALL {
        let pick = chip(
            commands,
            fonts,
            metrics,
            sound_name(offered).text(lang),
            Press::PickSound(offered),
            offered == prefs.sound,
        );
        commands.entity(line).add_child(pick);
    }
    line
}

/// The label a loudness step is offered under.
///
/// A `match` here rather than a method on [`Loudness`], for the reason
/// [`sky_name`] gives: the enum lives in the renderer-free crate and
/// `Phrase` is the interface's own vocabulary. `Loudness::key` stays the
/// stored spelling.
fn sound_name(level: Loudness) -> Phrase {
    match level {
        Loudness::Full => Phrase::SoundFull,
        Loudness::Half => Phrase::SoundHalf,
        Loudness::Off => Phrase::SoundOff,
    }
}

/// The label a sky mode is offered under.
///
/// A `match` rather than a method on `SkyMode`, because the enum lives in the
/// renderer-free crate beside the arithmetic and `Phrase` is the interface's
/// own vocabulary. The wire spelling stays `SkyMode::key`; only the label is
/// translated, which is the same split `Lang` and the house AI names use.
fn sky_name(mode: SkyMode) -> Phrase {
    match mode {
        SkyMode::Auto => Phrase::SkyAuto,
        SkyMode::Day => Phrase::SkyDay,
        SkyMode::Night => Phrase::SkyNight,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One chip per language, each naming itself. A language with no chip is
    /// a language nobody can pick, and the enum is the only list there is.
    #[test]
    fn every_language_names_itself_and_is_offered() {
        let mut codes: Vec<&str> = Lang::ALL.iter().map(|l| l.code()).collect();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), Lang::ALL.len(), "two languages share a code");
        for lang in Lang::ALL {
            assert!(!lang.name().is_empty());
            assert!(!Phrase::Language.text(lang).is_empty());
        }
    }

    /// One chip per sky, each with a label in every language.
    ///
    /// The same rule as the languages above, and the same failure: a mode
    /// with no chip is a mode nobody can pick. Every language, because
    /// `Phrase` makes a missing German a compile error only for phrases that
    /// *exist* — a mode added without one would sail past that.
    #[test]
    fn every_sky_is_offered_and_named() {
        let mut keys: Vec<&str> = SkyMode::ALL.iter().map(|m| m.key()).collect();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), SkyMode::ALL.len(), "two skies share a key");
        for mode in SkyMode::ALL {
            for lang in Lang::ALL {
                assert!(
                    !sky_name(mode).text(lang).is_empty(),
                    "{:?} has no name in {}",
                    mode,
                    lang.code()
                );
            }
        }
    }

    /// The screen lists one row per action and one switch per rule. Both
    /// lists come from the `ALL` constants, so a new action that nobody added
    /// there is an action nobody can rebind — which is invisible until a
    /// player goes looking for it.
    #[test]
    fn every_action_and_every_rule_has_a_row() {
        assert_eq!(Action::ALL.len(), 28);
        assert_eq!(AutoRule::ALL.len(), 4);
        for action in Action::ALL {
            assert!(!action.label().text(Lang::En).is_empty());
            assert!(!action.group().text(Lang::En).is_empty());
        }
        for rule in AutoRule::ALL {
            assert!(!rule.label().text(Lang::En).is_empty());
            assert!(!rule.detail().text(Lang::En).is_empty());
        }
        for step in RAIL_ROWS {
            assert!(!step.name().text(Lang::En).is_empty());
        }
    }

    /// Actions are drawn under group headings, and the heading only changes
    /// when the group does — so `ALL` has to keep each group's actions
    /// together or the same heading appears three times.
    #[test]
    fn the_action_list_keeps_each_group_in_one_run() {
        let mut seen: Vec<Phrase> = Vec::new();
        for action in Action::ALL {
            if seen.last() != Some(&action.group()) {
                assert!(
                    !seen.contains(&action.group()),
                    "{} is split into two runs of the list",
                    action.group().text(Lang::En)
                );
                seen.push(action.group());
            }
        }
    }

    #[test]
    fn a_bound_action_reads_as_its_keys_and_an_unbound_one_says_so() {
        let mut keymap = Keymap::standard();
        assert_eq!(chords_of(&keymap, Action::Confirm, Lang::En), "Space");
        assert_eq!(chords_of(&keymap, Action::NumberUp, Lang::En), "↑  /  →");
        keymap.bind(Action::Confirm, vec![]);
        assert_eq!(chords_of(&keymap, Action::Confirm, Lang::En), "unbound");
    }
}
