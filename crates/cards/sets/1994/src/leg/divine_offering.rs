//! Divine Offering — {1}{W} — Instant
//! Oracle: Destroy target artifact. You gain life equal to its mana value.
//! Set: MBS #5 — Mirrodin Besieged | Scryfall ID: fe7c7a65-5a96-4986-877b-b34583092bb6 | Oracle ID: 231f8edb-4ea1-44be-8794-b76a31462dfc
// IMPLEMENTED — destroys a target artifact, then you gain life equal to its
// mana value.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DIVINE_OFFERING,
    oracle_id = "231f8edb-4ea1-44be-8794-b76a31462dfc",
    scryfall_id = "fe7c7a65-5a96-4986-877b-b34583092bb6",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Divine Offering",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::INSTANT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[
            Effect::destroy(TargetSpec::Object(&Filter::ARTIFACT)),
            Effect::GainLife {
                amount: Amount::TargetCmc,
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::ARTIFACT)))
    )],
);
