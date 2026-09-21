//! Shaleskin Bruiser — {6}{R} — Creature — Beast
//! Oracle: Trample
//! Oracle: Whenever this creature attacks, it gets +3/+0 until end of turn for each other attacking Beast.
//! Set: ONS #226 — Onslaught | Scryfall ID: fc2de8a4-0d84-4f7c-bbe4-3a31172186ab | Oracle ID: b90e370a-5080-485e-a957-93d5f60e6cdb
// PARTIAL — trample is a keyword bit the engine reads; the attack trigger is
// left off the card (see NOT SUPPORTED above `card!`).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// NOT SUPPORTED: "it gets +3/+0 until end of turn for each other attacking
// Beast" — the size of the pump is a *count multiplied by three*, and no
// `Amount` multiplies a `CountOf`: `PumpFilter` would take it, but
// `Amount::CountOf` alone is +1/+0 per Beast and `Amount::DoubleX` doubles an
// announced X, not a count. (The per-count multiplier exists only as
// `Modifier::ModifyPTPerCount`, whose count is static — it would keep the
// bonus after combat and after the other Beasts die, which the printed
// "until end of turn" clause does not.)

card!(
    index = index::SHALESKIN_BRUISER,
    oracle_id = "b90e370a-5080-485e-a957-93d5f60e6cdb",
    scryfall_id = "fc2de8a4-0d84-4f7c-bbe4-3a31172186ab",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    keywords = KeywordSet::TRAMPLE,
    coverage = Coverage::Partial(
        "trample only: the attack trigger's +3/+0 per other attacking Beast needs an Amount that multiplies a count"
    ),
    faces = &[face!(
        name = "Shaleskin Bruiser",
        mana_cost = mana!("{6}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BEAST],
        power = Some(4),
        toughness = Some(4),
    ),],
);
