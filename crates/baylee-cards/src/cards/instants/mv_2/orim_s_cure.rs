//! Orim's Cure — {1}{W} — Instant
//! Oracle: If you control a Plains, you may tap an untapped creature you control rather than pay this spell's mana cost.
//! Oracle: Prevent the next 4 damage that would be dealt to any target this turn.
//! Set: MMQ #33 — Mercadian Masques | Scryfall ID: 754ae359-363b-456a-bbca-52fbfbaa86b8 | Oracle ID: 61eacaab-625b-499c-8602-f37ad5dda138
// NOT SUPPORTED: conditional alternative cost (tap an untapped creature if you control a Plains) not supported
// PARTIAL: conditional alternative cost (tap an untapped creature if you control a Plains) not supported.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ORIM_S_CURE,
    oracle_id = "61eacaab-625b-499c-8602-f37ad5dda138",
    scryfall_id = "754ae359-363b-456a-bbca-52fbfbaa86b8",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "conditional alternative cost (tap an untapped creature if you control a Plains) not supported"
    ),
    faces = &[face!(
        name = "Orim's Cure",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::PreventNextDamage {
            target: TargetSpec::AnyTarget,
            amount: Amount::Fixed(4),
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyTarget)),
    ),],
);
