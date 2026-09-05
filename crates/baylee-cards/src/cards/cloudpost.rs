//! Cloudpost — (no cost) — Land — Locus
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {C} for each Locus on the battlefield.
//! Set: MRD #280 — Mirrodin | Scryfall ID: 2f28ecdc-a4f0-4327-a78c-340be41555ee | Oracle ID: f705c0eb-9c6c-4315-a860-208ed0c5d93e
// IMPLEMENTED — enters tapped, {T} to add {C} for each Locus on the battlefield.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static LOCUS: Filter = Filter::HasSubtype(subtypes::land::LOCUS);

card! {
    index: 361,
    oracle_id: "f705c0eb-9c6c-4315-a860-208ed0c5d93e",
    scryfall_id: "2f28ecdc-a4f0-4327-a78c-340be41555ee",
    faces: &[
    face! {
        name: "Cloudpost",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::LOCUS],
        enter_modifiers: &[EnterModifier::Tapped],
    },
    ],
    coverage: Coverage::Implemented,
    abilities: &[
        mana_ability!(&[Effect::mana_dynamic(
            ManaColor::Colorless,
            Amount::CountOf {
                filter: &LOCUS,
                zone: ZoneSel::Battlefield,
            },
        )]),
    ],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_data() {
        assert_eq!(CARD.index.get(), 361);
        assert_eq!(CARD.oracle_id, "f705c0eb-9c6c-4315-a860-208ed0c5d93e");
        assert_eq!(CARD.scryfall_id, "2f28ecdc-a4f0-4327-a78c-340be41555ee");
        assert_eq!(CARD.faces[0].name, "Cloudpost");
        assert_eq!(CARD.faces[0].types, TypeSet::LAND);
        assert_eq!(CARD.faces[0].subtypes, &[subtypes::land::LOCUS]);
        assert_eq!(CARD.faces[0].enter_modifiers, &[EnterModifier::Tapped]);
        assert_eq!(CARD.coverage, Coverage::Implemented);
        assert_eq!(CARD.abilities.len(), 1);
    }
}
