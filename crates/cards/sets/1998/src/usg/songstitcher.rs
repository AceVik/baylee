//! Songstitcher — {W} — Creature — Human Cleric
//! Oracle: {1}{W}: Prevent all combat damage that would be dealt this turn by target attacking creature with flying.
//! Set: USG #52 — Urza's Saga | Scryfall ID: fa8d306f-3f4e-4c21-9461-caa3daf4fc50 | Oracle ID: eec72bcf-ccbc-43fc-8bdd-7bf4faa1fad7
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ATTACKING_FLYER: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Attacking,
    Filter::HasKeyword(KeywordSet::FLYING),
]);

card!(
    index = index::SONGSTITCHER,
    oracle_id = "eec72bcf-ccbc-43fc-8bdd-7bf4faa1fad7",
    scryfall_id = "fa8d306f-3f4e-4c21-9461-caa3daf4fc50",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Songstitcher",
        mana_cost = mana!("{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!("{1}{W}"),
        &[Effect::ModifyTargets {
            modifier: Modifier::PreventDamageFromIt,
            duration: Duration::UntilEndOfTurn,
        }],
        target = Some(TargetSpec::Object(&ATTACKING_FLYER)),
    ),],
);
