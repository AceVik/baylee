//! Marble Priest — {5} — Artifact Creature — Cleric
//! Oracle: All Walls able to block this creature do so.
//! Oracle: Prevent all combat damage that would be dealt to this creature by Walls.
//! Set: LEG #286 — Legends | Scryfall ID: 459b71d7-34c1-43b9-93ff-364f95aa4789 | Oracle ID: 96bcabee-e84e-409f-8439-80f5308d73e9
// PARTIAL — both abilities are off the card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MARBLE_PRIEST,
    oracle_id = "96bcabee-e84e-409f-8439-80f5308d73e9",
    scryfall_id = "459b71d7-34c1-43b9-93ff-364f95aa4789",
    faces = &[face!(
        name = "Marble Priest",
        mana_cost = mana!("{5}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::CLERIC],
        power = Some(3),
        toughness = Some(3),
    ),],
    coverage = Coverage::Partial(
        "MustBeBlockedByAllAble names every creature able to block and \
         PreventDamageToIt names every damage source, and neither carries \
         the Wall restriction this card prints"
    ),
    // NOT SUPPORTED: "All Walls able to block this creature do so." —
    // `Modifier::MustBeBlockedByAllAble` says "all creatures" and takes no
    // filter, so it cannot name Walls alone.
    // NOT SUPPORTED: "Prevent all combat damage that would be dealt to this
    // creature by Walls." — `Modifier::PreventDamageToIt` prevents combat
    // damage from every source and takes no filter.
);
