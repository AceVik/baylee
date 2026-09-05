//! Cragcrown Pathway // Timbercrown Pathway — (no cost) — Land // Land
//! Set: ZNR #261 — Zendikar Rising | Scryfall ID: da57eb54-5199-4a56-95f7-f6ac432876b1 | Oracle ID: 727ca426-f4cc-4218-8ae5-8c427af2e816
//! Face: Cragcrown Pathway —  — Land
//! Face: Timbercrown Pathway —  — Land
// IMPLEMENTED — MDFC land-face choice on play (CR 712.4a) + per-face
// mana abilities.

use baylee_cards_dsl::prelude::*;

static BACK_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::Green, 1)])];

card! {
    index: 381,
    oracle_id: "727ca426-f4cc-4218-8ae5-8c427af2e816",
    scryfall_id: "da57eb54-5199-4a56-95f7-f6ac432876b1",
    color_identity: ColorSet::from_slice(&[Color::Green, Color::Red]),
    faces: &[
        face! {
            name: "Cragcrown Pathway",
            types: TypeSet::LAND,
        },
        face! {
            name: "Timbercrown Pathway",
            types: TypeSet::LAND,
            abilities: BACK_MANA,
        },
    ],
    coverage: Coverage::Implemented,
    abilities: &[mana_ability!(&[Effect::mana(ManaColor::Red, 1)])],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_data() {
        assert_eq!(CARD.index.get(), 381);
        assert_eq!(CARD.oracle_id, "727ca426-f4cc-4218-8ae5-8c427af2e816");
        assert_eq!(CARD.scryfall_id, "da57eb54-5199-4a56-95f7-f6ac432876b1");
        assert_eq!(CARD.faces[0].name, "Cragcrown Pathway");
        assert_eq!(CARD.faces[0].types, TypeSet::LAND);
        assert_eq!(CARD.faces[1].name, "Timbercrown Pathway");
        assert_eq!(CARD.faces[1].types, TypeSet::LAND);
        assert_eq!(CARD.coverage, Coverage::Implemented);
        assert_eq!(
            CARD.color_identity,
            ColorSet::from_slice(&[Color::Green, Color::Red])
        );
        assert_eq!(CARD.abilities_for_face(0).len(), 1);
        assert_eq!(CARD.abilities_for_face(1).len(), 1);
    }
}
