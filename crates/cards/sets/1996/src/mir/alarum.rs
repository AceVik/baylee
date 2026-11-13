//! Alarum — {1}{W} — Instant
//! Oracle: Untap target nonattacking creature. It gets +1/+3 until end of turn.
//! Set: MIR #2 — Mirage | Scryfall ID: 155f2aa6-6c47-4a06-b0ef-2d9205cd133e | Oracle ID: 18e96674-6fd8-4046-a3fe-7ba80138d85d
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

static NONATTACKING_CREATURE: Filter =
    Filter::And(&[Filter::CREATURE, Filter::Not(&Filter::Attacking)]);

card!(
    index = index::ALARUM,
    oracle_id = "18e96674-6fd8-4046-a3fe-7ba80138d85d",
    scryfall_id = "155f2aa6-6c47-4a06-b0ef-2d9205cd133e",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Alarum",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::UntapTarget,
            Effect::PumpTarget {
                power: Amount::Fixed(1),
                toughness: Amount::Fixed(3),
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn,
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&NONATTACKING_CREATURE))),
    )],
);
