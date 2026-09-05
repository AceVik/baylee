//! Darkbore Pathway // Slitherbore Pathway — (no cost) — Land // Land
//! Set: KHM #254 — Kaldheim | Scryfall ID: 87a4e5fe-161f-42da-9ca2-67c8e8970e94 | Oracle ID: 868e6e68-4367-4073-a864-235d5961ae56
//! Face: Darkbore Pathway —  — Land
//! Face: Slitherbore Pathway —  — Land
// IMPLEMENTED — MDFC land-face choice on play (CR 712.4a) + per-face
// mana abilities.

use baylee_cards_dsl::prelude::*;

static BACK_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::Green, 1)])];

card! {
    index: 403,
    oracle_id: "868e6e68-4367-4073-a864-235d5961ae56",
    scryfall_id: "87a4e5fe-161f-42da-9ca2-67c8e8970e94",
    color_identity: ColorSet::from_slice(&[Color::Black, Color::Green]),
    faces: &[
        face! {
            name: "Darkbore Pathway",
            types: TypeSet::LAND,
        },
        face! {
            name: "Slitherbore Pathway",
            types: TypeSet::LAND,
            abilities: BACK_MANA,
        },
    ],
    coverage: Coverage::Implemented,
    abilities: &[mana_ability!(&[Effect::mana(ManaColor::Black, 1)])],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_data() {
        assert_eq!(CARD.index.get(), 403);
        assert_eq!(CARD.oracle_id, "868e6e68-4367-4073-a864-235d5961ae56");
        assert_eq!(CARD.scryfall_id, "87a4e5fe-161f-42da-9ca2-67c8e8970e94");
        assert_eq!(CARD.faces[0].name, "Darkbore Pathway");
        assert_eq!(CARD.faces[0].types, TypeSet::LAND);
        assert_eq!(CARD.faces[1].name, "Slitherbore Pathway");
        assert_eq!(CARD.faces[1].types, TypeSet::LAND);
        assert_eq!(CARD.coverage, Coverage::Implemented);
        assert_eq!(
            CARD.color_identity,
            ColorSet::from_slice(&[Color::Black, Color::Green])
        );
        assert_eq!(CARD.abilities_for_face(0).len(), 1);
        assert_eq!(CARD.abilities_for_face(1).len(), 1);
    }
}
