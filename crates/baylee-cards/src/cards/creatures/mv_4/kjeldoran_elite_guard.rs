//! Kjeldoran Elite Guard — {3}{W} — Creature — Human Soldier
//! Oracle: {T}: Target creature gets +2/+2 until end of turn. When that creature leaves the battlefield this turn, sacrifice this creature. Activate only during combat.
//! Set: ME2 #21 — Masters Edition II | Scryfall ID: 0121644b-9d5e-4f0e-bc39-6532028f7480 | Oracle ID: a1f6df2a-fc97-44d4-a0d1-902d8df94002

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::KJELDORAN_ELITE_GUARD,
    oracle_id = "a1f6df2a-fc97-44d4-a0d1-902d8df94002",
    scryfall_id = "0121644b-9d5e-4f0e-bc39-6532028f7480",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Kjeldoran Elite Guard",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SOLDIER],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[activated!(
        Cost::TAP,
        &[
            Effect::PumpTarget {
                power: Amount::Fixed(2),
                toughness: Amount::Fixed(2),
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn,
            },
            Effect::WhenTargetLeavesBattlefieldThisTurn {
                effects: &[Effect::SacrificeSelf],
            },
        ],
        target = Some(TargetSpec::Object(&Filter::CREATURE)),
        condition = Some(Condition::DuringCombat),
    )],
);
