//! Elven Warhounds — {3}{G} — Creature — Dog
//! Oracle: Whenever this creature becomes blocked by a creature, put that creature on top of its owner's library.
//! Set: TMP #225 — Tempest | Scryfall ID: 29138c1e-11cb-488f-8e04-f5488e08a81e | Oracle ID: ae997fee-2c1f-43c8-9dba-8b409d8e449e
// PARTIAL

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ELVEN_WARHOUNDS,
    oracle_id = "ae997fee-2c1f-43c8-9dba-8b409d8e449e",
    scryfall_id = "29138c1e-11cb-488f-8e04-f5488e08a81e",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "Whenever this creature becomes blocked by a creature, returns that creature to its owner's hand; putting the blocking creature on top of its owner's library is not yet modeled."
    ),
    faces = &[face!(
        name = "Elven Warhounds",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DOG],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[triggered!(
        Trigger::BecomesBlockedBy(&Filter::CREATURE),
        &[Effect::bounce(TargetSpec::EventObject)],
    )],
);
