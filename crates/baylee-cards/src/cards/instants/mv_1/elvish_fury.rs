//! Elvish Fury — {G} — Instant
//! Oracle: Buyback {4} (You may pay an additional {4} as you cast this spell. If you do, put this card into your hand as it resolves.)
//! Oracle: Target creature gets +2/+2 until end of turn.
//! Set: MH1 #162 — Modern Horizons | Scryfall ID: e5a24943-0121-4c64-9624-2e4cff37e6ca | Oracle ID: 557d2af0-04c5-461d-826b-7208c4535dc2
// PARTIAL — Target creature gets +2/+2 until end of turn; Buyback {4} (paying an additional {4} as you cast this spell to put it into your hand as it resolves) is not yet modeled.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ELVISH_FURY,
    oracle_id = "557d2af0-04c5-461d-826b-7208c4535dc2",
    scryfall_id = "e5a24943-0121-4c64-9624-2e4cff37e6ca",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "Target creature gets +2/+2 until end of turn; Buyback {4} (paying an additional {4} as you cast this spell to put it into your hand as it resolves) is not yet modeled."
    ),
    faces = &[face!(
        name = "Elvish Fury",
        mana_cost = mana!("{G}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::PumpTarget {
            power: Amount::Fixed(2),
            toughness: Amount::Fixed(2),
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )],
);
