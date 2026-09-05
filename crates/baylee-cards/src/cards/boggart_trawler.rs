//! Boggart Trawler // Boggart Bog — {2}{B} — Creature — Goblin // Land
//! Set: MH3 #243 — Modern Horizons 3 | Scryfall ID: d0d484a6-5610-4f1d-95ec-eda273c255e4 | Oracle ID: 727f3201-1cfc-4ab2-9dfe-be4f7251f42f
//! Face: Boggart Trawler — {2}{B} — Creature — Goblin
//! Face: Boggart Bog —  — Land
// IMPLEMENTED — ETB exiles target player's graveyard; back face enters tapped unless paying 3 life and taps for {B}.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static BOG_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::Black, 1)])];

card! {
    index: 294,
    oracle_id: "727f3201-1cfc-4ab2-9dfe-be4f7251f42f",
    scryfall_id: "d0d484a6-5610-4f1d-95ec-eda273c255e4",
    color_identity: ColorSet::from_slice(&[Color::Black]),
    coverage: Coverage::Implemented,
    faces: &[
        face! {
            name: "Boggart Trawler",
            mana_cost: baylee_core::mana!("{2}{B}"),
            types: TypeSet::CREATURE,
            subtypes: &[subtypes::creature::GOBLIN],
            power: Some(3),
            toughness: Some(1),
        },
        face! {
            name: "Boggart Bog",
            types: TypeSet::LAND,
            enter_modifiers: &[EnterModifier::TappedOrPayLife(3)],
            abilities: BOG_MANA,
        },
    ],
    abilities: &[triggered!(
        Trigger::EntersBattlefield(&Filter::This),
        &[Effect::ExileGraveyard {
            player: PlayerRel::Chosen,
        }],
        targets: Some(TargetReq::one(TargetSpec::AnyPlayer)),
    )],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_identity() {
        assert_eq!(CARD.index.get(), 294);
        assert_eq!(CARD.oracle_id, "727f3201-1cfc-4ab2-9dfe-be4f7251f42f");
        assert_eq!(CARD.scryfall_id, "d0d484a6-5610-4f1d-95ec-eda273c255e4");
        assert_eq!(CARD.color_identity, ColorSet::from_slice(&[Color::Black]));
        assert_eq!(CARD.coverage, Coverage::Implemented);
    }

    #[test]
    fn front_face_trawler() {
        let face = &CARD.faces[0];
        assert_eq!(face.name, "Boggart Trawler");
        assert_eq!(face.mana_cost, baylee_core::mana!("{2}{B}"));
        assert_eq!(face.types, TypeSet::CREATURE);
        assert_eq!(face.subtypes, &[subtypes::creature::GOBLIN]);
        assert_eq!(face.power, Some(3));
        assert_eq!(face.toughness, Some(1));
        assert_eq!(CARD.abilities_for_face(0).len(), 1);
    }

    #[test]
    fn back_face_bog() {
        let face = &CARD.faces[1];
        assert_eq!(face.name, "Boggart Bog");
        assert_eq!(face.types, TypeSet::LAND);
        assert_eq!(
            face.enter_modifiers,
            &[EnterModifier::TappedOrPayLife(3)]
        );
        assert_eq!(CARD.abilities_for_face(1).len(), 1);
    }
}

// Engine-level coverage: ETB exiles target player's graveyard;
// MDFC back face plays as land that enters tapped unless 3 life is paid
// and taps for {B}.
