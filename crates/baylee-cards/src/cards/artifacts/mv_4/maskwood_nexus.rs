//! Maskwood Nexus — {4} — Artifact
//! Oracle: Creatures you control are every creature type. The same is true for creature spells you control and creature cards you own that aren't on the battlefield.
//! Oracle: {3}, {T}: Create a 2/2 blue Shapeshifter creature token with changeling. (It is every creature type.)
//! {3}, {T}: Create a 2/2 blue Shapeshifter creature token with changeling. (It is every creature type.)
//! Set: CLB #865 — Commander Legends: Battle for Baldur's Gate | Scryfall ID: 1246c42d-57c0-4cba-959a-15ad89d8a50b | Oracle ID: 9b2cdbed-c733-409b-b0e4-2c8960c25111
// IMPLEMENTED — cross-zone type granting (library/hand/graveyard/stack) +
// shapeshifter token creation.

use baylee_cards_dsl::prelude::*;

// Creatures you control (battlefield), creature spells you control (stack),
// creature cards you own that aren't on the battlefield — CR 613.4 layer 4.
static NEXUS_FILTER: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Or(&[
        Filter::And(&[
            Filter::ControlledByYou,
            Filter::InZone(ZoneRef::Battlefield),
        ]),
        Filter::And(&[Filter::ControlledByYou, Filter::InZone(ZoneRef::Stack)]),
        Filter::And(&[Filter::OwnedByYou, Filter::InZone(ZoneRef::NotBattlefield)]),
    ]),
]);

use crate::tokens::SHAPESHIFTER_2_2_BLUE_CHANGELING as SHAPESHIFTER_TOKEN;

card!(
    index = index::MASKWOOD_NEXUS,
    oracle_id = "9b2cdbed-c733-409b-b0e4-2c8960c25111",
    scryfall_id = "1246c42d-57c0-4cba-959a-15ad89d8a50b",
    faces = &[face!(
        name = "Maskwood Nexus",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    )],
    coverage = Coverage::Implemented,
    abilities = &[
        AbilityDef::Static(StaticAbility {
            layer: Layer::Type,
            filter: NEXUS_FILTER,
            modifier: Modifier::AllCreatureTypes,
        }),
        activated!(
            cost!("{3}", TapSelf),
            &[Effect::CreateToken {
                token: &SHAPESHIFTER_TOKEN,
            }]
        ),
    ],
);

// Engine-level coverage lives in baylee-engine (m2 cross-zone test):
// with Nexus out, a non-Ally creature card in the library counts as an
// Ally for General Tazri's ETB tutor.
