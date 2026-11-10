//! Cradle of the Accursed — (no cost) — Land — Desert
//! Oracle: {T}: Add {C}.
//! Oracle: {3}, {T}, Sacrifice this land: Create a 2/2 black Zombie creature token. Activate only as a sorcery.
//! Set: AKH #241 — Amonkhet | Scryfall ID: 41713e82-c3d3-4c2f-b075-f684cbd68ce8 | Oracle ID: 36d06c91-5080-4f97-8e4c-ca8ac390e808
// IMPLEMENTED — {T}: Add {C}, and the sorcery-speed activation that trades
// the land for its Zombie.

use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CRADLE_OF_THE_ACCURSED,
    oracle_id = "36d06c91-5080-4f97-8e4c-ca8ac390e808",
    scryfall_id = "41713e82-c3d3-4c2f-b075-f684cbd68ce8",
    faces = &[face!(
        name = "Cradle of the Accursed",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::DESERT],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // "Activate only as a sorcery" is the printed restriction and so a
        // `timing`, not a condition: CR 602.5d is about *when* priority is
        // held, which is a different question from whether the board earns
        // the ability, and the two are checked in different places.
        activated!(
            cost!("{3}", TapSelf, SacrificeSelf),
            &[Effect::CreateToken {
                token: &generated_tokens::ZOMBIE_2_2_BLACK
            }],
            timing = ActivationTiming::SorcerySpeed,
        ),
    ],
);
