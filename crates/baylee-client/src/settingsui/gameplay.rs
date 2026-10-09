//! Settings › Gameplay (the account's): automation's four rules, where to
//! stop (the rails and their presets), and the ability answers the client
//! keeps giving — today's Automation, re-homed (`DESIGN-v5` §12).
//!
//! Both rails stand at once here: every seat bar carries only the row its
//! turn belongs to, and a player arranging stops wants the whole
//! arrangement in front of them.

use baylee_client_core::automation::{RAIL_PHASES, RAIL_ROWS, RailPreset, RailSide};
use baylee_client_core::i18n::Phrase;
use baylee_client_core::prefs::AutoRule;
use baylee_client_core::settings_map::Row;
use bevy::prelude::*;

use crate::lobby::{Press, SettingsPress};
use crate::shellkit::controls::{self, Weight};

use super::View;
use super::rows::{Out, item};

/// The section.
pub(crate) fn gameplay(out: &mut Out, view: &View) {
    let lang = out.lang;
    let prefs = view.prefs;
    let mut lines = Vec::new();
    for (i, rule) in AutoRule::ALL.into_iter().enumerate() {
        let toggle = controls::toggle(
            out.commands,
            out.kit,
            rule.get(&prefs.auto),
            (
                Press::Settings(SettingsPress::ToggleAuto(rule)),
                item("automation", i),
            ),
        );
        let line = out.line(
            rule.label().text(lang),
            Some(rule.detail().text(lang)),
            &[toggle],
        );
        lines.push(line);
    }
    out.block(Row::Automation, &lines);

    // The presets above the rails they write: pick a starting point, then
    // correct it by hand. A preset is lit only while the rail still is it.
    let mut lines = Vec::new();
    for (i, preset) in RailPreset::ALL.into_iter().enumerate() {
        let on = prefs.orders.is(preset);
        let use_it = out.button_item(
            "rail-preset",
            i,
            Phrase::UsePreset.text(lang),
            if on {
                Weight::Primary
            } else {
                Weight::Secondary
            },
            Press::Settings(SettingsPress::SetRail(preset)),
        );
        lines.push(out.line(
            preset.label().text(lang),
            Some(preset.detail().text(lang)),
            &[use_it],
        ));
    }
    for side in RailSide::BOTH {
        let (caption, id) = match side {
            RailSide::Mine => (Phrase::YourTurns, "rail-mine"),
            RailSide::Theirs => (Phrase::TheirTurns, "rail-theirs"),
        };
        lines.push(out.words(caption.text(lang), true));
        let mut chips = Vec::new();
        for (i, step) in RAIL_ROWS.into_iter().enumerate() {
            let skipped = prefs.orders.is_skipped(side, step);
            chips.push(controls::chip(
                out.commands,
                out.kit,
                step.name().text(lang),
                skipped,
                None,
                false,
                (
                    Press::Settings(SettingsPress::ToggleRail(side, step)),
                    item(id, i),
                ),
            ));
        }
        lines.push(by_phase(out, &chips));
    }
    out.block(Row::Stops, &lines);

    let mut lines = Vec::new();
    if prefs.ability_orders.is_empty() {
        lines.push(out.words(Phrase::NoAbilityAnswers.text(lang), true));
    } else {
        lines.push(out.button(
            "ability-reset-all",
            Phrase::StackResetRules.text(lang),
            Weight::Secondary,
            Press::Settings(SettingsPress::ResetAbilityOrders),
        ));
        for (i, order) in prefs.ability_orders.iter().enumerate() {
            let name = baylee_cards::by_index(order.ability.card).map_or("—", |card| card.name());
            let label = Phrase::StackRuleName.fill(
                lang,
                &[name, &(order.ability.index.saturating_add(1)).to_string()],
            );
            let reset = out.button_item(
                "ability",
                i,
                Phrase::Reset.text(lang),
                Weight::Ghost,
                Press::Settings(SettingsPress::ForgetAbility(order.ability)),
            );
            lines.push(out.line(&label, None, &[reset]));
        }
    }
    out.block(Row::AbilityAnswers, &lines);
}

/// The stops' chips (one per [`RAIL_ROWS`] entry, in its order) stood in
/// the turn's five phases (CR 500.1), the run wrapping between phases: a
/// row never leaves "Cleanup" alone under the rest (4K pass, 09.10.2026).
/// A phase wider than the whole run (a phone's combat) still wraps inside
/// itself.
fn by_phase(out: &mut Out, chips: &[Entity]) -> Entity {
    let mut at = 0;
    let mut groups = Vec::new();
    for phase in RAIL_PHASES {
        let count = phase.rows().len();
        let group = out
            .commands
            .spawn((
                Node {
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: out.kit.m.px(8.0),
                    row_gap: out.kit.m.px(6.0),
                    align_items: AlignItems::Center,
                    max_width: Val::Percent(100.0),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .add_children(&chips[at..at + count])
            .id();
        at += count;
        groups.push(group);
    }
    debug_assert_eq!(at, chips.len(), "the phases hold every row");
    out.wrap(&groups)
}
