//! Hidden Predators — {G} — Enchantment
//! Oracle: When an opponent controls a creature with power 4 or greater, if this permanent is an enchantment, it becomes a 4/4 Beast creature.
//! Set: USG #263 — Urza's Saga | Scryfall ID: 27f8ee86-a576-4f40-b165-be5b4558507d | Oracle ID: 3b143227-f373-4403-b246-ee1c9a0dc85a
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HIDDEN_PREDATORS,
    oracle_id = "3b143227-f373-4403-b246-ee1c9a0dc85a",
    scryfall_id = "27f8ee86-a576-4f40-b165-be5b4558507d",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Hidden Predators",
        mana_cost = mana!("{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::State(&Condition::OpponentControlCount(
            &Filter::And(&[
                Filter::CREATURE,
                Filter::ControlledByOpponent,
                Filter::PowerAtLeast(4),
            ]),
            1
        )),
        &[
            Effect::continuous(
                &Filter::This,
                Modifier::BecomeType {
                    types: TypeSet::CREATURE,
                    subtype: subtypes::creature::BEAST,
                },
                Duration::Indefinitely,
            ),
            Effect::continuous(&Filter::This, Modifier::SetPT(4, 4), Duration::Indefinitely),
        ],
        condition = Some(Condition::SourceMatches(&Filter::ENCHANTMENT)),
    )],
);
