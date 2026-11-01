//! Deep Analysis — {3}{U} — Sorcery
//! Oracle: Target player draws two cards.
//! Oracle: Flashback—{1}{U}, Pay 3 life. (You may cast this card from your graveyard for its flashback cost. Then exile it.)
//! Set: MSC #149 — Marvel Super Heroes Commander | Scryfall ID: 1a297bae-f259-4fc7-b76a-285f972d69e3 | Oracle ID: 579cbd92-797f-4cdf-91ed-fca7a523eae5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DEEP_ANALYSIS,
    oracle_id = "579cbd92-797f-4cdf-91ed-fca7a523eae5",
    scryfall_id = "1a297bae-f259-4fc7-b76a-285f972d69e3",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Deep Analysis",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
