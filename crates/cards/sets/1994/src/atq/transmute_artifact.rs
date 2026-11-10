//! Transmute Artifact — {U}{U} — Sorcery
//! Oracle: Sacrifice an artifact. If you do, search your library for an artifact card. If that card's mana value is less than or equal to the sacrificed artifact's mana value, put it onto the battlefield. If it's greater, you may pay {X}, where X is the difference. If you do, put it onto the battlefield. If you don't, put it into its owner's graveyard. Then shuffle.
//! Set: ME4 #69 — Masters Edition IV | Scryfall ID: 2888553d-5ed0-4e47-8cc2-1491b5557c53 | Oracle ID: dbe792f9-22be-4972-b418-99a6190c4421

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TRANSMUTE_ARTIFACT,
    oracle_id = "dbe792f9-22be-4972-b418-99a6190c4421",
    scryfall_id = "2888553d-5ed0-4e47-8cc2-1491b5557c53",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Transmute Artifact",
        mana_cost = mana!("{U}{U}"),
        types = TypeSet::SORCERY,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[spell!(&[Effect::SacrificeAndIf {
        filter: &Filter::ARTIFACT,
        check: &Filter::Any,
        effect: &Effect::Sequence(&[
            Effect::SearchWithManaDifference {
                filter: &Filter::ARTIFACT,
                value: Amount::SacrificedManaValue
            },
            Effect::ShuffleLibrary {
                who: PlayerRel::You
            },
        ]),
    }])],
);
