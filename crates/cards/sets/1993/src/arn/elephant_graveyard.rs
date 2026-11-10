//! Elephant Graveyard — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Regenerate target Elephant.
//! Set: ME4 #244 — Masters Edition IV | Scryfall ID: 88e7d9d5-3bca-4791-b850-5ae104706042 | Oracle ID: 8ada7388-fd8b-434c-a17a-bce19cf3e615
// IMPLEMENTED — {T}: Add {C}, and the regeneration shield the second line
// hands to an Elephant.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

/// "Target Elephant" — the printed word and nothing else. It is not
/// `Filter::CREATURE` and an Elephant at once: an animated land with the
/// subtype is an Elephant, and a card that says the type says only the
/// type (CR 205.3).
static ELEPHANT: Filter = Filter::HasSubtype(creature::ELEPHANT);

card!(
    index = index::ELEPHANT_GRAVEYARD,
    oracle_id = "8ada7388-fd8b-434c-a17a-bce19cf3e615",
    scryfall_id = "88e7d9d5-3bca-4791-b850-5ae104706042",
    faces = &[face!(name = "Elephant Graveyard", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            Cost::TAP,
            &[Effect::regenerate(TargetSpec::Object(&ELEPHANT))],
            target = Some(TargetSpec::Object(&ELEPHANT))
        ),
    ],
);
