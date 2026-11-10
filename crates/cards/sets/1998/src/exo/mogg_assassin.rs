//! Mogg Assassin — {2}{R} — Creature — Goblin Assassin
//! Oracle: {T}: You choose target creature an opponent controls, and that opponent chooses target creature. Flip a coin. If you win the flip, destroy the creature you chose. If you lose the flip, destroy the creature your opponent chose.
//! Set: EXO #88 — Exodus | Scryfall ID: 1124725d-e643-43a1-873e-255636c7f334 | Oracle ID: 36a6925a-621e-47d6-8cb3-1b2886e1986e
// PARTIAL — The opponent choosing a second target and destruction on a lost flip are unsupported; the entire activated ability is omitted.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MOGG_ASSASSIN,
    oracle_id = "36a6925a-621e-47d6-8cb3-1b2886e1986e",
    scryfall_id = "1124725d-e643-43a1-873e-255636c7f334",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "The opponent choosing a second target and destruction on a lost flip are unsupported; the entire activated ability is omitted."
    ),
    faces = &[face!(
        name = "Mogg Assassin",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN, subtypes::creature::ASSASSIN],
        power = Some(2),
        toughness = Some(1),
    ),],
    // NOT SUPPORTED: The opponent choosing a second target and destruction on a lost flip are unsupported; the entire activated ability is omitted.
    abilities = &[],
);
