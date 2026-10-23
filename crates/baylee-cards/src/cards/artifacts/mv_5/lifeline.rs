//! Lifeline — {5} — Artifact
//! Oracle: Whenever a creature dies, if another creature is on the battlefield, return the first card to the battlefield under its owner's control at the beginning of the next end step.
//! Set: USG #299 — Urza's Saga | Scryfall ID: 40cee82f-36b2-48a9-930a-8e23cb2742fc | Oracle ID: f475b636-eb7f-4dd7-bc87-a1af07064ffe
// PARTIAL

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LIFELINE,
    oracle_id = "f475b636-eb7f-4dd7-bc87-a1af07064ffe",
    scryfall_id = "40cee82f-36b2-48a9-930a-8e23cb2742fc",
    coverage = Coverage::Partial(
        "returning the dying creature card at the beginning of the next end step rather than immediately is not in AtNextEndStep; triggers whenever a creature dies while another creature is on the battlefield to return that card from its owner's graveyard to the battlefield under its owner's control immediately",
    ),
    faces = &[face!(
        name = "Lifeline",
        mana_cost = mana!("{5}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[triggered!(
        Trigger::Dies(&Filter::CREATURE),
        &[Effect::GraveyardToBattlefield {
            target: TargetSpec::EventObject,
            owner_control: true,
            counters: None,
        }],
        condition = Some(Condition::BattlefieldCount(&Filter::CREATURE, 1)),
    )],
);
