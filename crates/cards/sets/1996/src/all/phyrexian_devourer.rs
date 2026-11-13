//! Phyrexian Devourer — {6} — Artifact Creature — Phyrexian Construct
//! Oracle: When this creature's power is 7 or greater, sacrifice it.
//! Oracle: Exile the top card of your library: Put X +1/+1 counters on this creature, where X is the exiled card's mana value.
//! Set: ME2 #216 — Masters Edition II | Scryfall ID: ca4e27ca-a731-44f5-8e47-701c7a7e6149 | Oracle ID: 28b1ed69-784c-4642-86c7-104792d5afc2
// PARTIAL — the sacrifice at power 7 or more is played; the ability that exiles the top card of your library and grows it is not, so it stays a 1/1.
// NOT SUPPORTED: exiling the top card of your library as an activation cost and putting X +1/+1 counters on Phyrexian Devourer equal to the exiled card's mana value has no CostPart or Amount representation.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static SELF_POWER_7: Condition = Condition::SourceMatches(&Filter::PowerAtLeast(7));

card!(
    index = index::PHYREXIAN_DEVOURER,
    oracle_id = "28b1ed69-784c-4642-86c7-104792d5afc2",
    scryfall_id = "ca4e27ca-a731-44f5-8e47-701c7a7e6149",
    coverage = Coverage::Partial(
        "Exiling the top card of your library as an activation cost and putting X +1/+1 counters on Phyrexian Devourer equal to the exiled card's mana value has no CostPart or Amount representation; in play, Phyrexian Devourer sacrifices itself when its power is 7 or greater.",
    ),
    faces = &[face!(
        name = "Phyrexian Devourer",
        mana_cost = mana!("{6}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::PHYREXIAN, subtypes::creature::CONSTRUCT],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[triggered!(
        Trigger::State(&SELF_POWER_7),
        &[Effect::SacrificeSelf],
    )],
);
