//! Heartwood Giant — {3}{G}{G} — Creature — Giant
//! Oracle: {T}, Sacrifice a Forest: This creature deals 2 damage to target player or planeswalker.
//! Set: TPR #174 — Tempest Remastered | Scryfall ID: d7ba2884-128b-4bfb-9a6e-cb81ff4ac00d | Oracle ID: b7d7418d-222b-4ae0-b0d9-9ea71d6ad800
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const YOUR_FOREST: Filter = Filter::And(&[
    Filter::ControlledByYou,
    Filter::HasSubtype(subtypes::land::FOREST),
]);

card!(
    index = index::HEARTWOOD_GIANT,
    oracle_id = "b7d7418d-222b-4ae0-b0d9-9ea71d6ad800",
    scryfall_id = "d7ba2884-128b-4bfb-9a6e-cb81ff4ac00d",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Heartwood Giant",
        mana_cost = mana!("{3}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GIANT],
        power = Some(4),
        toughness = Some(4),
    ),],
    abilities = &[activated!(
        cost!(TapSelf, Sacrifice(&YOUR_FOREST)),
        &[Effect::DealDamage {
            target: TargetSpec::PlayerOrObject(&Filter::PLANESWALKER),
            amount: Amount::Fixed(2),
        }],
        target = Some(TargetSpec::PlayerOrObject(&Filter::PLANESWALKER)),
    )],
);
