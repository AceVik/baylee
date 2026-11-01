//! Jolt — {2}{U} — Instant
//! Oracle: You may tap or untap target artifact, creature, or land.
//! Oracle: Draw a card at the beginning of the next turn's upkeep.
//! Set: MIR #70 — Mirage | Scryfall ID: 3e0c085a-e17d-4003-bb58-f97555365fcf | Oracle ID: 13f69c70-dc93-4983-b649-d91b22633eb9
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

static TARGET1: Filter = Filter::Or(&[Filter::ARTIFACT, Filter::CREATURE, Filter::LAND]);

card!(
    index = index::JOLT,
    oracle_id = "13f69c70-dc93-4983-b649-d91b22633eb9",
    scryfall_id = "3e0c085a-e17d-4003-bb58-f97555365fcf",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Jolt",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::MayDo {
                effects: &[Effect::ToggleTapTarget],
            },
            Effect::AtNextTurnsUpkeep {
                effects: &[Effect::draw(1)],
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&TARGET1))),
    )],
);
