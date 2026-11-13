//! Reprocess — {2}{B}{B} — Sorcery
//! Oracle: Sacrifice any number of artifacts, creatures, and/or lands. Draw a card for each permanent sacrificed this way.
//! Set: USG #154 — Urza's Saga | Scryfall ID: 569408b3-43c8-4cb1-a70f-15ab2aeaf8ed | Oracle ID: 27177cb0-1670-488e-82be-4610da5a7de3
// PARTIAL: sacrificing any number of artifacts, creatures, and/or lands on resolution and drawing a card for each sacrificed is not in Effect; optionally sacrifices one artifact, creature, or land you control and draws a card if you do.

use baylee_cards_dsl::prelude::*;

static YOUR_ARTIFACT_CREATURE_OR_LAND: Filter = Filter::And(&[
    Filter::HasType(
        TypeSet::ARTIFACT
            .union(TypeSet::CREATURE)
            .union(TypeSet::LAND),
    ),
    Filter::ControlledByYou,
]);

card!(
    index = index::REPROCESS,
    oracle_id = "27177cb0-1670-488e-82be-4610da5a7de3",
    scryfall_id = "569408b3-43c8-4cb1-a70f-15ab2aeaf8ed",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "sacrificing any number of artifacts, creatures, and/or lands on resolution and drawing a card for each sacrificed is not in Effect; optionally sacrifices one artifact, creature, or land you control and draws a card if you do",
    ),
    faces = &[face!(
        name = "Reprocess",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::SORCERY,
    ),],
    // NOT SUPPORTED: sacrificing any number of artifacts, creatures, and/or lands on resolution and drawing a card for each sacrificed is not in Effect; optionally sacrifices one artifact, creature, or land you control and draws a card if you do.
    abilities = &[spell!(&[Effect::PlayerMayPayCostThen {
        player: PlayerRel::You,
        cost: &CostPart::Sacrifice(&YOUR_ARTIFACT_CREATURE_OR_LAND),
        effect: &Effect::draw(1),
    }]),],
);
