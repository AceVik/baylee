//! Drafna's Restoration — {U} — Sorcery
//! Oracle: Put any number of target artifact cards from target player's graveyard on top of their library in any order.
//! Set: ATQ #8 — Antiquities | Scryfall ID: 4be2aa3b-207b-4d21-abfb-6788520c7676 | Oracle ID: 24a78116-d5e0-4e30-b192-d8bffe347a0c

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DRAFNA_S_RESTORATION,
    oracle_id = "24a78116-d5e0-4e30-b192-d8bffe347a0c",
    scryfall_id = "4be2aa3b-207b-4d21-abfb-6788520c7676",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Drafna's Restoration",
        mana_cost = mana!("{U}"),
        types = TypeSet::SORCERY,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[Effect::TargetedGraveyardToTop],
        targets = Some(TargetReq::one(TargetSpec::AnyPlayer)),
        second_targets = Some(TargetReq::up_to(
            TargetSpec::CardInFirstTargetsGraveyard(&Filter::ARTIFACT),
            u8::MAX
        ))
    )],
);
