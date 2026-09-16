//! Ragavan, Nimble Pilferer — {R} — Legendary Creature — Monkey Pirate
//! Oracle: Whenever Ragavan deals combat damage to a player, create a Treasure token and exile the top card of that player's library. Until end of turn, you may cast that card.
//! Oracle: Dash {1}{R} (You may cast this spell for its dash cost. If you do, it gains haste, and it's returned from the battlefield to its owner's hand at the beginning of the next end step.)
//! Set: MH2 #138 — Modern Horizons 2 | Scryfall ID: a9738cda-adb1-47fb-9f4c-ecd930228c4d | Oracle ID: 37108cd4-bbab-4ce3-9ed6-f60e8422e703
// PARTIAL — the combat-damage trigger is built as far as the DSL reaches
// (the Treasure token); the impulse half and dash are named below.

use crate::tokens::TREASURE;
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RAGAVAN_NIMBLE_PILFERER,
    oracle_id = "37108cd4-bbab-4ce3-9ed6-f60e8422e703",
    scryfall_id = "a9738cda-adb1-47fb-9f4c-ecd930228c4d",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    commander = CommanderRule::Legendary,
    coverage = Coverage::Partial(
        "the combat-damage trigger makes the Treasure only — no Effect exiles the top card of \
         a library or grants a cast permission on one, and AlternativeCost carries no dash rider"
    ),
    faces = &[face!(
        name = "Ragavan, Nimble Pilferer",
        mana_cost = mana!("{R}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::MONKEY, subtypes::creature::PIRATE],
        power = Some(2),
        toughness = Some(1),
    ),],
    abilities = &[
        // NOT SUPPORTED: "exile the top card of that player's library. Until
        // end of turn, you may cast that card" — nothing in `Effect` exiles a
        // card off the top of a library (`LookAtTopPick` puts it into hand,
        // `Mill` into the graveyard) and nothing grants a cast permission on
        // a card in exile, so the trigger makes the Treasure and stops.
        triggered!(
            Trigger::DealsCombatDamageToPlayer(&Filter::This),
            &[Effect::CreateToken { token: &TREASURE }]
        ),
        // NOT SUPPORTED: "Dash {1}{R} (You may cast this spell for its dash
        // cost. If you do, it gains haste, and it's returned from the
        // battlefield to its owner's hand at the beginning of the next end
        // step.)" — `AlternativeCost` carries a cost and a condition and no
        // rider, so writing it would be a {1}{R} cast with no haste and no
        // return to hand.
    ],
);
