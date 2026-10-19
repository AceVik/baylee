//! Heroism — {2}{W} — Enchantment
//! Oracle: Sacrifice a white creature: For each attacking red creature, prevent all combat damage that would be dealt by that creature this turn unless its controller pays {2}{R}.
//! Set: FEM #6 — Fallen Empires | Scryfall ID: 08ee87a0-a7eb-4472-9045-85d11e8a1501 | Oracle ID: 74716642-fc8c-4f62-a556-154ed0b4af0e
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

const WHITE_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasColor(ColorSet::from_slice(&[Color::White])),
]);
const ATTACKING_RED_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Attacking,
    Filter::HasColor(ColorSet::from_slice(&[Color::Red])),
]);

card!(
    index = index::HEROISM,
    oracle_id = "74716642-fc8c-4f62-a556-154ed0b4af0e",
    scryfall_id = "08ee87a0-a7eb-4472-9045-85d11e8a1501",
    coverage = Coverage::Implemented,
    color_identity = ColorSet::from_slice(&[Color::Red, Color::White]),
    faces = &[face!(
        name = "Heroism",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[activated!(
        cost!(Sacrifice(&WHITE_CREATURE)),
        &[Effect::ForEachMatching {
            filter: &ATTACKING_RED_CREATURE,
            effects: &[Effect::PlayerMayPayManaOr {
                player: PlayerRel::EventPlayer,
                cost: mana!("{2}{R}"),
                effect: &Effect::ModifyEvent {
                    modifier: Modifier::AssignsNoCombatDamage,
                    duration: Duration::UntilEndOfTurn,
                },
            }],
        }],
    ),],
);
