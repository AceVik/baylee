//! Blightstep Pathway // Searstep Pathway — (no cost) — Land // Land
//! Set: KHM #252 — Kaldheim | Scryfall ID: 0ce39a19-f51d-4a35-ae80-5b82eb15fcff | Oracle ID: e580a229-e800-4746-9d37-c32fcef8de28
//! Face: Blightstep Pathway —  — Land
//! Face: Searstep Pathway —  — Land
// IMPLEMENTED — MDFC land-face choice on play (CR 712.4a) + per-face
// mana abilities.

use baylee_cards_dsl::prelude::*;

static BACK_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::Red, 1)])];

card! {
    index: 285,
    oracle_id: "e580a229-e800-4746-9d37-c32fcef8de28",
    scryfall_id: "0ce39a19-f51d-4a35-ae80-5b82eb15fcff",
    color_identity: ColorSet::from_slice(&[Color::Black, Color::Red]),
    faces: &[
        face! {
            name: "Blightstep Pathway",
            types: TypeSet::LAND,
        },
        face! {
            name: "Searstep Pathway",
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
        assert_eq!(CARD.index.get(), 285);
        assert_eq!(CARD.oracle_id, "e580a229-e800-4746-9d37-c32fcef8de28");
        assert_eq!(CARD.scryfall_id, "0ce39a19-f51d-4a35-ae80-5b82eb15fcff");
        assert_eq!(CARD.faces[0].name, "Blightstep Pathway");
        assert_eq!(CARD.faces[0].types, TypeSet::LAND);
        assert_eq!(CARD.faces[1].name, "Searstep Pathway");
        assert_eq!(CARD.faces[1].types, TypeSet::LAND);
        assert_eq!(CARD.coverage, Coverage::Implemented);
        assert_eq!(
            CARD.color_identity,
            ColorSet::from_slice(&[Color::Black, Color::Red])
        );
        assert_eq!(CARD.abilities_for_face(0).len(), 1);
        assert_eq!(CARD.abilities_for_face(1).len(), 1);
    }
}
