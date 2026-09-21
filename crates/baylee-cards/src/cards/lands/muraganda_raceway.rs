//! Muraganda Raceway — (no cost) — Land
//! Oracle: Start your engines! (If you have no speed, it starts at 1. It increases once on each of your turns when an opponent loses life. Max speed is 4.)
//! Oracle: {T}: Add {C}.
//! Oracle: Max speed — {T}: Add {C}{C}.
//! Set: DFT #257 — Aetherdrift | Scryfall ID: 5041ae16-29ff-4ad5-8a37-4736e9409294 | Oracle ID: b5fa5651-d714-44d6-867b-be0e3224b7ed
// PARTIAL — {T}: Add {C}, which is the whole of what the DSL can say here.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MURAGANDA_RACEWAY,
    oracle_id = "b5fa5651-d714-44d6-867b-be0e3224b7ed",
    scryfall_id = "5041ae16-29ff-4ad5-8a37-4736e9409294",
    faces = &[face!(name = "Muraganda Raceway", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "speed (CR 702.179) is not modelled — nothing sets it and no Condition reads it",
    ),
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)])],
);

// NOT SUPPORTED: "Start your engines! (If you have no speed, it starts at 1.
// …Max speed is 4.)" — no Effect sets a player's speed, and "speed" is not a
// keyword bit the engine reads.
// NOT SUPPORTED: "Max speed — {T}: Add {C}{C}." — no Condition asks what a
// player's speed is (ControlCount, OpponentGraveyardCountAtLeast,
// CountersOnSelf(Exactly) and SourceMatches all read something else), so the
// ability could only be written unconditional and would give {C}{C} to a
// player at speed 0. It comes off the card.
