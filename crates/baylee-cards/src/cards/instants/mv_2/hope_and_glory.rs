//! Hope and Glory — {1}{W} — Instant
//! Oracle: Untap two target creatures. Each of them gets +1/+1 until end of turn.
//! Set: ULG #9 — Urza's Legacy | Scryfall ID: 7cc6478f-4ae5-4f26-baa9-b28e992f962e | Oracle ID: 506ea048-aca5-439a-a5b5-059ddda0672a
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HOPE_AND_GLORY,
    oracle_id = "506ea048-aca5-439a-a5b5-059ddda0672a",
    scryfall_id = "7cc6478f-4ae5-4f26-baa9-b28e992f962e",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Hope and Glory",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::UntapTarget,
            Effect::PumpTarget {
                power: Amount::Fixed(1),
                toughness: Amount::Fixed(1),
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn,
            },
        ],
        targets = Some(TargetReq::exactly(TargetSpec::Object(&Filter::CREATURE), 2)),
    )],
);
