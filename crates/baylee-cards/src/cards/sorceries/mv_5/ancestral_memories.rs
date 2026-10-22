//! Ancestral Memories — {2}{U}{U}{U} — Sorcery
//! Oracle: Look at the top seven cards of your library. Put two of them into your hand and the rest into your graveyard.
//! Set: 7ED #59 — Seventh Edition | Scryfall ID: 100f0ca8-a66a-452f-be5f-17f631ba0ee0 | Oracle ID: 95a2802a-2621-40c3-84f8-51e8aad7b6f0
// NOT SUPPORTED: putting the remaining five cards into your graveyard is not supported by LookAtTopPick, which puts them on the bottom of your library.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ANCESTRAL_MEMORIES,
    oracle_id = "95a2802a-2621-40c3-84f8-51e8aad7b6f0",
    scryfall_id = "100f0ca8-a66a-452f-be5f-17f631ba0ee0",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "looks at the top seven cards of your library and puts two of them into your hand; puts the remaining cards on the bottom of your library instead of into your graveyard",
    ),
    faces = &[face!(
        name = "Ancestral Memories",
        mana_cost = mana!("{2}{U}{U}{U}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[Effect::LookAtTopPick {
        count: Amount::Fixed(7),
        pick: 2,
        random: false,
    }])],
);
