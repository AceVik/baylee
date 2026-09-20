//! Rhystic Cave — (no cost) — Land
//! Oracle: {T}: Choose a color. Add one mana of that color unless any player pays {1}. Activate only as an instant.
//! Set: PCY #142 — Prophecy | Scryfall ID: 4ae74463-4426-4ad4-b7a2-324694854245 | Oracle ID: 609fbc2c-514a-4feb-aaad-b9e6dcfd335c
// PARTIAL — {T} adds one mana of any color, and "only as an instant" is the
// timing a mana ability already has; the payment half of the sentence has no
// decider to hand `Effect::PlayerMayPayOr`.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RHYSTIC_CAVE,
    oracle_id = "609fbc2c-514a-4feb-aaad-b9e6dcfd335c",
    scryfall_id = "4ae74463-4426-4ad4-b7a2-324694854245",
    faces = &[face!(name = "Rhystic Cave", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "\"unless any player pays {1}\": Effect::PlayerMayPayOr asks one PlayerRel, and no variant of it is \"any player\""
    ),
    abilities = &[
        mana_ability!(&[Effect::mana_of_any_color()]),
        // NOT SUPPORTED: "Add one mana of that color unless any player pays
        // {1}" — the payment half. `Effect::PlayerMayPayOr` names a single
        // `PlayerRel` as the decider and the pool has no "any player" one:
        // `EachPlayer` is not it, because it runs the non-payment branch once
        // per declining seat, which would hand the caster the mana before the
        // next seat could pay it. The colour choice ("Choose a color.") is
        // `ManaSource::Choice`, and the {T} cost is built.
    ],
);
