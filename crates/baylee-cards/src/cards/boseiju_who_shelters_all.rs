//! Boseiju, Who Shelters All — (no cost) — Legendary Land
//! Oracle: Boseiju enters tapped.
//! Oracle: {T}, Pay 2 life: Add {C}. If that mana is spent on an instant or sorcery spell, that spell can't be countered.
//! Set: CHK #273 — Champions of Kamigawa | Scryfall ID: 0180d9a8-992c-4d55-8ac4-33a587786993 | Oracle ID: 36937483-30cb-449a-8028-75017a124922
// IMPLEMENTED — enters tapped; {T}, pay 2 life: add {C} that makes instant or sorcery spells uncounterable.

use baylee_cards_dsl::prelude::*;

card! {
    index: 301,
    oracle_id: "36937483-30cb-449a-8028-75017a124922",
    scryfall_id: "0180d9a8-992c-4d55-8ac4-33a587786993",
    faces: &[
    face! {
        name: "Boseiju, Who Shelters All",
        types: TypeSet::LAND,
        supertypes: SupertypeSet::LEGENDARY,
        enter_modifiers: &[EnterModifier::Tapped],
    },
    ],
    coverage: Coverage::Implemented,
    abilities: &[
        mana_ability!(
            Cost {
                mana: ManaCost::ZERO,
                parts: &[CostPart::TapSelf, CostPart::PayLife(2)],
            },
            &[Effect::mana(ManaColor::Colorless, 1).restricted(
                &Filter::INSTANT_OR_SORCERY,
                SpendRider::Uncounterable,
            )],
        ),
    ],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_data() {
        assert_eq!(CARD.index.get(), 301);
        assert_eq!(CARD.oracle_id, "36937483-30cb-449a-8028-75017a124922");
        assert_eq!(CARD.scryfall_id, "0180d9a8-992c-4d55-8ac4-33a587786993");
        assert_eq!(CARD.faces[0].name, "Boseiju, Who Shelters All");
        assert_eq!(CARD.faces[0].types, TypeSet::LAND);
        assert_eq!(CARD.faces[0].supertypes, SupertypeSet::LEGENDARY);
        assert_eq!(CARD.faces[0].enter_modifiers, &[EnterModifier::Tapped]);
        assert_eq!(CARD.coverage, Coverage::Implemented);
        assert_eq!(CARD.abilities.len(), 1);
    }
}
