//! Phyrexian Negator — {2}{B} — Creature — Phyrexian Horror
//! Oracle: Trample
//! Oracle: Whenever this creature is dealt damage, sacrifice that many permanents.
//! Set: DDE #1 — Duel Decks: Phyrexia vs. the Coalition | Scryfall ID: 7b3543e0-c15f-4fed-9a68-8f590b40ab16 | Oracle ID: f22fcd17-bc24-41df-81ce-6d11208861b9
// IMPLEMENTED — 5/5 Phyrexian Horror with trample; whenever it is dealt damage, its controller sacrifices that many permanents.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PHYREXIAN_NEGATOR,
    oracle_id = "f22fcd17-bc24-41df-81ce-6d11208861b9",
    scryfall_id = "7b3543e0-c15f-4fed-9a68-8f590b40ab16",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    keywords = KeywordSet::TRAMPLE,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Phyrexian Negator",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::PHYREXIAN, subtypes::creature::HORROR],
        power = Some(5),
        toughness = Some(5),
    ),],
    abilities = &[triggered!(
        Trigger::DealtDamage(&Filter::This),
        &[Effect::SacrificeAmountForPlayers {
            who: PlayerRel::You,
            filter: &Filter::Any,
            amount: Amount::EventAmount,
        }],
    )],
);
