//! Sanctum Guardian — {1}{W}{W} — Creature — Human Cleric
//! Oracle: Sacrifice this creature: The next time a source of your choice would deal damage to any target this turn, prevent that damage.
//! Set: USG #43 — Urza's Saga | Scryfall ID: 0d09137f-0f4c-4389-a915-0ce02c833d94 | Oracle ID: eb7fe094-2237-40c8-b831-93498c93b768
// PARTIAL: preventing the next damage from a chosen source to any target is not in Effect; sacrifices itself to prevent the next 1 damage that would be dealt to any target this turn.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SANCTUM_GUARDIAN,
    oracle_id = "eb7fe094-2237-40c8-b831-93498c93b768",
    scryfall_id = "0d09137f-0f4c-4389-a915-0ce02c833d94",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "preventing the next damage from a chosen source to any target is not in Effect; sacrifices itself to prevent the next 1 damage that would be dealt to any target this turn",
    ),
    faces = &[face!(
        name = "Sanctum Guardian",
        mana_cost = mana!("{1}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(1),
        toughness = Some(4),
    ),],
    abilities = &[activated!(
        cost!(SacrificeSelf),
        &[Effect::PreventNextDamage {
            target: TargetSpec::AnyTarget,
            amount: Amount::Fixed(1),
        }],
        target = Some(TargetSpec::AnyTarget),
    ),],
);
