//! Hellfire — {2}{B}{B}{B} — Sorcery
//! Oracle: Destroy all nonblack creatures. Hellfire deals X plus 3 damage to you, where X is the number of creatures that died this way.
//! Set: ME3 #70 — Masters Edition III | Scryfall ID: e66002e6-f722-429c-94a0-f571d9110fbd | Oracle ID: e7b1975d-9574-4333-8530-33167948078a
// PARTIAL — "Destroy all nonblack creatures" is built; the X-plus-3 damage
// to you has no amount that counts what this destruction killed.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HELLFIRE,
    oracle_id = "e7b1975d-9574-4333-8530-33167948078a",
    scryfall_id = "e66002e6-f722-429c-94a0-f571d9110fbd",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "\"Destroy all nonblack creatures\" is built; no Amount counts the \
         creatures this resolution destroyed, so the X-plus-3 damage to you \
         is left off"
    ),
    faces = &[face!(
        name = "Hellfire",
        mana_cost = mana!("{2}{B}{B}{B}"),
        types = TypeSet::SORCERY,
    ),],
    // NOT SUPPORTED: "Hellfire deals X plus 3 damage to you, where X is the
    // number of creatures that died this way." — no amount counts what a
    // resolution destroyed: `Amount::TargetsPutIntoGraveyard` counts the
    // resolving ability's targets and `Effect::DestroyAll` has none, while
    // `Amount::CreaturesDiedThisTurn` counts every creature that died this
    // turn, however it died.
    abilities = &[spell!(&[Effect::destroy_all(&Filter::And(&[
        Filter::CREATURE,
        Filter::Not(&Filter::HasColor(ColorSet::from_slice(&[Color::Black]))),
    ]))])],
);
