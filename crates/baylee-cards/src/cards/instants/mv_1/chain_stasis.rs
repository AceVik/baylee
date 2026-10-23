//! Chain Stasis — {U} — Instant
//! Oracle: You may tap or untap target creature. Then that creature's controller may pay {2}{U}. If the player does, they may copy this spell and may choose a new target for that copy.
//! Set: HML #23 — Homelands | Scryfall ID: f14f0c52-67c2-4302-82bd-fbb4e3c6d4f4 | Oracle ID: 1ec365f8-bfb8-416f-b4db-868a8f5d042a
// IMPLEMENTED — you may tap or untap target creature; then that creature's controller may pay {2}{U} and, if they do, may copy this spell and choose a new target for the copy.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CHAIN_STASIS,
    oracle_id = "1ec365f8-bfb8-416f-b4db-868a8f5d042a",
    scryfall_id = "f14f0c52-67c2-4302-82bd-fbb4e3c6d4f4",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Chain Stasis",
        mana_cost = mana!("{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::MayDo {
                effects: &[Effect::ChooseOne {
                    choices: &[Effect::TapTarget, Effect::UntapTarget],
                }],
            },
            // "Then that creature's controller may pay {2}{U}. If the player
            // does, they may copy this spell and may choose a new target for
            // that copy." The creature's controller decides, pays and owns
            // the copy.
            Effect::PlayerMayPayManaThen {
                player: PlayerRel::TargetPlayerOrController,
                cost: mana!("{2}{U}"),
                effects: &[Effect::MayDoFor {
                    player: PlayerRel::TargetPlayerOrController,
                    effects: &[Effect::CopyResolvingSpell {
                        controller: PlayerRel::TargetPlayerOrController,
                    }],
                }],
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )],
);
