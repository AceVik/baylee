//! Pirate Ship — {4}{U} — Creature — Human Pirate
//! Oracle: This creature can't attack unless defending player controls an Island.
//! Oracle: {T}: This creature deals 1 damage to any target.
//! Oracle: When you control no Islands, sacrifice this creature.
//! Set: TSB #28 — Time Spiral Timeshifted | Scryfall ID: 4f91724c-0e3b-4289-9af6-9de489d811eb | Oracle ID: c6b3f924-806d-47d3-b044-72b48470196c
// PARTIAL — the attack restriction and the sacrifice when you control no Islands
// are not in the engine; it deals its 1 damage.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PIRATE_SHIP,
    oracle_id = "c6b3f924-806d-47d3-b044-72b48470196c",
    scryfall_id = "4f91724c-0e3b-4289-9af6-9de489d811eb",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "the attack restriction and the sacrifice when you control no Islands are not in the engine; it deals its 1 damage"
    ),
    faces = &[face!(
        name = "Pirate Ship",
        mana_cost = mana!("{4}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::PIRATE],
        power = Some(4),
        toughness = Some(3),
    ),],
    abilities = &[
        activated!(
            Cost::TAP,
            &[Effect::DealDamage {
                amount: Amount::Fixed(1),
                target: TargetSpec::AnyTarget
            }],
            target = Some(TargetSpec::AnyTarget)
        ),
        // NOT SUPPORTED: This creature can't attack unless defending player controls an
        // Island.
        // NOT SUPPORTED: When you control no Islands, sacrifice this creature.
    ],
);
