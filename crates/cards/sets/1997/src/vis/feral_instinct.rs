//! Feral Instinct — {1}{G} — Instant
//! Oracle: Target creature gets +1/+1 until end of turn.
//! Oracle: Draw a card at the beginning of the next turn's upkeep.
//! Set: VIS #107 — Visions | Scryfall ID: 20dec7cf-2865-4642-9022-d3006fd7ac30 | Oracle ID: f8f3fc11-25b4-4a23-9d79-f380a14f0420
// IMPLEMENTED — target creature gets +1/+1 until end of turn and draw a card at the beginning of the next turn's upkeep.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FERAL_INSTINCT,
    oracle_id = "f8f3fc11-25b4-4a23-9d79-f380a14f0420",
    scryfall_id = "20dec7cf-2865-4642-9022-d3006fd7ac30",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Feral Instinct",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::PumpTarget {
                power: Amount::Fixed(1),
                toughness: Amount::Fixed(1),
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn,
            },
            Effect::AtNextTurnsUpkeep {
                effects: &[Effect::draw(1)],
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )],
);
