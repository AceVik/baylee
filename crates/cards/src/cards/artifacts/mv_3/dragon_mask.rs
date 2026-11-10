//! Dragon Mask — {3} — Artifact
//! Oracle: {3}, {T}: Target creature you control gets +2/+2 until end of turn. Return it to its owner's hand at the beginning of the next end step. (Return it only if it's on the battlefield.)
//! Set: 6ED #283 — Classic Sixth Edition | Scryfall ID: b5109270-b052-489c-951c-d4b21a41ff6f | Oracle ID: fce29dd8-23b1-465c-a977-526f716dc8f6
// IMPLEMENTED — {3}, {T} pumps target creature you control +2/+2 until end of turn and returns it to its owner's hand at the beginning of the next end step.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DRAGON_MASK,
    oracle_id = "fce29dd8-23b1-465c-a977-526f716dc8f6",
    scryfall_id = "b5109270-b052-489c-951c-d4b21a41ff6f",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Dragon Mask",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{3}", TapSelf),
        &[
            Effect::PumpTarget {
                power: Amount::Fixed(2),
                toughness: Amount::Fixed(2),
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn,
            },
            Effect::AtNextEndStep {
                effects: &[Effect::ReturnToHand {
                    target: TargetSpec::EventObject,
                }],
            },
        ],
        target = Some(TargetSpec::Object(&Filter::YOUR_CREATURE)),
    )],
);
