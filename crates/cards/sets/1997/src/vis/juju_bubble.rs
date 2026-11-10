//! Juju Bubble — {1} — Artifact
//! Oracle: Cumulative upkeep {1} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Oracle: When you play a card, sacrifice this artifact.
//! Oracle: {2}: You gain 1 life.
//! Set: VIS #147 — Visions | Scryfall ID: a5fa8208-7d65-4f8f-b07e-f5c3a66e1143 | Oracle ID: 9344ae2e-cb2e-400d-9157-e8158ce31813
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::JUJU_BUBBLE,
    oracle_id = "9344ae2e-cb2e-400d-9157-e8158ce31813",
    scryfall_id = "a5fa8208-7d65-4f8f-b07e-f5c3a66e1143",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Juju Bubble",
        mana_cost = mana!("{1}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[
                Effect::AddCounter {
                    kind: counters::AGE,
                    amount: Amount::Fixed(1),
                },
                Effect::PlayerMayPayManaEachThenElse {
                    player: PlayerRel::You,
                    each: mana!("{1}"),
                    times: Amount::CountersOnSource(counters::AGE),
                    paid: &[],
                    unpaid: &[Effect::SacrificeSelf],
                },
            ],
            condition = Some(Condition::SourceMatches(&Filter::InZone(
                ZoneRef::Battlefield,
            ))),
        ),
        // "When you play a card" is playing a land or casting a spell (CR 701.18a): two events,
        // so two triggers say the one printed sentence.
        triggered!(Trigger::PlaysLand(PlayerRel::You), &[Effect::SacrificeSelf]),
        triggered!(
            Trigger::SpellCast(&Filter::ControlledByYou),
            &[Effect::SacrificeSelf],
        ),
        activated!(cost!("{2}"), &[Effect::gain_life(1)]),
    ],
);
