//! Young Wolf — {G} — Creature — Wolf
//! Oracle: Undying (When this creature dies, if it had no +1/+1 counters on it, return it to the battlefield under its owner's control with a +1/+1 counter on it.)
//! Set: INR #227 — Innistrad Remastered | Scryfall ID: ed2ca825-b029-495f-83fc-54366229d417 | Oracle ID: 8b492764-10b6-4506-be11-22daa9220a91
// PARTIAL — the 1/1 Wolf body only: undying is a death trigger/replacement the
// DSL has no variant for.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::YOUNG_WOLF,
    oracle_id = "8b492764-10b6-4506-be11-22daa9220a91",
    scryfall_id = "ed2ca825-b029-495f-83fc-54366229d417",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "undying: nothing returns the card from the graveyard with a +1/+1 counter",
    ),
    faces = &[face!(
        name = "Young Wolf",
        mana_cost = mana!("{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WOLF],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// NOT SUPPORTED: Undying — "When this creature dies, if it had no +1/+1
// counters on it, return it to the battlefield under its owner's control with
// a +1/+1 counter on it." No keyword bit carries it, and the DSL has no
// mechanism to return the source card from the graveyard with a +1/+1 counter.
