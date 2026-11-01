//! Kjeldoran Guard — {1}{W} — Creature — Human Soldier
//! Oracle: {T}: Target creature gets +1/+1 until end of turn. When that creature leaves the battlefield this turn, sacrifice this creature. Activate only during combat and only if defending player controls no snow lands.
//! Set: ICE #35 — Ice Age | Scryfall ID: bdf41f17-8f82-4a8c-adec-0f3804faff3b | Oracle ID: bd699cf8-56c7-4375-920e-8dd22c83a82c
// PARTIAL — body only; choosing a defending player for the activation's
// snow-land restriction is unavailable before attackers are declared.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::KJELDORAN_GUARD,
    oracle_id = "bd699cf8-56c7-4375-920e-8dd22c83a82c",
    scryfall_id = "bdf41f17-8f82-4a8c-adec-0f3804faff3b",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "body only; defending-player selection at activation is unavailable, so the optional pump and its linked sacrifice obligation are omitted together",
    ),
    faces = &[face!(
        name = "Kjeldoran Guard",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SOLDIER],
        power = Some(1),
        toughness = Some(1),
    ),],
    // NOT SUPPORTED: choosing a defending player at activation to check that
    // player's snow lands. Before attackers are declared, the existing filter
    // instead includes all opponents' lands. The whole optional ability,
    // including its linked sacrifice obligation, is omitted.
);
