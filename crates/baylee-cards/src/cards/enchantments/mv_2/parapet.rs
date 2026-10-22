//! Parapet — {1}{W} — Enchantment
//! Oracle: You may cast this spell as though it had flash. If you cast it any time a sorcery couldn't have been cast, the controller of the permanent it becomes sacrifices it at the beginning of the next cleanup step.
//! Oracle: Creatures you control get +0/+1.
//! Set: VIS #14 — Visions | Scryfall ID: a7bbcaa9-edbf-48ad-bcd2-65e8fb9bb938 | Oracle ID: 4e73634a-9df8-4c36-b1d2-f366eae902fc
// PARTIAL: has flash and gives creatures you control +0/+1; sacrificing Parapet at the beginning of the next cleanup step if cast at instant speed is not yet supported.

use baylee_cards_dsl::prelude::*;

// NOT SUPPORTED: If you cast it any time a sorcery couldn't have been cast, the controller of the permanent it becomes sacrifices it at the beginning of the next cleanup step.
card!(
    index = index::PARAPET,
    oracle_id = "4e73634a-9df8-4c36-b1d2-f366eae902fc",
    scryfall_id = "a7bbcaa9-edbf-48ad-bcd2-65e8fb9bb938",
    color_identity = ColorSet::from_slice(&[Color::White]),
    keywords = KeywordSet::FLASH,
    coverage = Coverage::Partial(
        "has flash and gives creatures you control +0/+1; sacrificing Parapet at the beginning of the next cleanup step if cast at instant speed is not yet supported"
    ),
    faces = &[face!(
        name = "Parapet",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[static_ability!(Filter::YOUR_CREATURE, Modifier::ModifyPT(0, 1))],
);
