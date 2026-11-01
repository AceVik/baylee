//! Whim of Volrath — {U} — Instant
//! Oracle: Buyback {2} (You may pay an additional {2} as you cast this spell. If you do, put this card into your hand as it resolves.)
//! Oracle: Change the text of target permanent by replacing all instances of one color word with another or one basic land type with another until end of turn. (For example, you may change "nonred creature" to "nongreen creature" or "plainswalk" to "swampwalk.")
//! Set: TMP #102 — Tempest | Scryfall ID: e259da60-c8bc-4a77-98ed-e529dc067732 | Oracle ID: 51a20c9f-68d2-471b-8622-67d481b53b92
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WHIM_OF_VOLRATH,
    oracle_id = "51a20c9f-68d2-471b-8622-67d481b53b92",
    scryfall_id = "e259da60-c8bc-4a77-98ed-e529dc067732",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Whim of Volrath",
        mana_cost = mana!("{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
