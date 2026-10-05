//! Bronze Horse — {7} — Artifact Creature — Horse
//! Oracle: Trample
//! Oracle: As long as you control another creature, prevent all damage that would be dealt to this creature by spells that target it.
//! Set: ME4 #186 — Masters Edition IV | Scryfall ID: 0003b07e-0d6e-4844-93c7-3f1f6a7d8c4d | Oracle ID: a7887b24-977d-4a40-bb30-4bf467e5dba6
// PARTIAL — trample is on the card; the spell-damage prevention is not.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BRONZE_HORSE,
    oracle_id = "a7887b24-977d-4a40-bb30-4bf467e5dba6",
    scryfall_id = "0003b07e-0d6e-4844-93c7-3f1f6a7d8c4d",
    faces = &[face!(
        name = "Bronze Horse",
        mana_cost = mana!("{7}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::HORSE],
        power = Some(4),
        toughness = Some(4),
    ),],
    keywords = KeywordSet::TRAMPLE,
    coverage = Coverage::Partial(
        "no Modifier prevents damage from a spell that targets the affected \
         creature: PreventDamageToIt is combat damage from any source, and \
         CantBeTargetedBy stops targeting rather than the damage"
    ),
    // NOT SUPPORTED: "As long as you control another creature, prevent all
    // damage that would be dealt to this creature by spells that target
    // it." — no `Modifier` prevents a spell's damage to the affected object,
    // and the one prevention effect that names the object
    // (`PreventDamageToIt`) covers combat damage alone.
);
