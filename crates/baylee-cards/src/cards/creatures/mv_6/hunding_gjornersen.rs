//! Hunding Gjornersen — {3}{W}{U}{U} — Legendary Creature — Human Warrior
//! Oracle: Rampage 1 (Whenever this creature becomes blocked, it gets +1/+1 until end of turn for each creature blocking it beyond the first.)
//! Set: ME3 #152 — Masters Edition III | Scryfall ID: 4049abac-cb54-4af2-b2df-ffb8fdc22e84 | Oracle ID: dd4f3a78-3167-42d0-8334-5ac36750a253
// PARTIAL — the whole card is rampage 1, which has no DSL vocabulary (see NOT SUPPORTED below).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HUNDING_GJORNERSEN,
    oracle_id = "dd4f3a78-3167-42d0-8334-5ac36750a253",
    scryfall_id = "4049abac-cb54-4af2-b2df-ffb8fdc22e84",
    color_identity = ColorSet::from_slice(&[Color::Blue, Color::White]),
    commander = CommanderRule::Legendary,
    coverage = Coverage::Partial(
        "rampage 1 has no DSL vocabulary: no rampage keyword bit, no \
         \"becomes blocked\" trigger, and no amount counting the blockers \
         beyond the first"
    ),
    faces = &[face!(
        name = "Hunding Gjornersen",
        mana_cost = mana!("{3}{W}{U}{U}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WARRIOR],
        power = Some(5),
        toughness = Some(4),
    ),],
    // NOT SUPPORTED: "Rampage 1 (Whenever this creature becomes blocked, it
    // gets +1/+1 until end of turn for each creature blocking it beyond the
    // first.)" — the DSL has no rampage keyword (`KeywordSet` carries only
    // text-independent bits, and rampage carries a number), no trigger for
    // "becomes blocked" (`Trigger::BlocksOrBecomesBlockedBy` also fires
    // when this creature blocks and takes a filter rather than a count),
    // and no `Amount` counts "each creature blocking it beyond the first".
    abilities = &[],
);
