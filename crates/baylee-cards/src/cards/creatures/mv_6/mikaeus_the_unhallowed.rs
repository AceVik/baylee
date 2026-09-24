//! Mikaeus, the Unhallowed — {3}{B}{B}{B} — Legendary Creature — Zombie Cleric
//! Oracle: Intimidate (This creature can't be blocked except by artifact creatures and/or creatures that share a color with it.)
//! Oracle: Whenever a Human deals damage to you, destroy it.
//! Oracle: Other non-Human creatures you control get +1/+1 and have undying. (When a creature with undying dies, if it had no +1/+1 counters on it, return it to the battlefield under its owner's control with a +1/+1 counter on it.)
//! Set: CMM #173 — Commander Masters | Scryfall ID: bc1f42a2-fe11-45da-9552-069803b4068a | Oracle ID: 5d27c63e-d1ef-48af-b51d-01ebc6daeac9
// PARTIAL — one static on "other non-Human creatures you control" (+1/+1).
// Intimidate, undying and the Human-damage trigger are all dropped:
// intimidate is a keyword bit no engine rule reads, and a granted undying is
// forgotten by the time the creature has died, so either would be a card
// that looks finished and changes nothing at the table.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// Named although one static is left pointing at it: the filter is the card's
// own sentence, and the undying half comes back here the day the undying
// look-back reads granted keywords.
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
        "intimidate is a keyword bit no combat rule reads; undying granted by a \
         static is lost at death, because the undying trigger's look-back reads \
         only printed keywords; and \"Whenever a Human deals damage to you, \
         destroy it\" has no trigger, Trigger::DealsCombatDamageToPlayer being \
         combat damage only and to any player",
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
        // NOT SUPPORTED: "... and have undying" — the engine reads undying,
        // but only a printed one: the look-back in `trigger.rs` asks the card
        // that died, not the keywords it had on the battlefield, so a
        // `Modifier::AddKeyword` grant is gone by the time it is asked, and
        // the creatures would die and stay dead while the card said otherwise.
        // NOT SUPPORTED: "Whenever a Human deals damage to you, destroy it." —
        // no Trigger for a source dealing (non-combat) damage to a player, so
        // the ability comes off rather than being approximated with the
        // combat-damage trigger, which would miss every Human that pings.
        static_ability!(OTHER_NON_HUMAN_CREATURES, Modifier::ModifyPT(1, 1)),
    ],
);
