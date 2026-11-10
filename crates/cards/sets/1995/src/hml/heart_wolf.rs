//! Heart Wolf — {3}{R} — Creature — Wolf
//! Oracle: First strike
//! Oracle: {T}: Target Dwarf creature gets +2/+0 and gains first strike until end of turn. When that creature leaves the battlefield this turn, sacrifice this creature. Activate only during combat.
//! Set: HML #75 — Homelands | Scryfall ID: e0427dcd-26da-462b-b936-a382d3d8afce | Oracle ID: 2f50c45f-058d-41fb-9965-08c2e0edc8ac
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static DWARF_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasSubtype(subtypes::creature::DWARF),
]);

card!(
    index = index::HEART_WOLF,
    oracle_id = "2f50c45f-058d-41fb-9965-08c2e0edc8ac",
    scryfall_id = "e0427dcd-26da-462b-b936-a382d3d8afce",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    keywords = KeywordSet::FIRST_STRIKE,
    faces = &[face!(
        name = "Heart Wolf",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WOLF],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[activated!(
        Cost::TAP,
        &[
            Effect::ModifyTargets {
                modifier: Modifier::ModifyPT(2, 0),
                duration: Duration::UntilEndOfTurn,
            },
            Effect::ModifyTargets {
                modifier: Modifier::AddKeyword(KeywordSet::FIRST_STRIKE),
                duration: Duration::UntilEndOfTurn,
            },
            Effect::WhenTargetLeavesBattlefieldThisTurn {
                effects: &[Effect::SacrificeSelf],
            },
        ],
        target = Some(TargetSpec::Object(&DWARF_CREATURE)),
        condition = Some(Condition::DuringCombat),
    )],
);
