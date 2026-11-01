//! Elven Warhounds — {3}{G} — Creature — Dog
//! Oracle: Whenever this creature becomes blocked by a creature, put that creature on top of its owner's library.
//! Set: TMP #225 — Tempest | Scryfall ID: 29138c1e-11cb-488f-8e04-f5488e08a81e | Oracle ID: ae997fee-2c1f-43c8-9dba-8b409d8e449e
// PARTIAL — Only the 2/2 creature body is supported; putting the blocking creature on top of its owner's library is unsupported.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ELVEN_WARHOUNDS,
    oracle_id = "ae997fee-2c1f-43c8-9dba-8b409d8e449e",
    scryfall_id = "29138c1e-11cb-488f-8e04-f5488e08a81e",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "Only the 2/2 creature body is supported; putting the blocking creature on top of its owner's library is unsupported."
    ),
    faces = &[face!(
        name = "Elven Warhounds",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DOG],
        power = Some(2),
        toughness = Some(2),
    ),],
    // NOT SUPPORTED: the blocking creature must go on top of its owner's library, not into its hand.
    abilities = &[],
);
