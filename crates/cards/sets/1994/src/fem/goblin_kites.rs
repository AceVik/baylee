//! Goblin Kites — {1}{R} — Enchantment
//! Oracle: {R}: Target creature you control with toughness 2 or less gains flying until end of turn. Flip a coin at the beginning of the next end step. If you lose the flip, sacrifice that creature.
//! Set: FEM #57 — Fallen Empires | Scryfall ID: a0a27ac3-2273-469a-92ba-3f4a3d55de6f | Oracle ID: 6efd11c8-be8a-4d44-a776-66d058638ad1
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

const YOUR_CREATURE_TOUGHNESS_AT_MOST_2: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::ControlledByYou,
    Filter::ToughnessAtMost(2),
]);

card!(
    index = index::GOBLIN_KITES,
    oracle_id = "6efd11c8-be8a-4d44-a776-66d058638ad1",
    scryfall_id = "a0a27ac3-2273-469a-92ba-3f4a3d55de6f",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Goblin Kites",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[activated!(
        cost!("{R}"),
        &[
            Effect::ModifyTargets {
                modifier: Modifier::AddKeyword(KeywordSet::FLYING),
                duration: Duration::UntilEndOfTurn,
            },
            Effect::AtNextEndStep {
                effects: &[Effect::FlipCoin {
                    won: &[],
                    lost: &[Effect::SacrificeObject {
                        target: TargetSpec::EventObject,
                    }],
                }],
            },
        ],
        target = Some(TargetSpec::Object(&YOUR_CREATURE_TOUGHNESS_AT_MOST_2)),
    ),],
);
