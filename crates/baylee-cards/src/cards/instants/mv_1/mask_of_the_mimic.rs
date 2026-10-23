//! Mask of the Mimic — {U} — Instant
//! Oracle: As an additional cost to cast this spell, sacrifice a creature.
//! Oracle: Search your library for a card with the same name as target nontoken creature, put that card onto the battlefield, then shuffle.
//! Set: STH #37 — Stronghold | Scryfall ID: 09891c87-eb74-4174-ad46-11a7f79de859 | Oracle ID: f1a16f6d-ea11-485e-8c5b-7f919a731fe0
// PARTIAL: sacrifices a creature as an additional cost, targets a nontoken creature, and searches your library for a creature card to put onto the battlefield; restricting the search to a card with the same name as the target is not supported

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MASK_OF_THE_MIMIC,
    oracle_id = "f1a16f6d-ea11-485e-8c5b-7f919a731fe0",
    scryfall_id = "09891c87-eb74-4174-ad46-11a7f79de859",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "sacrifices a creature as an additional cost, targets a nontoken creature, and searches your library for a creature card to put onto the battlefield; restricting the search to a card with the same name as the target is not supported"
    ),
    faces = &[face!(
        name = "Mask of the Mimic",
        mana_cost = mana!("{U}"),
        types = TypeSet::INSTANT,
        mandatory_additional_costs = &[CostPart::Sacrifice(&Filter::CREATURE)],
    ),],
    abilities = &[spell!(
        &[Effect::SearchLibrary {
            filter: &Filter::CREATURE,
            finds: &[Find::BATTLEFIELD],
            optional: false,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(
            &Filter::NONTOKEN_CREATURE
        ))),
    )],
);
