//! Dryad Arbor — (no cost) — Land Creature — Forest Dryad
//! Oracle: (This land isn't a spell, it's affected by summoning sickness, and it has "{T}: Add {G}.")
//! Set: DSC #273 — Duskmourn: House of Horror Commander | Scryfall ID: e3ddbebf-72cd-4d1b-ba0d-d94934654ab7 | Oracle ID: e996cd67-739c-40f4-b276-0042acf26c71
// IMPLEMENTED — intrinsic Forest mana ability: {T}: Add {G}.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 446,
    oracle_id: "e996cd67-739c-40f4-b276-0042acf26c71",
    scryfall_id: "e3ddbebf-72cd-4d1b-ba0d-d94934654ab7",
    color_identity: ColorSet::from_slice(&[Color::Green]),
    coverage: Coverage::Implemented,
    faces: &[
    face! {
        name: "Dryad Arbor",
        types: TypeSet::LAND.union(TypeSet::CREATURE),
        subtypes: &[subtypes::land::FOREST, subtypes::creature::DRYAD],
        power: Some(1),
        toughness: Some(1),
    },
    ],
    abilities: &[mana_ability!(&[Effect::mana(ManaColor::Green, 1)])],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_data() {
        assert_eq!(CARD.index.get(), 446);
        assert_eq!(CARD.oracle_id, "e996cd67-739c-40f4-b276-0042acf26c71");
        assert_eq!(CARD.scryfall_id, "e3ddbebf-72cd-4d1b-ba0d-d94934654ab7");
        assert_eq!(CARD.faces[0].name, "Dryad Arbor");
        assert_eq!(
            CARD.faces[0].types,
            TypeSet::LAND.union(TypeSet::CREATURE)
        );
        assert_eq!(
            CARD.faces[0].subtypes,
            &[subtypes::land::FOREST, subtypes::creature::DRYAD]
        );
        assert_eq!(CARD.faces[0].power, Some(1));
        assert_eq!(CARD.faces[0].toughness, Some(1));
        assert_eq!(CARD.coverage, Coverage::Implemented);
        assert_eq!(
            CARD.color_identity,
            ColorSet::from_slice(&[Color::Green])
        );
        assert_eq!(CARD.abilities.len(), 1);
    }
}

