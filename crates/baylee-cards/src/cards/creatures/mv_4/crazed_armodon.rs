//! Crazed Armodon — {2}{G}{G} — Creature — Elephant
//! Oracle: {G}: This creature gets +3/+0 and gains trample until end of turn. Destroy this creature at the beginning of the next end step. Activate only once each turn.
//! Set: TMP #220 — Tempest | Scryfall ID: b83e4b36-57c1-493d-ab79-52075990b2d5 | Oracle ID: ce7d672a-7cb9-40e5-a8ad-dd3ce0f8ecd3
// IMPLEMENTED — Pays {G} once each turn to get +3/+0 and trample until end of turn and be destroyed at the beginning of the next end step.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CRAZED_ARMODON,
    oracle_id = "ce7d672a-7cb9-40e5-a8ad-dd3ce0f8ecd3",
    scryfall_id = "b83e4b36-57c1-493d-ab79-52075990b2d5",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Crazed Armodon",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELEPHANT],
        power = Some(3),
        toughness = Some(3),
    ),],
    abilities = &[activated!(
        cost!("{G}"),
        &[
            Effect::PumpFilter {
                filter: &Filter::This,
                controlled_by: None,
                power: Amount::Fixed(3),
                toughness: Amount::Fixed(0),
                keywords: KeywordSet::TRAMPLE,
                duration: Duration::UntilEndOfTurn,
            },
            Effect::AtNextEndStep {
                effects: &[Effect::destroy(TargetSpec::EventObject)],
            },
        ],
        limit = ActivationLimit::PerTurn(1),
    )],
);
