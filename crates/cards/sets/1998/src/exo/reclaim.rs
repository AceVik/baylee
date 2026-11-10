//! Reclaim — {G} — Instant
//! Oracle: Put target card from your graveyard on top of your library.
//! Set: ORI #195 — Magic Origins | Scryfall ID: 88f3c600-66f1-4a5e-ba4f-61e9f7f7a1d9 | Oracle ID: 797155cd-faf4-4321-8629-c8c352392748
// IMPLEMENTED — all printed abilities are represented.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RECLAIM,
    oracle_id = "797155cd-faf4-4321-8629-c8c352392748",
    scryfall_id = "88f3c600-66f1-4a5e-ba4f-61e9f7f7a1d9",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Reclaim",
        mana_cost = mana!("{G}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::GraveyardToTop {
            target: TargetSpec::CardInGraveyard(&Filter::Any, PlayerRel::You),
        }],
        targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(
            &Filter::Any,
            PlayerRel::You
        )))
    ),],
);
