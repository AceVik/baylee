//! Necromancy — {2}{B} — Enchantment
//! Oracle: You may cast this spell as though it had flash. If you cast it any time a sorcery couldn't have been cast, the controller of the permanent it becomes sacrifices it at the beginning of the next cleanup step.
//! Oracle: When this enchantment enters, if it's on the battlefield, it becomes an Aura with "enchant creature put onto the battlefield with Necromancy." Put target creature card from a graveyard onto the battlefield under your control and attach this enchantment to it. When this enchantment leaves the battlefield, that creature's controller sacrifices it.
//! Set: MKC #131 — Murders at Karlov Manor Commander | Scryfall ID: 3dd34a1a-e76b-46ee-be8d-5f7f4a092648 | Oracle ID: 699b192c-f3f2-4ac5-9ca9-e35bcf08ad8a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::NECROMANCY,
    oracle_id = "699b192c-f3f2-4ac5-9ca9-e35bcf08ad8a",
    scryfall_id = "3dd34a1a-e76b-46ee-be8d-5f7f4a092648",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Necromancy",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
