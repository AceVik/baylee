//! Aerathi Berserker — {2}{R}{R}{R} — Creature — Human Berserker
//! Oracle: Rampage 3 (Whenever this creature becomes blocked, it gets +3/+3 until end of turn for each creature blocking it beyond the first.)
//! Set: LEG #131 — Legends | Scryfall ID: 06673800-22a7-4ee3-92fa-7c7cd4865d30 | Oracle ID: 5cb495a2-c683-4066-b6ee-d0b7d8843cb9
// PARTIAL — rampage 3 is off the card (see NOT SUPPORTED below).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// NOT SUPPORTED: "Rampage 3 (Whenever this creature becomes blocked, it
// gets +3/+3 until end of turn for each creature blocking it beyond the
// first.)" — the DSL has no rampage keyword (`KeywordSet` carries only
// text-independent bits, and rampage carries a number), and no `Amount`
// counts "each creature blocking it beyond the first": `Filter::Blocking`
// can say that the event's other creature is a blocker, but nothing counts
// the blockers of this creature, let alone with the first one subtracted.

card!(
    index = index::AERATHI_BERSERKER,
    oracle_id = "5cb495a2-c683-4066-b6ee-d0b7d8843cb9",
    scryfall_id = "06673800-22a7-4ee3-92fa-7c7cd4865d30",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "rampage 3 has no DSL vocabulary: no rampage keyword bit and no \
         amount counting the creatures blocking this creature beyond the \
         first"
    ),
    faces = &[face!(
        name = "Aerathi Berserker",
        mana_cost = mana!("{2}{R}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::BERSERKER],
        power = Some(2),
        toughness = Some(4),
    ),],
);
