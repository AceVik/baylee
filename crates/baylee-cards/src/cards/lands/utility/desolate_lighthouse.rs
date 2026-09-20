//! Desolate Lighthouse — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {1}{U}{R}, {T}: Draw a card, then discard a card.
//! Set: LCC #327 — The Lost Caverns of Ixalan Commander | Scryfall ID: c155cc10-09f4-424b-b366-142adf9c1712 | Oracle ID: aa6dbdf2-2379-4ff5-8a6c-70258784dc35
// IMPLEMENTED — {T} for {C}, and the loot: draw one, then discard one.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DESOLATE_LIGHTHOUSE,
    oracle_id = "aa6dbdf2-2379-4ff5-8a6c-70258784dc35",
    scryfall_id = "c155cc10-09f4-424b-b366-142adf9c1712",
    color_identity = ColorSet::from_slice(&[Color::Red, Color::Blue]),
    faces = &[face!(name = "Desolate Lighthouse", types = TypeSet::LAND,)],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{1}{U}{R}", TapSelf),
            &[
                Effect::draw(1),
                Effect::DiscardForPlayers {
                    who: PlayerRel::You,
                    count: 1
                }
            ]
        ),
    ],
);
