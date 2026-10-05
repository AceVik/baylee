//! Frost Giant — {3}{R}{R}{R} — Creature — Giant
//! Oracle: Rampage 2 (Whenever this creature becomes blocked, it gets +2/+2 until end of turn for each creature blocking it beyond the first.)
//! Set: ME3 #101 — Masters Edition III | Scryfall ID: a742bfa4-3730-4bfd-865c-92fb4b40f2f2 | Oracle ID: 56ed6af3-8f85-4907-9364-eca71f21bb11
// PARTIAL — the whole card is rampage 2, which has no DSL vocabulary (see NOT SUPPORTED below).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FROST_GIANT,
    oracle_id = "56ed6af3-8f85-4907-9364-eca71f21bb11",
    scryfall_id = "a742bfa4-3730-4bfd-865c-92fb4b40f2f2",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "rampage 2 has no DSL vocabulary: no rampage keyword bit, no \
         \"becomes blocked\" trigger, and no amount counting the blockers \
         beyond the first"
    ),
    faces = &[face!(
        name = "Frost Giant",
        mana_cost = mana!("{3}{R}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GIANT],
        power = Some(4),
        toughness = Some(4),
    ),],
    // NOT SUPPORTED: "Rampage 2 (Whenever this creature becomes blocked, it
    // gets +2/+2 until end of turn for each creature blocking it beyond the
    // first.)" — the DSL has no rampage keyword (`KeywordSet` carries only
    // text-independent bits, and rampage carries a number), no trigger for
    // "becomes blocked" (`Trigger::BlocksOrBecomesBlockedBy` also fires
    // when this creature blocks and takes a filter rather than a count),
    // and no `Amount` counts "each creature blocking it beyond the first".
    abilities = &[],
);
