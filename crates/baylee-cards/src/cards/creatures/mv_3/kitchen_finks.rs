//! Kitchen Finks — {1}{G/W}{G/W} — Creature — Ouphe
//! Oracle: When this creature enters, you gain 2 life.
//! Oracle: Persist (When this creature dies, if it had no -1/-1 counters on it, return it to the battlefield under its owner's control with a -1/-1 counter on it.)
//! Set: UMA #216 — Ultimate Masters | Scryfall ID: a844d537-df73-422d-a154-f1c6b92d0469 | Oracle ID: 5470dcfa-4eff-43da-abf7-19922841f719
// PARTIAL — the enter trigger is built; persist is not expressible.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::KITCHEN_FINKS,
    oracle_id = "5470dcfa-4eff-43da-abf7-19922841f719",
    scryfall_id = "a844d537-df73-422d-a154-f1c6b92d0469",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    coverage = Coverage::Partial(
        "persist: no keyword bit, no 'at most zero -1/-1 counters' condition, and no effect that returns a card from a graveyard with a counter on it"
    ),
    faces = &[face!(
        name = "Kitchen Finks",
        mana_cost = mana!("{1}{G/W}{G/W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::OUPHE],
        power = Some(3),
        toughness = Some(2),
    ),],
    abilities = &[
        triggered!(Trigger::ETB, &[Effect::gain_life(2)]),
        // NOT SUPPORTED: Persist — "When this creature dies, if it had no
        // -1/-1 counters on it, return it to the battlefield under its
        // owner's control with a -1/-1 counter on it." No keyword bit or
        // synthetic engine trigger implements persist; `Condition` counts
        // counters on the source *at least* N and never at most N; and no
        // effect returns a card from a graveyard to the battlefield with a
        // counter on it (GraveyardToBattlefield takes a `TargetSpec` it
        // cannot put counters on, and EnterModifier has no counter variant).
    ],
);
