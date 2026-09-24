//! Base Camp — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add one mana of any color. Spend this mana only to cast a Cleric, Rogue, Warrior, or Wizard spell or to activate an ability of a Cleric, Rogue, Warrior, or Wizard.
//! Set: ZNR #257 — Zendikar Rising | Scryfall ID: dc85412e-333d-4e7d-8c85-40618cf1b6c2 | Oracle ID: 41fbf835-baee-4530-9155-e2c1b9045567
// PARTIAL — enters tapped, {C}, and the any-color line carrying the four
// tribes as its spend restriction; the restriction's second half has no shape.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card!(
    index = index::BASE_CAMP,
    oracle_id = "41fbf835-baee-4530-9155-e2c1b9045567",
    scryfall_id = "dc85412e-333d-4e7d-8c85-40618cf1b6c2",
    faces = &[face!(
        name = "Base Camp",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    coverage = Coverage::Partial(
        "the any-color mana can pay only for Cleric, Rogue, Warrior or Wizard \
         spells: \"or to activate an ability of a Cleric, Rogue, Warrior, or \
         Wizard\" has no shape, because a ManaRestriction's filter is asked only \
         of a spell being cast and restricted mana never pays for an activated \
         ability"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: "or to activate an ability of a Cleric, Rogue,
        // Warrior, or Wizard" — one ManaRestriction holds one filter, applied
        // to the spell on the stack; restricted mana pays for no activation.
        mana_ability!(&[Effect::mana_of_any_color().restricted(
            &Filter::Or(&[
                Filter::HasSubtype(creature::CLERIC),
                Filter::HasSubtype(creature::ROGUE),
                Filter::HasSubtype(creature::WARRIOR),
                Filter::HasSubtype(creature::WIZARD),
            ]),
            SpendRider::None,
        )]),
    ],
);
