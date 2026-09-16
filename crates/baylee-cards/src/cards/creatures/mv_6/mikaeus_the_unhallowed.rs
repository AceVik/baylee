//! Mikaeus, the Unhallowed — {3}{B}{B}{B} — Legendary Creature — Zombie Cleric
//! Oracle: Intimidate (This creature can't be blocked except by artifact creatures and/or creatures that share a color with it.)
//! Oracle: Whenever a Human deals damage to you, destroy it.
//! Oracle: Other non-Human creatures you control get +1/+1 and have undying. (When a creature with undying dies, if it had no +1/+1 counters on it, return it to the battlefield under its owner's control with a +1/+1 counter on it.)
//! Set: CMM #173 — Commander Masters | Scryfall ID: bc1f42a2-fe11-45da-9552-069803b4068a | Oracle ID: 5d27c63e-d1ef-48af-b51d-01ebc6daeac9
// PARTIAL — one static on "other non-Human creatures you control" (+1/+1).
// Intimidate, undying and the Human-damage trigger are all dropped: the
// first two are keyword bits no engine rule reads, and a keyword nothing
// reads is a card that looks finished and changes nothing at the table.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// Named although one static is left pointing at it: the filter is the card's
// own sentence, and the undying half comes back here the day a rule reads it.
static OTHER_NON_HUMAN_CREATURES: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::ControlledByYou,
    Filter::Another,
    Filter::Not(&Filter::HasSubtype(subtypes::creature::HUMAN)),
]);

card!(
    index = index::MIKAEUS_THE_UNHALLOWED,
    oracle_id = "5d27c63e-d1ef-48af-b51d-01ebc6daeac9",
    scryfall_id = "bc1f42a2-fe11-45da-9552-069803b4068a",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    commander = CommanderRule::Legendary,
    coverage = Coverage::Partial(
        "intimidate and undying are keyword bits no engine rule reads; and \
         \"Whenever a Human deals damage to you, destroy it\" is dropped, the \
         DSL's only damage trigger being Trigger::DealsCombatDamageToPlayer, \
         which is combat damage alone and names no player to be damaged",
    ),
    faces = &[face!(
        name = "Mikaeus, the Unhallowed",
        mana_cost = mana!("{3}{B}{B}{B}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::ZOMBIE, subtypes::creature::CLERIC],
        power = Some(5),
        toughness = Some(5),
    ),],
    abilities = &[
        // NOT SUPPORTED: "Intimidate" — the bit exists in `KeywordSet` and no
        // rule in `combat` reads it, so claiming it would print a keyword the
        // table never applies (`keyword_tests::ENFORCED` is the list).
        // NOT SUPPORTED: "... and have undying" — the same, one layer further
        // out: `Modifier::AddKeyword` would grant a bit nothing reads, so the
        // creatures would die and stay dead while the card said otherwise.
        // NOT SUPPORTED: "Whenever a Human deals damage to you, destroy it." —
        // no Trigger for a source dealing (non-combat) damage to a player, so
        // the ability comes off rather than being approximated with the
        // combat-damage trigger, which would miss every Human that pings.
        static_ability!(OTHER_NON_HUMAN_CREATURES, Modifier::ModifyPT(1, 1)),
    ],
);
