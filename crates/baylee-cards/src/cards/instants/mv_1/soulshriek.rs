//! Soulshriek — {B} — Instant
//! Oracle: Target creature you control gets +X/+0 until end of turn, where X is the number of creature cards in your graveyard. Sacrifice that creature at the beginning of the next end step.
//! Set: MIR #145 — Mirage | Scryfall ID: 5c3bf717-2b88-4704-a7e9-f62dbb3d3d3d | Oracle ID: 77ae796e-fc56-47b9-b8a0-97e24fbd60f7
// IMPLEMENTED — pumps target creature you control +X/+0 for creature cards in your graveyard and sacrifices it at the next end step.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SOULSHRIEK,
    oracle_id = "77ae796e-fc56-47b9-b8a0-97e24fbd60f7",
    scryfall_id = "5c3bf717-2b88-4704-a7e9-f62dbb3d3d3d",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Soulshriek",
        mana_cost = mana!("{B}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::PumpTarget {
                power: Amount::CountOf {
                    filter: &Filter::CREATURE,
                    zone: ZoneSel::GraveyardYou,
                },
                toughness: Amount::Fixed(0),
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn,
            },
            Effect::AtNextEndStep {
                effects: &[Effect::SacrificeEvent],
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::YOUR_CREATURE))),
    )],
);
