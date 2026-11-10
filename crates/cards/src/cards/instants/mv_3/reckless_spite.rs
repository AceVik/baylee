//! Reckless Spite — {1}{B}{B} — Instant
//! Oracle: Destroy two target nonblack creatures. You lose 5 life.
//! Set: C21 #151 — Commander 2021 | Scryfall ID: 2c39aec6-5186-4329-80a1-52778f1d4fea | Oracle ID: a684df3a-5441-4daa-86d1-c47a91b35e6a
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

static NONBLACK_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Not(&Filter::HasColor(ColorSet::from_slice(&[Color::Black]))),
]);

card!(
    index = index::RECKLESS_SPITE,
    oracle_id = "a684df3a-5441-4daa-86d1-c47a91b35e6a",
    scryfall_id = "2c39aec6-5186-4329-80a1-52778f1d4fea",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Reckless Spite",
        mana_cost = mana!("{1}{B}{B}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::destroy(TargetSpec::Object(&NONBLACK_CREATURE)),
            Effect::LoseLife {
                amount: Amount::Fixed(5),
                target: PlayerRel::You,
            },
        ],
        targets = Some(TargetReq::exactly(
            TargetSpec::Object(&NONBLACK_CREATURE),
            2,
        )),
    )],
);
