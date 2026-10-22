//! Aku Djinn — {3}{B}{B} — Creature — Djinn
//! Oracle: Trample
//! Oracle: At the beginning of your upkeep, put a +1/+1 counter on each creature each opponent controls.
//! Set: VIS #51 — Visions | Scryfall ID: 369a5df5-fc36-476c-84f4-ec4bdeb4f9d2 | Oracle ID: 233c6686-ead8-4569-941d-3bfa768ff270
// IMPLEMENTED — trample and upkeep trigger putting a +1/+1 counter on each creature each opponent controls.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::AKU_DJINN,
    oracle_id = "233c6686-ead8-4569-941d-3bfa768ff270",
    scryfall_id = "369a5df5-fc36-476c-84f4-ec4bdeb4f9d2",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    keywords = KeywordSet::TRAMPLE,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Aku Djinn",
        mana_cost = mana!("{3}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DJINN],
        power = Some(5),
        toughness = Some(6),
        keywords = KeywordSet::TRAMPLE,
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You,
        },
        &[Effect::AddCounterFilter {
            filter: &Filter::OPPONENT_CREATURE,
            kind: CounterKind::P1P1,
            amount: Amount::Fixed(1),
        }],
    )],
);
