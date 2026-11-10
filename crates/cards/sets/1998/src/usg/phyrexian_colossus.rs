//! Phyrexian Colossus — {7} — Artifact Creature — Phyrexian Golem
//! Oracle: This creature doesn't untap during your untap step.
//! Oracle: Pay 8 life: Untap this creature.
//! Oracle: This creature can't be blocked except by three or more creatures.
//! Set: USG #305 — Urza's Saga | Scryfall ID: 1b0ba37a-ce52-4c01-89aa-25e90cda04ba | Oracle ID: caab555c-7852-418a-bdc3-f908af7e0932
// PARTIAL: blocking restriction 'can't be blocked except by three or more creatures' is not in Modifier; doesn't untap during your untap step and pays 8 life to untap itself.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PHYREXIAN_COLOSSUS,
    oracle_id = "caab555c-7852-418a-bdc3-f908af7e0932",
    scryfall_id = "1b0ba37a-ce52-4c01-89aa-25e90cda04ba",
    coverage = Coverage::Partial(
        "blocking restriction 'can't be blocked except by three or more creatures' is not in Modifier; doesn't untap during your untap step and pays 8 life to untap itself",
    ),
    faces = &[face!(
        name = "Phyrexian Colossus",
        mana_cost = mana!("{7}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::PHYREXIAN, subtypes::creature::GOLEM],
        power = Some(8),
        toughness = Some(8),
    ),],
    // NOT SUPPORTED: blocking restriction 'can't be blocked except by three or more creatures' is not in Modifier; doesn't untap during your untap step and pays 8 life to untap itself.
    abilities = &[
        static_ability!(Filter::This, Modifier::DoesNotUntap),
        activated!(cost!(PayLife(8)), &[Effect::UntapSelf]),
    ],
);
