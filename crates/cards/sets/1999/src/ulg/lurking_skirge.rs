//! Lurking Skirge — {1}{B} — Enchantment
//! Oracle: When a creature is put into an opponent's graveyard from the battlefield, if this permanent is an enchantment, it becomes a 3/2 Phyrexian Imp creature with flying.
//! Set: ULG #55 — Urza's Legacy | Scryfall ID: 9063cc12-a822-4488-856e-93d70ecfe37f | Oracle ID: 21e90420-aedb-4a84-ba57-d5eae497b09b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LURKING_SKIRGE,
    oracle_id = "21e90420-aedb-4a84-ba57-d5eae497b09b",
    scryfall_id = "9063cc12-a822-4488-856e-93d70ecfe37f",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Lurking Skirge",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
