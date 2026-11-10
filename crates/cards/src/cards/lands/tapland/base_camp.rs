//! Base Camp — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add one mana of any color. Spend this mana only to cast a Cleric, Rogue, Warrior, or Wizard spell or to activate an ability of a Cleric, Rogue, Warrior, or Wizard.
//! Set: ZNR #257 — Zendikar Rising | Scryfall ID: dc85412e-333d-4e7d-8c85-40618cf1b6c2 | Oracle ID: 41fbf835-baee-4530-9155-e2c1b9045567

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

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
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(&[Effect::mana_of_any_color().restricted_for(
            &Filter::Or(&[
                Filter::HasSubtype(creature::CLERIC),
                Filter::HasSubtype(creature::ROGUE),
                Filter::HasSubtype(creature::WARRIOR),
                Filter::HasSubtype(creature::WIZARD),
            ]),
            SpendRider::None,
            ManaUse::SpellsOrAbilities,
        )]),
    ],
);
