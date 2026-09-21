//! Ruins of Oran-Rief — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {C}. ({C} represents colorless mana.)
//! Oracle: {T}: Put a +1/+1 counter on target colorless creature that entered this turn.
//! Set: CMM #1023 — Commander Masters | Scryfall ID: d1159ef6-f3ac-42a0-ae46-7d5eb9b3a6eb | Oracle ID: 7140f396-1bfa-4b28-ba28-fa15eba74652
// PARTIAL — enters tapped and {T}: Add {C} are built; putting a counter on a creature that entered this turn has no DSL filter.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RUINS_OF_ORAN_RIEF,
    oracle_id = "7140f396-1bfa-4b28-ba28-fa15eba74652",
    scryfall_id = "d1159ef6-f3ac-42a0-ae46-7d5eb9b3a6eb",
    faces = &[face!(
        name = "Ruins of Oran-Rief",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    coverage = Coverage::Partial(
        "putting a counter on a creature that entered this turn is not expressible: Filter has no variant for objects that entered the battlefield this turn"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: "{T}: Put a +1/+1 counter on target colorless creature that entered this turn."
    ],
);
