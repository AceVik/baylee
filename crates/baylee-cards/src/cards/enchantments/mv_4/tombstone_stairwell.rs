//! Tombstone Stairwell — {2}{B}{B} — World Enchantment
//! Oracle: Cumulative upkeep {1}{B} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Oracle: At the beginning of each upkeep, if this enchantment is on the battlefield, each player creates a 2/2 black Zombie creature token with haste named Tombspawn for each creature card in their graveyard.
//! Oracle: At the beginning of each end step and when this enchantment leaves the battlefield, destroy all tokens created with this enchantment. They can't be regenerated.
//! Set: MIR #149 — Mirage | Scryfall ID: f8fe2f99-7ec2-490c-8ec3-aa2fb4680826 | Oracle ID: 9aaf1149-eccb-434a-a1a7-14b93fa50141
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TOMBSTONE_STAIRWELL,
    oracle_id = "9aaf1149-eccb-434a-a1a7-14b93fa50141",
    scryfall_id = "f8fe2f99-7ec2-490c-8ec3-aa2fb4680826",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Tombstone Stairwell",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::ENCHANTMENT,
        supertypes = SupertypeSet::WORLD,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
