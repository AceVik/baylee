//! Crystal Shard — {3} — Artifact
//! Oracle: {3}, {T} or {U}, {T}: Return target creature to its owner's hand unless its controller pays {1}.
//! Set: MRD #159 — Mirrodin | Scryfall ID: b1c1d05b-92be-40d7-859f-75293a531a84 | Oracle ID: cd8ccb10-a2c8-4a84-aa41-6ce6018f9bb7
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CRYSTAL_SHARD,
    oracle_id = "cd8ccb10-a2c8-4a84-aa41-6ce6018f9bb7",
    scryfall_id = "b1c1d05b-92be-40d7-859f-75293a531a84",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Crystal Shard",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[
        activated!(
            cost!("{3}", TapSelf),
            &[Effect::PlayerMayPayOr {
                player: PlayerRel::ControllerOfTarget,
                mana: Amount::Fixed(1),
                effect: &Effect::bounce(TargetSpec::Object(&Filter::CREATURE)),
            }],
            target = Some(TargetSpec::Object(&Filter::CREATURE)),
        ),
        activated!(
            cost!("{U}", TapSelf),
            &[Effect::PlayerMayPayOr {
                player: PlayerRel::ControllerOfTarget,
                mana: Amount::Fixed(1),
                effect: &Effect::bounce(TargetSpec::Object(&Filter::CREATURE)),
            }],
            target = Some(TargetSpec::Object(&Filter::CREATURE)),
        ),
    ],
);
