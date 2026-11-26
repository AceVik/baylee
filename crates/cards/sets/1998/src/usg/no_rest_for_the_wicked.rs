//! No Rest for the Wicked — {1}{B} — Enchantment
//! Oracle: Sacrifice this enchantment: Return to your hand all creature cards in your graveyard that were put there from the battlefield this turn.
//! Set: 10E #165 — Tenth Edition | Scryfall ID: 65cfd454-5bc3-456c-9ca2-151af99f69ce | Oracle ID: d03fd358-ff3f-45ab-9601-07dbb535cb51
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::NO_REST_FOR_THE_WICKED,
    oracle_id = "d03fd358-ff3f-45ab-9601-07dbb535cb51",
    scryfall_id = "65cfd454-5bc3-456c-9ca2-151af99f69ce",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "No Rest for the Wicked",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
