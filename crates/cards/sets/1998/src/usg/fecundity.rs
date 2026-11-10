//! Fecundity — {2}{G} — Enchantment
//! Oracle: Whenever a creature dies, that creature's controller may draw a card.
//! Set: USG #251 — Urza's Saga | Scryfall ID: 54c93c11-448c-402d-ac55-8d5ab4c83d7b | Oracle ID: ffa64ac6-fe55-48b8-b015-849982cd7ad8
// IMPLEMENTED — each dying creature's controller decides whether to draw.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FECUNDITY,
    oracle_id = "ffa64ac6-fe55-48b8-b015-849982cd7ad8",
    scryfall_id = "54c93c11-448c-402d-ac55-8d5ab4c83d7b",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Fecundity",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::Dies(&Filter::CREATURE),
        &[Effect::MayDoFor {
            player: PlayerRel::ControllerOfEvent,
            effects: &[Effect::DrawCardsFor {
                amount: Amount::Fixed(1),
                who: PlayerRel::ControllerOfEvent,
            }],
        }],
    )],
);
