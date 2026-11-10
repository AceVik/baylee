//! Scavenger Grounds — (no cost) — Land — Desert
//! Oracle: {T}: Add {C}.
//! Oracle: {2}, {T}, Sacrifice a Desert: Exile all graveyards.
//! Set: MSC #263 — Marvel Super Heroes Commander | Scryfall ID: 9fbe68ba-ffe5-4fe0-ac0a-0b3221e4f395 | Oracle ID: 5ece7d03-9ee7-4953-a06e-9d8e41874903
// IMPLEMENTED — {T} for {C}, and {2}, {T}, sacrifice a Desert to exile every
// graveyard (CostPart::Sacrifice + Effect::ExileGraveyard over each player).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SCAVENGER_GROUNDS,
    oracle_id = "5ece7d03-9ee7-4953-a06e-9d8e41874903",
    scryfall_id = "9fbe68ba-ffe5-4fe0-ac0a-0b3221e4f395",
    faces = &[face!(
        name = "Scavenger Grounds",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::DESERT],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!(
                "{2}",
                TapSelf,
                Sacrifice(&Filter::HasSubtype(subtypes::land::DESERT))
            ),
            &[Effect::ExileGraveyard {
                player: PlayerRel::EachPlayer,
            }]
        ),
    ],
);
