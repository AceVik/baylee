//! Bala Ged Recovery // Bala Ged Sanctuary — {2}{G} — Sorcery // Land
//! Oracle: Return target card from your graveyard to your hand.
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {G}.
//! Set: ZNR #180 — Zendikar Rising | Scryfall ID: c5cb3052-358d-44a7-8cfd-cd31b236494a | Oracle ID: d2075f58-b0e9-4e85-b7e6-0523a27a1d5b
//! Face: Bala Ged Recovery — {2}{G} — Sorcery
//! Face: Bala Ged Sanctuary —  — Land
// IMPLEMENTED — the sorcery face returns a target card from your graveyard
// to your hand; the land face enters tapped and taps for {G}.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BALA_GED_RECOVERY,
    oracle_id = "d2075f58-b0e9-4e85-b7e6-0523a27a1d5b",
    scryfall_id = "c5cb3052-358d-44a7-8cfd-cd31b236494a",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[
        face!(
            name = "Bala Ged Recovery",
            mana_cost = mana!("{2}{G}"),
            types = TypeSet::SORCERY,
        ),
        face!(
            name = "Bala Ged Sanctuary",
            types = TypeSet::LAND,
            enter_modifiers = &[EnterModifier::Tapped],
            abilities = &[mana_ability!(&[Effect::mana(ManaColor::Green, 1)])],
        ),
    ],
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[Effect::GraveyardToHand {
            target: TargetSpec::CardInGraveyard(&Filter::Any, PlayerRel::You),
        }],
        targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(
            &Filter::Any,
            PlayerRel::You,
        )))
    )],
);
