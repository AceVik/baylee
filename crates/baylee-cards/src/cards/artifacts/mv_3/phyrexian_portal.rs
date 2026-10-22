//! Phyrexian Portal — {3} — Artifact
//! Oracle: {3}: If your library has ten or more cards in it, target opponent looks at the top ten cards of your library and separates them into two face-down piles. Exile one of those piles. Search the other pile for a card, put it into your hand, then shuffle the rest of that pile into your library.
//! Set: ME2 #217 — Masters Edition II | Scryfall ID: c165b61b-3cd5-4035-a9d3-1c03289ae5ba | Oracle ID: 1a3ce329-ee48-4abc-b820-b43e9e68eae9
// NOT SUPPORTED: having target opponent look at the top ten cards of your library and separate them into two face-down piles, exiling one pile, and searching the other pile for a card has no DSL representation.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PHYREXIAN_PORTAL,
    oracle_id = "1a3ce329-ee48-4abc-b820-b43e9e68eae9",
    scryfall_id = "c165b61b-3cd5-4035-a9d3-1c03289ae5ba",
    coverage = Coverage::Partial(
        "Having target opponent look at the top ten cards of your library and separate them into two face-down piles, exiling one pile, and searching the other pile for a card has no DSL representation; in play, Phyrexian Portal's '{3}' ability looks at the top ten cards of your library, puts one into your hand, puts the rest on the bottom of your library, and shuffles your library.",
    ),
    faces = &[face!(
        name = "Phyrexian Portal",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{3}"),
        &[
            Effect::LookAtTopPick {
                count: Amount::Fixed(10),
                pick: 1,
                random: true,
            },
            Effect::ShuffleLibrary {
                who: PlayerRel::You,
            },
        ],
    )],
);
