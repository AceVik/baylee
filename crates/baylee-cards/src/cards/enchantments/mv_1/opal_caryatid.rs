//! Opal Caryatid — {W} — Enchantment
//! Oracle: When an opponent casts a creature spell, if this permanent is an enchantment, it becomes a 2/2 Soldier creature.
//! Set: USG #24 — Urza's Saga | Scryfall ID: 3a8a2e24-c959-40a2-883d-c0114589cfe7 | Oracle ID: eabcb1bd-5317-4e01-8f18-94f01c12c852
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const OPPONENT_CREATURE_SPELL: Filter =
    Filter::And(&[Filter::CREATURE, Filter::ControlledByOpponent]);

card!(
    index = index::OPAL_CARYATID,
    oracle_id = "eabcb1bd-5317-4e01-8f18-94f01c12c852",
    scryfall_id = "3a8a2e24-c959-40a2-883d-c0114589cfe7",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Opal Caryatid",
        mana_cost = mana!("{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::SpellCast(&OPPONENT_CREATURE_SPELL),
        &[
            Effect::continuous(
                &Filter::This,
                Modifier::BecomeType {
                    types: TypeSet::CREATURE,
                    subtype: subtypes::creature::SOLDIER,
                },
                Duration::Indefinitely,
            ),
            Effect::continuous(&Filter::This, Modifier::SetPT(2, 2), Duration::Indefinitely),
        ],
        condition = Some(Condition::SourceMatches(&Filter::ENCHANTMENT)),
    )],
);
