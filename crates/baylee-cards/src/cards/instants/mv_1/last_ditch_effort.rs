//! Last-Ditch Effort — {R} — Instant
//! Oracle: Sacrifice any number of creatures. Last-Ditch Effort deals that much damage to any target.
//! Set: ULG #83 — Urza's Legacy | Scryfall ID: 295f7fe0-0681-4b25-807f-30ed70ec78d5 | Oracle ID: cdd8d7cf-28f2-4a4d-a373-40d48bff615b
// PARTIAL

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LAST_DITCH_EFFORT,
    oracle_id = "cdd8d7cf-28f2-4a4d-a373-40d48bff615b",
    scryfall_id = "295f7fe0-0681-4b25-807f-30ed70ec78d5",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "Sacrifices all creatures you control and deals damage equal to the number of creatures that died this way to any target; choosing any number of creatures to sacrifice during resolution is not yet in Effect.",
    ),
    faces = &[face!(
        name = "Last-Ditch Effort",
        mana_cost = mana!("{R}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::SacrificeAll {
                filter: &Filter::YOUR_CREATURE,
            },
            Effect::DealDamage {
                amount: Amount::ThisWay(ThisWay::Died(&Filter::CREATURE)),
                target: TargetSpec::AnyTarget,
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::AnyTarget)),
    )],
);
