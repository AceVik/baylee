//! Meteor Shower — {X}{X}{R} — Sorcery
//! Oracle: Meteor Shower deals X plus 1 damage divided as you choose among any number of targets.
//! Set: ME2 #135 — Masters Edition II | Scryfall ID: d2092ea9-f840-4fbf-9671-5a2858476a97 | Oracle ID: d2110083-b30f-43f1-9352-7536f5e19d3c
// NOT SUPPORTED: dividing the X plus 1 damage among multiple targets is not supported because DivisionAmount has no X+1 variant; deals X plus 1 damage to a single target.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::METEOR_SHOWER,
    oracle_id = "d2110083-b30f-43f1-9352-7536f5e19d3c",
    scryfall_id = "d2092ea9-f840-4fbf-9671-5a2858476a97",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "dividing the X plus 1 damage among multiple targets is not supported because DivisionAmount has no X+1 variant; deals X plus 1 damage to a single target",
    ),
    faces = &[face!(
        name = "Meteor Shower",
        mana_cost = mana!("{X}{X}{R}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::DealDamage {
            amount: Amount::Plus {
                base: &Amount::X,
                offset: 1,
            },
            target: TargetSpec::AnyTarget,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyTarget)),
    )],
);
