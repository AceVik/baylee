//! Rix Maadi, Dungeon Palace — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {1}{B}{R}, {T}: Each player discards a card. Activate only as a sorcery.
//! Set: C19 #269 — Commander 2019 | Scryfall ID: f8429762-e756-43e0-81b4-9d0a34270040 | Oracle ID: a1bee68d-135b-4e30-8830-48a3315d13a9
// IMPLEMENTED — {T} for {C}, plus a sorcery-speed {1}{B}{R} activation that
// makes each player discard a card (Effect::DiscardForPlayers, no targets).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RIX_MAADI_DUNGEON_PALACE,
    oracle_id = "a1bee68d-135b-4e30-8830-48a3315d13a9",
    scryfall_id = "f8429762-e756-43e0-81b4-9d0a34270040",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Rix Maadi, Dungeon Palace",
        types = TypeSet::LAND,
    ),],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{1}{B}{R}", TapSelf),
            &[Effect::DiscardForPlayers {
                who: PlayerRel::EachPlayer,
                count: 1,
            }],
            timing = ActivationTiming::SorcerySpeed
        ),
    ],
);
