//! Barbed-Back Wurm — {4}{B} — Creature — Wurm
//! Oracle: {B}: Target green creature blocking this creature gets -1/-1 until end of turn.
//! Set: MIR #105 — Mirage | Scryfall ID: 1b96810d-72d3-4dee-a29f-cdf85ea5ce6f | Oracle ID: d60cbdf0-f20f-4306-be6b-eed90d2a2f77
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static GREEN_BLOCKING_SOURCE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasColor(ColorSet::from_slice(&[Color::Green])),
    Filter::BlockingSource,
]);

card!(
    index = index::BARBED_BACK_WURM,
    oracle_id = "d60cbdf0-f20f-4306-be6b-eed90d2a2f77",
    scryfall_id = "1b96810d-72d3-4dee-a29f-cdf85ea5ce6f",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Barbed-Back Wurm",
        mana_cost = mana!("{4}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WURM],
        power = Some(4),
        toughness = Some(3),
    ),],
    abilities = &[activated!(
        cost!("{B}"),
        &[Effect::ModifyTargets {
            modifier: Modifier::ModifyPT(-1, -1),
            duration: Duration::UntilEndOfTurn,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&GREEN_BLOCKING_SOURCE))),
    )],
);
