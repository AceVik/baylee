//! Blazing Effigy — {1}{R} — Creature — Elemental
//! Oracle: When this creature dies, it deals X damage to target creature, where X is 3 plus the amount of damage dealt to this creature this turn by other sources named Blazing Effigy.
//! Set: LEG #134 — Legends | Scryfall ID: 921011ff-1696-4575-9198-abe993a0ee7a | Oracle ID: 7879b37c-5e2a-4945-9378-47744b715a6c
// PARTIAL — the whole ability is off the card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BLAZING_EFFIGY,
    oracle_id = "7879b37c-5e2a-4945-9378-47744b715a6c",
    scryfall_id = "921011ff-1696-4575-9198-abe993a0ee7a",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Blazing Effigy",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELEMENTAL],
        power = Some(0),
        toughness = Some(3),
    ),],
    coverage = Coverage::Partial(
        "no Amount reads the damage dealt to a permanent this turn, and none \
         sums that damage over sources by name"
    ),
    // NOT SUPPORTED: "When this creature dies, it deals X damage to target
    // creature, where X is 3 plus the amount of damage dealt to this creature
    // this turn by other sources named Blazing Effigy." — the dies trigger
    // (`Trigger::Dies(&Filter::This)`) and the damage
    // (`Effect::DealDamage` at a target creature) are writable, but X is not:
    // the engine's per-turn damage tally is per player
    // (`Amount::DamageDealtToYouThisTurn`), the per-permanent damage history
    // serves the `DiesAfterDamageByThis` trigger and no amount reads it, and
    // nothing scopes it to sources named Blazing Effigy. `Amount::Fixed(3)`
    // would ship a strictly weaker card, so the ability comes off instead.
);
