//! Argivian Blacksmith — {1}{W}{W} — Creature — Human Artificer
//! Oracle: {T}: Prevent the next 2 damage that would be dealt to target artifact creature this turn.
//! Set: ME4 #4 — Masters Edition IV | Scryfall ID: f1fb4d0b-fa3f-4794-9285-89ddb9ac21c3 | Oracle ID: 80240b6b-d20d-4dfb-a2c5-c272c3b43a70
// IMPLEMENTED — {T}: a two-point prevention shield on a target artifact
// creature.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ARTIFACT_CREATURE: Filter = Filter::And(&[Filter::ARTIFACT, Filter::CREATURE]);

card!(
    index = index::ARGIVIAN_BLACKSMITH,
    oracle_id = "80240b6b-d20d-4dfb-a2c5-c272c3b43a70",
    scryfall_id = "f1fb4d0b-fa3f-4794-9285-89ddb9ac21c3",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Argivian Blacksmith",
        mana_cost = mana!("{1}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::ARTIFICER],
        power = Some(2),
        toughness = Some(2),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[activated!(
        Cost::TAP,
        &[Effect::PreventNextDamage {
            target: TargetSpec::Object(&ARTIFACT_CREATURE),
            amount: Amount::Fixed(2)
        }],
        target = Some(TargetSpec::Object(&ARTIFACT_CREATURE))
    )],
);
