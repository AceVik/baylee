//! Flash of Insight — {X}{1}{U} — Instant
//! Oracle: Look at the top X cards of your library. Put one of them into your hand and the rest on the bottom of your library in any order.
//! Oracle: Flashback—{1}{U}, Exile X blue cards from your graveyard. (You may cast this card from your graveyard for its flashback cost, then exile it. You can't exile Flash of Insight to pay for its own flashback cost.)
//! Set: JUD #40 — Judgment | Scryfall ID: ffaab905-0b97-42c2-a1a3-1e72275caa82 | Oracle ID: 9196c8dc-bca6-4c35-949e-8c14a699a41d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FLASH_OF_INSIGHT,
    oracle_id = "9196c8dc-bca6-4c35-949e-8c14a699a41d",
    scryfall_id = "ffaab905-0b97-42c2-a1a3-1e72275caa82",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Flash of Insight",
        mana_cost = mana!("{X}{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
