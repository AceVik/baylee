//! Serra Avenger — {W}{W} — Creature — Angel
//! Oracle: You can't cast Serra Avenger during your first, second, or third turns of the game.
//! Oracle: Flying, vigilance
//! Set: TSR #41 — Time Spiral Remastered | Scryfall ID: 8e5627de-fda7-4e39-a6ec-1e895bc353ed | Oracle ID: b91b65b3-b35c-460e-a49b-333000a9a519
// IMPLEMENTED — early-turn casting restriction, flying, and vigilance.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static AFTER_THIRD_TURN: Condition = Condition::Not(&Condition::DuringYourFirstTurns(3));

card!(
    index = index::SERRA_AVENGER,
    oracle_id = "b91b65b3-b35c-460e-a49b-333000a9a519",
    scryfall_id = "8e5627de-fda7-4e39-a6ec-1e895bc353ed",
    color_identity = ColorSet::from_slice(&[Color::White]),
    keywords = KeywordSet::FLYING.union(KeywordSet::VIGILANCE),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Serra Avenger",
        mana_cost = mana!("{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ANGEL],
        power = Some(3),
        toughness = Some(3),
    ),],
    abilities = &[spell!(&[], condition = Some(AFTER_THIRD_TURN))],
);
