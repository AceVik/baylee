//! Shaman en-Kor — {1}{W} — Creature — Kor Cleric Shaman
//! Oracle: {0}: The next 1 damage that would be dealt to this creature this turn is dealt to target creature you control instead.
//! Oracle: {1}{W}: The next time a source of your choice would deal damage to target creature this turn, that damage is dealt to this creature instead.
//! Set: TPR #27 — Tempest Remastered | Scryfall ID: 02e48948-f4bb-443f-aa35-d59328283cfc | Oracle ID: 4d9d5dbb-25ab-41a5-a277-3ee5f9ef579b
// PARTIAL: {0} and {1}{W} creature-to-creature damage redirection abilities are not supported; plays as a 1/2 Kor Cleric Shaman.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SHAMAN_EN_KOR,
    oracle_id = "4d9d5dbb-25ab-41a5-a277-3ee5f9ef579b",
    scryfall_id = "02e48948-f4bb-443f-aa35-d59328283cfc",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "{0} and {1}{W} creature-to-creature damage redirection abilities are not supported; plays as a 1/2 Kor Cleric Shaman",
    ),
    faces = &[face!(
        name = "Shaman en-Kor",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[
            subtypes::creature::KOR,
            subtypes::creature::CLERIC,
            subtypes::creature::SHAMAN
        ],
        power = Some(1),
        toughness = Some(2),
    ),],
);
