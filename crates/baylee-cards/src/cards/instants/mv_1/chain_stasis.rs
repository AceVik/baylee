//! Chain Stasis — {U} — Instant
//! Oracle: You may tap or untap target creature. Then that creature's controller may pay {2}{U}. If the player does, they may copy this spell and may choose a new target for that copy.
//! Set: HML #23 — Homelands | Scryfall ID: f14f0c52-67c2-4302-82bd-fbb4e3c6d4f4 | Oracle ID: 1ec365f8-bfb8-416f-b4db-868a8f5d042a
// PARTIAL — target controller paying {2}{U} to copy this spell on resolution is not in Effect; may tap or untap target creature.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CHAIN_STASIS,
    oracle_id = "1ec365f8-bfb8-416f-b4db-868a8f5d042a",
    scryfall_id = "f14f0c52-67c2-4302-82bd-fbb4e3c6d4f4",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "target controller paying {2}{U} to copy this spell on resolution not in Effect; may tap or untap target creature"
    ),
    faces = &[face!(
        name = "Chain Stasis",
        mana_cost = mana!("{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::MayDo {
            effects: &[Effect::ChooseOne {
                choices: &[Effect::TapTarget, Effect::UntapTarget],
            }],
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )],
);
