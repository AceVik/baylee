//! Bloodscent — {3}{G} — Instant
//! Oracle: All creatures able to block target creature this turn do so.
//! Set: MRD #114 — Mirrodin | Scryfall ID: dca1eb7b-e964-4c36-8fc5-e1d2a98e841f | Oracle ID: b2aabfaf-5996-4b41-987a-6bd941f6ae88
use baylee_cards_dsl::prelude::*;

card!(
    index = index::BLOODSCENT,
    oracle_id = "b2aabfaf-5996-4b41-987a-6bd941f6ae88",
    scryfall_id = "dca1eb7b-e964-4c36-8fc5-e1d2a98e841f",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Bloodscent",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::INSTANT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[Effect::ModifyTargets {
            modifier: Modifier::MustBeBlockedByAllAble,
            duration: Duration::UntilEndOfTurn,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )],
);
