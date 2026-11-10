//! Skullclamp — {1} — Artifact — Equipment
//! Oracle: Equipped creature gets +1/-1.
//! Oracle: Whenever equipped creature dies, draw two cards.
//! Oracle: Equip {1}
//! Set: MSC #210 — Marvel Super Heroes Commander | Scryfall ID: 1d8b007b-3169-4ee3-80c7-781fc096fc7a | Oracle ID: 65986c1b-8e51-4604-b685-d82fa7d1263a
// IMPLEMENTED — static +1/-1 to equipped creature, death trigger to draw two, and equip {1}.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SKULLCLAMP,
    oracle_id = "65986c1b-8e51-4604-b685-d82fa7d1263a",
    scryfall_id = "1d8b007b-3169-4ee3-80c7-781fc096fc7a",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Skullclamp",
        mana_cost = mana!("{1}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[subtypes::artifact::EQUIPMENT],
    ),],
    abilities = &[
        static_ability!(Filter::AttachedToBySource, Modifier::ModifyPT(1, -1)),
        triggered!(
            Trigger::Dies(&Filter::AttachedToBySource),
            &[Effect::draw(2)]
        ),
        equip!("{1}"),
    ],
);
