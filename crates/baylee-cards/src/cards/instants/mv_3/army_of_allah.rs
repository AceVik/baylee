//! Army of Allah — {1}{W}{W} — Instant
//! Oracle: Attacking creatures get +2/+0 until end of turn.
//! Set: ARN #2 — Arabian Nights | Scryfall ID: 3d170015-b125-49a6-a15e-8fd116bbcb14 | Oracle ID: 3483946d-8645-4c22-b0ba-a65a44456324
// IMPLEMENTED — one untargeted `PumpFilter` over every attacking creature
// gives +2/+0 until end of turn.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ARMY_OF_ALLAH,
    oracle_id = "3483946d-8645-4c22-b0ba-a65a44456324",
    scryfall_id = "3d170015-b125-49a6-a15e-8fd116bbcb14",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Army of Allah",
        mana_cost = mana!("{1}{W}{W}"),
        types = TypeSet::INSTANT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[spell!(&[Effect::PumpFilter {
        filter: &Filter::ATTACKING_CREATURE,
        controlled_by: None,
        power: Amount::Fixed(2),
        toughness: Amount::Fixed(0),
        keywords: KeywordSet::EMPTY,
        duration: Duration::UntilEndOfTurn,
    }])],
);
