//! Katabatic Winds — {2}{G} — Enchantment
//! Oracle: Phasing (This phases in or out before you untap during each of your untap steps. While it's phased out, it's treated as though it doesn't exist.)
//! Oracle: Creatures with flying can't attack or block, and their activated abilities with {T} in their costs can't be activated.
//! Set: VIS #109 — Visions | Scryfall ID: 97b34ce8-1eb2-44eb-813a-09d0308e27a0 | Oracle ID: 70446c56-07de-4d7f-94ae-4da88fba41cf
// PARTIAL — Preventing only activated abilities with {T} in their costs on creatures with flying is not yet supported by Modifier; in play, Katabatic Winds has phasing and prevents creatures with flying from attacking or blocking.

use baylee_cards_dsl::prelude::*;

static FLYING_CREATURE: Filter =
    Filter::And(&[Filter::CREATURE, Filter::HasKeyword(KeywordSet::FLYING)]);

card!(
    index = index::KATABATIC_WINDS,
    oracle_id = "70446c56-07de-4d7f-94ae-4da88fba41cf",
    scryfall_id = "97b34ce8-1eb2-44eb-813a-09d0308e27a0",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    keywords = KeywordSet::PHASING,
    coverage = Coverage::Partial(
        "Preventing only activated abilities with {T} in their costs on creatures with flying is not yet supported by Modifier; in play, Katabatic Winds has phasing and prevents creatures with flying from attacking or blocking.",
    ),
    faces = &[face!(
        name = "Katabatic Winds",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        static_ability!(FLYING_CREATURE, Modifier::CantAttack),
        static_ability!(FLYING_CREATURE, Modifier::CantBlock(&Filter::Any)),
    ],
);
