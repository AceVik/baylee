//! Rhystic Cave — (no cost) — Land
//! Oracle: {T}: Choose a color. Add one mana of that color unless any player pays {1}. Activate only as an instant.
//! Set: PCY #142 — Prophecy | Scryfall ID: 4ae74463-4426-4ad4-b7a2-324694854245 | Oracle ID: 609fbc2c-514a-4feb-aaad-b9e6dcfd335c
use baylee_cards_dsl::prelude::*;

card!(
    index = index::RHYSTIC_CAVE,
    oracle_id = "609fbc2c-514a-4feb-aaad-b9e6dcfd335c",
    scryfall_id = "4ae74463-4426-4ad4-b7a2-324694854245",
    faces = &[face!(name = "Rhystic Cave", types = TypeSet::LAND)],
    coverage = Coverage::Implemented,
    abilities = &[activated!(
        Cost::TAP,
        &[Effect::AddManaUnlessAnyPlayerPays {
            source: ManaSource::Choice(ALL_MANA_COLORS),
            amount: 1,
            cost: mana!("{1}"),
        }],
        mana_ability = true,
        timing = ActivationTiming::PriorityOnly
    )],
);
