//! Pit Scorpion — {2}{B} — Creature — Scorpion
//! Oracle: Whenever this creature deals damage to a player, that player gets a poison counter. (A player with ten or more poison counters loses the game.)
//! Set: 5ED #187 — Fifth Edition | Scryfall ID: fe106ff1-cdc6-44cc-adb7-131203a05292 | Oracle ID: 4804fc42-588c-487a-8ff7-bcc16749fc1f
// PARTIAL — the poison trigger is off the card, see the NOT SUPPORTED line below.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PIT_SCORPION,
    oracle_id = "4804fc42-588c-487a-8ff7-bcc16749fc1f",
    scryfall_id = "fe106ff1-cdc6-44cc-adb7-131203a05292",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "no Trigger hears this creature deal damage to any player (the \
         damage triggers name opponents or ignore the source) and no Effect \
         puts a poison counter on a player, so the ability is off the card"
    ),
    faces = &[face!(
        name = "Pit Scorpion",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SCORPION],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// NOT SUPPORTED: "Whenever this creature deals damage to a player, that
// player gets a poison counter. (A player with ten or more poison counters
// loses the game.)" — `Trigger::DealsDamageToOpponent` and
// `Trigger::DealsCombatDamageToOpponent` hear only opponents, while this
// sentence says any player and `Trigger::PlayerDealtDamage` ignores which
// source dealt the damage; and `Effect::AddCounter` puts counters on objects
// (`replacement::put_counters` takes an `ObjectId`), so nothing gives a
// player a poison counter. A `Toxic` ability is not a substitute: it is
// combat damage only and does not say "that player gets" a counter.
