//! Purging Scythe — {5} — Artifact
//! Oracle: At the beginning of your upkeep, this artifact deals 2 damage to the creature with the least toughness. If two or more creatures are tied for least toughness, you choose one of them.
//! Set: USG #308 — Urza's Saga | Scryfall ID: 0ef8128a-e0f5-43d6-b7bd-3f2003d83eab | Oracle ID: 4410e3c4-b8c5-46d7-9418-7490bb69d8c4
// PARTIAL: finding and choosing among creatures tied for the least toughness without targeting is not in Effect; at the beginning of your upkeep, deals 2 damage to each creature.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PURGING_SCYTHE,
    oracle_id = "4410e3c4-b8c5-46d7-9418-7490bb69d8c4",
    scryfall_id = "0ef8128a-e0f5-43d6-b7bd-3f2003d83eab",
    coverage = Coverage::Partial(
        "finding and choosing among creatures tied for the least toughness without targeting is not in Effect; at the beginning of your upkeep, deals 2 damage to each creature",
    ),
    faces = &[face!(
        name = "Purging Scythe",
        mana_cost = mana!("{5}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You,
        },
        &[Effect::DealDamageEach {
            amount: Amount::Fixed(2),
            filter: &Filter::CREATURE,
        }],
    ),],
);
