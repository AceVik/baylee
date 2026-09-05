//! Base Camp — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add one mana of any color. Spend this mana only to cast a Cleric, Rogue, Warrior, or Wizard spell or to activate an ability of a Cleric, Rogue, Warrior, or Wizard.
//! Set: ZNR #257 — Zendikar Rising | Scryfall ID: dc85412e-333d-4e7d-8c85-40618cf1b6c2 | Oracle ID: 41fbf835-baee-4530-9155-e2c1b9045567
// IMPLEMENTED — enters tapped; {T}: {C}; {T}: any color restricted to
// Cleric/Rogue/Warrior/Wizard spells (ability-activation half is a
// payment-solver refinement; spells only today — SpendRider::None).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

static PARTY_FILTER: Filter = Filter::Or(&[
    Filter::HasSubtype(creature::CLERIC),
    Filter::HasSubtype(creature::ROGUE),
    Filter::HasSubtype(creature::WARRIOR),
    Filter::HasSubtype(creature::WIZARD),
]);

card! {
    index: 263,
    oracle_id: "41fbf835-baee-4530-9155-e2c1b9045567",
    scryfall_id: "dc85412e-333d-4e7d-8c85-40618cf1b6c2",
    faces: &[
    face! {
        name: "Base Camp",
        types: TypeSet::LAND,
        enter_modifiers: &[EnterModifier::Tapped],
    },
    ],
    coverage: Coverage::Implemented,
    abilities: &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(&[Effect::mana_of_any_color().restricted(&PARTY_FILTER, SpendRider::None)]),
    ],
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_cards_dsl::prelude::*;

    #[test]
    fn base_camp_is_a_land() {
        let face = &CARD.faces[0];
        assert!(face.types.contains(TypeSet::LAND));
    }

    #[test]
    fn base_camp_enters_tapped() {
        let face = &CARD.faces[0];
        assert!(face.enter_modifiers.iter().any(|m| matches!(m, EnterModifier::Tapped)));
    }

    #[test]
    fn base_camp_has_two_mana_abilities() {
        assert_eq!(CARD.abilities.len(), 2);
    }

    #[test]
    fn base_camp_is_implemented() {
        assert!(matches!(CARD.coverage, Coverage::Implemented));
    }

    #[test]
    fn base_camp_has_no_color_identity() {
        assert!(CARD.color_identity.is_empty());
    }
}
