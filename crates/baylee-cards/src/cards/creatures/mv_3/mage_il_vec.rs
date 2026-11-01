//! Mage il-Vec — {2}{R} — Creature — Human Wizard
//! Oracle: {T}, Discard a card at random: This creature deals 1 damage to any target.
//! Set: TPR #140 — Tempest Remastered | Scryfall ID: 672b4f9a-84e4-4a27-8d03-11a63a701fec | Oracle ID: adea7db6-634c-4c5d-be40-264b4acffc53
// IMPLEMENTED — all printed abilities are represented.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MAGE_IL_VEC,
    oracle_id = "adea7db6-634c-4c5d-be40-264b4acffc53",
    scryfall_id = "672b4f9a-84e4-4a27-8d03-11a63a701fec",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Mage il-Vec",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[activated!(
        cost!(TapSelf, DiscardRandom),
        &[Effect::DealDamage {
            amount: Amount::Fixed(1),
            target: TargetSpec::AnyTarget
        }],
        target = Some(TargetSpec::AnyTarget)
    ),],
);
