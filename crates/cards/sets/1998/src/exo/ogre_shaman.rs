//! Ogre Shaman — {3}{R}{R} — Creature — Ogre Shaman
//! Oracle: {2}, Discard a card at random: This creature deals 2 damage to any target.
//! Set: TPR #148 — Tempest Remastered | Scryfall ID: e5e11a62-7885-4494-b128-938485dea8a5 | Oracle ID: 04c7bb20-6e40-4e79-a0ec-ced920d3491e
// IMPLEMENTED — all printed abilities are represented.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::OGRE_SHAMAN,
    oracle_id = "04c7bb20-6e40-4e79-a0ec-ced920d3491e",
    scryfall_id = "e5e11a62-7885-4494-b128-938485dea8a5",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Ogre Shaman",
        mana_cost = mana!("{3}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::OGRE, subtypes::creature::SHAMAN],
        power = Some(3),
        toughness = Some(3),
    ),],
    abilities = &[activated!(
        cost!("{2}", DiscardRandom),
        &[Effect::DealDamage {
            amount: Amount::Fixed(2),
            target: TargetSpec::AnyTarget
        }],
        target = Some(TargetSpec::AnyTarget)
    ),],
);
