//! Branchloft Pathway // Boulderloft Pathway — (no cost) — Land // Land
//! Set: ZNR #258 — Zendikar Rising | Scryfall ID: 0511e232-2a72-40f5-a400-4f7ebc442d17 | Oracle ID: 7c304547-a4b1-46c9-baed-16d2bfbe16eb
//! Face: Branchloft Pathway —  — Land
//! Face: Boulderloft Pathway —  — Land
// IMPLEMENTED — MDFC land-face choice on play (CR 712.4a) + per-face
// mana abilities.

use baylee_cards_dsl::prelude::*;

static BACK_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::White, 1)])];

card! {
    index: 308,
    oracle_id: "7c304547-a4b1-46c9-baed-16d2bfbe16eb",
    scryfall_id: "0511e232-2a72-40f5-a400-4f7ebc442d17",
    color_identity: ColorSet::from_slice(&[Color::Green, Color::White]),
    faces: &[
        face! {
            name: "Branchloft Pathway",
            types: TypeSet::LAND,
        },
        face! {
            name: "Boulderloft Pathway",
            types: TypeSet::LAND,
            abilities: BACK_MANA,
        },
    ],
    coverage: Coverage::Implemented,
    abilities: &[mana_ability!(&[Effect::mana(ManaColor::Green, 1)])],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_data() {
        assert_eq!(CARD.index.get(), 308);
        assert_eq!(CARD.oracle_id, "7c304547-a4b1-46c9-baed-16d2bfbe16eb");
        assert_eq!(CARD.scryfall_id, "0511e232-2a72-40f5-a400-4f7ebc442d17");
        assert_eq!(CARD.faces[0].name, "Branchloft Pathway");
        assert_eq!(CARD.faces[0].types, TypeSet::LAND);
        assert_eq!(CARD.faces[1].name, "Boulderloft Pathway");
        assert_eq!(CARD.faces[1].types, TypeSet::LAND);
        assert_eq!(CARD.coverage, Coverage::Implemented);
        assert_eq!(
            CARD.color_identity,
            ColorSet::from_slice(&[Color::Green, Color::White])
        );
        assert_eq!(CARD.abilities_for_face(0).len(), 1);
        assert_eq!(CARD.abilities_for_face(1).len(), 1);
    }
}
