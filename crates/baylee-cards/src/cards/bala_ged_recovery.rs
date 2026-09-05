//! Bala Ged Recovery // Bala Ged Sanctuary — {2}{G} — Sorcery // Land
//! Set: ZNR #180 — Zendikar Rising | Scryfall ID: c5cb3052-358d-44a7-8cfd-cd31b236494a | Oracle ID: d2075f58-b0e9-4e85-b7e6-0523a27a1d5b
//! Face: Bala Ged Recovery — {2}{G} — Sorcery
//! Face: Bala Ged Sanctuary —  — Land
// IMPLEMENTED — return target card from your graveyard to your hand; back face enters tapped and taps for {G}.

use baylee_cards_dsl::prelude::*;

static SANCTUARY_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::Green, 1)])];

card! {
    index: 252,
    oracle_id: "d2075f58-b0e9-4e85-b7e6-0523a27a1d5b",
    scryfall_id: "c5cb3052-358d-44a7-8cfd-cd31b236494a",
    color_identity: ColorSet::from_slice(&[Color::Green]),
    coverage: Coverage::Implemented,
    faces: &[
        face! {
            name: "Bala Ged Recovery",
            mana_cost: baylee_core::mana!("{2}{G}"),
            types: TypeSet::SORCERY,
        },
        face! {
            name: "Bala Ged Sanctuary",
            types: TypeSet::LAND,
            enter_modifiers: &[EnterModifier::Tapped],
            abilities: SANCTUARY_MANA,
        },
    ],
    abilities: &[spell!(
        &[Effect::GraveyardToHand {
            target: TargetSpec::CardInGraveyard(&Filter::Any, PlayerRel::You),
        }],
        targets: Some(TargetReq::one(TargetSpec::CardInGraveyard(
            &Filter::Any,
            PlayerRel::You,
        ))),
    )],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_identity() {
        assert_eq!(CARD.index.get(), 252);
        assert_eq!(CARD.oracle_id, "d2075f58-b0e9-4e85-b7e6-0523a27a1d5b");
        assert_eq!(CARD.scryfall_id, "c5cb3052-358d-44a7-8cfd-cd31b236494a");
        assert_eq!(CARD.color_identity, ColorSet::from_slice(&[Color::Green]));
        assert_eq!(CARD.coverage, Coverage::Implemented);
    }

    #[test]
    fn front_face_recovery() {
        let face = &CARD.faces[0];
        assert_eq!(face.name, "Bala Ged Recovery");
        assert_eq!(face.mana_cost, baylee_core::mana!("{2}{G}"));
        assert_eq!(face.types, TypeSet::SORCERY);
        assert_eq!(CARD.abilities_for_face(0).len(), 1);
    }

    #[test]
    fn back_face_sanctuary() {
        let face = &CARD.faces[1];
        assert_eq!(face.name, "Bala Ged Sanctuary");
        assert_eq!(face.types, TypeSet::LAND);
        assert_eq!(face.enter_modifiers, &[EnterModifier::Tapped]);
        assert_eq!(CARD.abilities_for_face(1).len(), 1);
    }
}

// Engine-level coverage: GraveyardToHand moves target card from your
// graveyard to hand; MDFC back face plays as land that enters tapped
// and taps for {G}.
