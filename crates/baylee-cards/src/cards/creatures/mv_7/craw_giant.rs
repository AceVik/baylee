//! Craw Giant — {3}{G}{G}{G}{G} — Creature — Giant
//! Oracle: Trample
//! Oracle: Rampage 2 (Whenever this creature becomes blocked, it gets +2/+2 until end of turn for each creature blocking it beyond the first.)
//! Set: TSB #76 — Time Spiral Timeshifted | Scryfall ID: 81c5ee8f-a060-486c-a7f1-7bd600f6911a | Oracle ID: 152b621e-4c18-438c-9999-77e2685fda79
// PARTIAL — trample is written; rampage 2 is off the card (see NOT SUPPORTED below).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// NOT SUPPORTED: "Rampage 2 (Whenever this creature becomes blocked, it
// gets +2/+2 until end of turn for each creature blocking it beyond the
// first.)" — the DSL has no rampage keyword (`KeywordSet` carries only
// text-independent bits, and rampage carries a number), no trigger for
// "becomes blocked" (`Trigger::BlocksOrBecomesBlockedBy` also fires when
// this creature blocks and takes a filter rather than a count), and no
// `Amount` counts "each creature blocking it beyond the first".

card!(
    index = index::CRAW_GIANT,
    oracle_id = "152b621e-4c18-438c-9999-77e2685fda79",
    scryfall_id = "81c5ee8f-a060-486c-a7f1-7bd600f6911a",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    keywords = KeywordSet::TRAMPLE,
    coverage = Coverage::Partial(
        "rampage 2 has no DSL vocabulary: no rampage keyword bit, no \
         \"becomes blocked\" trigger, and no amount counting the blockers \
         beyond the first"
    ),
    faces = &[face!(
        name = "Craw Giant",
        mana_cost = mana!("{3}{G}{G}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GIANT],
        power = Some(6),
        toughness = Some(4),
    ),],
);
