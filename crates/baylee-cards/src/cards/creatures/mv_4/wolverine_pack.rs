//! Wolverine Pack — {2}{G}{G} — Creature — Wolverine
//! Oracle: Rampage 2 (Whenever this creature becomes blocked, it gets +2/+2 until end of turn for each creature blocking it beyond the first.)
//! Set: 5ED #344 — Fifth Edition | Scryfall ID: fab6d1e8-0985-4560-aea2-7ad1925a2f5a | Oracle ID: d67d9d82-2786-48fc-8107-11d12c5813d0
// PARTIAL — rampage 2 is off the card (see NOT SUPPORTED below).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// NOT SUPPORTED: "Rampage 2 (Whenever this creature becomes blocked, it
// gets +2/+2 until end of turn for each creature blocking it beyond the
// first.)" — the DSL has no rampage keyword (`KeywordSet` carries only
// text-independent bits, and rampage carries a number), and no `Amount`
// counts "each creature blocking it beyond the first": `Filter::Blocking`
// can say that the event's other creature is a blocker, but nothing counts
// the blockers of this creature, let alone with the first one subtracted.

card!(
    index = index::WOLVERINE_PACK,
    oracle_id = "d67d9d82-2786-48fc-8107-11d12c5813d0",
    scryfall_id = "fab6d1e8-0985-4560-aea2-7ad1925a2f5a",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "rampage 2 has no DSL vocabulary: no rampage keyword bit and no \
         amount counting the creatures blocking this creature beyond the \
         first"
    ),
    faces = &[face!(
        name = "Wolverine Pack",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WOLVERINE],
        power = Some(2),
        toughness = Some(4),
    ),],
);
