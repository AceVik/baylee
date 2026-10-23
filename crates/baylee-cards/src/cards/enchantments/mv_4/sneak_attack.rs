//! Sneak Attack — {3}{R} — Enchantment
//! Oracle: {R}: You may put a creature card from your hand onto the battlefield. That creature gains haste. Sacrifice the creature at the beginning of the next end step.
//! Set: USG #218 — Urza's Saga | Scryfall ID: d07dc95d-82a8-4a58-8ea2-d4513bd7316d | Oracle ID: e7aee670-e6ac-4b3a-b3d1-7b7013db8f3d
// PARTIAL: granting haste and sacrificing at the beginning of the next end step to the creature put from hand onto the battlefield is not in PutFromHandOntoBattlefield; activates {R} to optionally put a creature card from your hand onto the battlefield.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SNEAK_ATTACK,
    oracle_id = "e7aee670-e6ac-4b3a-b3d1-7b7013db8f3d",
    scryfall_id = "d07dc95d-82a8-4a58-8ea2-d4513bd7316d",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "granting haste and sacrificing at the beginning of the next end step to the creature put from hand onto the battlefield is not in PutFromHandOntoBattlefield; activates {R} to optionally put a creature card from your hand onto the battlefield",
    ),
    faces = &[face!(
        name = "Sneak Attack",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[activated!(
        cost!("{R}"),
        &[Effect::PutFromHandOntoBattlefield {
            tapped: false,
            filter: &Filter::CREATURE,
            mana_value: None,
            optional: true,
        }],
    ),],
);
