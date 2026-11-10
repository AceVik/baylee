//! Spelljack — {3}{U}{U}{U} — Instant
//! Oracle: Counter target spell. If that spell is countered this way, exile it instead of putting it into its owner's graveyard. You may play it without paying its mana cost for as long as it remains exiled. (If it has X in its mana cost, X is 0.)
//! Set: JUD #51 — Judgment | Scryfall ID: 3eda8c7b-ce35-482a-bece-52a30cc78a9a | Oracle ID: 7687b2a7-816d-4416-979b-675e35e235fc
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SPELLJACK,
    oracle_id = "7687b2a7-816d-4416-979b-675e35e235fc",
    scryfall_id = "3eda8c7b-ce35-482a-bece-52a30cc78a9a",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Spelljack",
        mana_cost = mana!("{3}{U}{U}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
