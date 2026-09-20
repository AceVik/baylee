//! Hall of the Bandit Lord — (no cost) — Legendary Land
//! Oracle: Hall of the Bandit Lord enters tapped.
//! Oracle: {T}, Pay 3 life: Add {C}. If that mana is spent on a creature spell, it gains haste.
//! Set: CHK #277 — Champions of Kamigawa | Scryfall ID: 59fa5bab-8626-4b45-a3a3-621f6d9509ab | Oracle ID: 32fe7ac4-86f5-44af-9f73-ee8f6a9ce2ba
// PARTIAL — enters tapped, and {T} plus 3 life for {C}: the mana's "spent on
// a creature spell, it gains haste" rider has no variant.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HALL_OF_THE_BANDIT_LORD,
    oracle_id = "32fe7ac4-86f5-44af-9f73-ee8f6a9ce2ba",
    scryfall_id = "59fa5bab-8626-4b45-a3a3-621f6d9509ab",
    faces = &[face!(
        name = "Hall of the Bandit Lord",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    coverage = Coverage::Partial(
        "the mana's rider is not expressible: SpendRider is None/Uncounterable/Scry and has no \"it gains haste\"",
    ),
    abilities = &[
        // NOT SUPPORTED: "If that mana is spent on a creature spell, it gains
        // haste." `ManaRestriction`'s `rider` cannot say it, and marking the
        // mana restricted to creature spells would be a different card: the
        // land prints no restriction on what the {C} buys.
        mana_ability!(
            cost!(TapSelf, PayLife(3)),
            &[Effect::mana(ManaColor::Colorless, 1)]
        ),
    ],
);
