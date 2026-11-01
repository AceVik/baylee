//! Eye of Singularity — {3}{W} — World Enchantment
//! Oracle: When this enchantment enters, destroy each permanent with the same name as another permanent, except for basic lands. They can't be regenerated.
//! Oracle: Whenever a permanent other than a basic land enters, destroy all other permanents with that name. They can't be regenerated.
//! Set: VIS #4 — Visions | Scryfall ID: fa84e4ad-738a-4d23-a84c-06c39ff4200b | Oracle ID: e13edf92-cdba-4c24-b81b-088b1554fde6
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::EYE_OF_SINGULARITY,
    oracle_id = "e13edf92-cdba-4c24-b81b-088b1554fde6",
    scryfall_id = "fa84e4ad-738a-4d23-a84c-06c39ff4200b",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Eye of Singularity",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::ENCHANTMENT,
        supertypes = SupertypeSet::WORLD,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
