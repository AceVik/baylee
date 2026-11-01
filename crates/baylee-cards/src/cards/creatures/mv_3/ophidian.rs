//! Ophidian — {2}{U} — Creature — Snake
//! Oracle: Whenever this creature attacks and isn't blocked, you may draw a card. If you do, this creature assigns no combat damage this turn.
//! Set: JVC #9 — Duel Decks Anthology: Jace vs. Chandra | Scryfall ID: 03be605e-dddf-4a1f-a4ee-0d71ab771e47 | Oracle ID: 953bdefa-4639-4524-af5c-5055d1289f40
// IMPLEMENTED — Whenever it attacks and isn't blocked, you may draw a card and have it assign no combat damage this turn.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::OPHIDIAN,
    oracle_id = "953bdefa-4639-4524-af5c-5055d1289f40",
    scryfall_id = "03be605e-dddf-4a1f-a4ee-0d71ab771e47",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Ophidian",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SNAKE],
        power = Some(1),
        toughness = Some(3),
    ),],
    abilities = &[triggered!(
        Trigger::AttacksAndIsntBlocked(&Filter::This),
        &[Effect::MayDo {
            effects: &[
                Effect::draw(1),
                Effect::ModifySource {
                    modifier: Modifier::AssignsNoCombatDamage,
                    duration: Duration::UntilEndOfTurn,
                },
            ],
        }],
    ),],
);
