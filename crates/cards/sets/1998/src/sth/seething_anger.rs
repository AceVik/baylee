//! Seething Anger — {R} — Sorcery
//! Oracle: Buyback {3} (You may pay an additional {3} as you cast this spell. If you do, put this card into your hand as it resolves.)
//! Oracle: Target creature gets +3/+0 until end of turn.
//! Set: TPR #156 — Tempest Remastered | Scryfall ID: 2294ef08-c27a-4bb4-bd59-434516717961 | Oracle ID: 2effcd1c-5833-4e0e-bb67-2d2ffb22a014

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SEETHING_ANGER,
    oracle_id = "2effcd1c-5833-4e0e-bb67-2d2ffb22a014",
    scryfall_id = "2294ef08-c27a-4bb4-bd59-434516717961",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Seething Anger",
        mana_cost = mana!("{R}"),
        types = TypeSet::SORCERY,
        buyback = Some(cost!("{3}")),
    ),],
    abilities = &[spell!(
        &[Effect::PumpTarget {
            power: Amount::Fixed(3),
            toughness: Amount::Fixed(0),
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    ),],
);
