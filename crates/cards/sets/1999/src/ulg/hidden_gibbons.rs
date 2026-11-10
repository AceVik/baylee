//! Hidden Gibbons — {G} — Enchantment
//! Oracle: When an opponent casts an instant spell, if this permanent is an enchantment, it becomes a 4/4 Ape creature.
//! Set: ULG #104 — Urza's Legacy | Scryfall ID: 8f94da51-b4d9-4d79-9113-39f8f4a1be34 | Oracle ID: 2315b044-bfd2-40dd-a73e-4cf064b36999
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const OPPONENT_INSTANT: Filter = Filter::And(&[
    Filter::HasType(TypeSet::INSTANT),
    Filter::ControlledByOpponent,
]);

card!(
    index = index::HIDDEN_GIBBONS,
    oracle_id = "2315b044-bfd2-40dd-a73e-4cf064b36999",
    scryfall_id = "8f94da51-b4d9-4d79-9113-39f8f4a1be34",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Hidden Gibbons",
        mana_cost = mana!("{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::SpellCast(&OPPONENT_INSTANT),
        &[
            Effect::continuous(
                &Filter::This,
                Modifier::BecomeType {
                    types: TypeSet::CREATURE,
                    subtype: subtypes::creature::APE,
                },
                Duration::Indefinitely,
            ),
            Effect::continuous(&Filter::This, Modifier::SetPT(4, 4), Duration::Indefinitely),
        ],
        condition = Some(Condition::SourceMatches(&Filter::ENCHANTMENT)),
    )],
);
