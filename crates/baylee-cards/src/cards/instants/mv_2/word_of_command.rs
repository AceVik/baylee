//! Word of Command — {B}{B} — Instant
//! Oracle: Look at target opponent's hand and choose a card from it. You control that player until Word of Command finishes resolving. The player plays that card if able. While doing so, the player can activate mana abilities only if they're from lands that player controls and only if mana they produce is spent to activate other mana abilities of lands the player controls and/or to play that card. If the chosen card is cast as a spell, you control the player while that spell is resolving.
//! Set: ME4 #103 — Masters Edition IV | Scryfall ID: 8b1be5ba-6e5e-4801-815b-5e82b9e72b3d | Oracle ID: e8ad3a77-b293-4d69-b080-27ca9f95d443
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WORD_OF_COMMAND,
    oracle_id = "e8ad3a77-b293-4d69-b080-27ca9f95d443",
    scryfall_id = "8b1be5ba-6e5e-4801-815b-5e82b9e72b3d",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Word of Command",
        mana_cost = mana!("{B}{B}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
