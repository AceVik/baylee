//! Juxtapose — {3}{U} — Sorcery
//! Oracle: You and target player exchange control of the creature you each control with the greatest mana value. Then exchange control of artifacts the same way. If two or more permanents a player controls are tied for greatest, their controller chooses one of them.
//! Set: ME1 #41 — Masters Edition | Scryfall ID: 3b894a51-6d4e-4e0f-b42a-dbab594fb81d | Oracle ID: a52ccb40-e950-4cd0-93ab-50daa52b03f3
// IMPLEMENTED — sequential greatest-mana-value creature and artifact
// exchanges, with each controller choosing among their tied permanents.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::JUXTAPOSE,
    oracle_id = "a52ccb40-e950-4cd0-93ab-50daa52b03f3",
    scryfall_id = "3b894a51-6d4e-4e0f-b42a-dbab594fb81d",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Juxtapose",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::SORCERY,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[
            Effect::ExchangeGreatestManaValue {
                filter: &Filter::CREATURE,
                with: PlayerRel::Chosen,
            },
            Effect::ExchangeGreatestManaValue {
                filter: &Filter::ARTIFACT,
                with: PlayerRel::Chosen,
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::AnyPlayer)),
    )],
);
