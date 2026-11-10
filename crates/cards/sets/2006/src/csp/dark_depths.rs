//! Dark Depths — (no cost) — Legendary Snow Land
//! Oracle: Dark Depths enters with ten ice counters on it.
//! Oracle: {3}: Remove an ice counter from Dark Depths.
//! Oracle: When Dark Depths has no ice counters on it, sacrifice it. If you do, create Marit Lage, a legendary 20/20 black Avatar creature token with flying and indestructible.
//! Set: DMR #244 — Dominaria Remastered | Scryfall ID: 3a8b11ad-d077-40b6-988c-0462e118fe3d | Oracle ID: c9b82110-7dfd-4617-9399-9510be449043
// IMPLEMENTED — enters with 10 ice counters, {3} removes an ice counter, state trigger (CR 603.8) sacrifices and creates Marit Lage when no ice counters remain.

use crate::generated_tokens;
use baylee_cards_dsl::counters;
use baylee_cards_dsl::prelude::*;

card!(
    index = index::DARK_DEPTHS,
    oracle_id = "c9b82110-7dfd-4617-9399-9510be449043",
    scryfall_id = "3a8b11ad-d077-40b6-988c-0462e118fe3d",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Dark Depths",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY.union(SupertypeSet::SNOW),
        enter_modifiers = &[EnterModifier::WithCounters {
            kind: counters::ICE,
            amount: Amount::Fixed(10),
        }],
    ),],
    abilities = &[
        activated!(
            cost!("{3}"),
            &[Effect::RemoveCounterSelf {
                kind: counters::ICE,
                n: 1,
            }],
        ),
        triggered!(
            Trigger::State(&Condition::Not(&Condition::CountersOnSelf(
                counters::ICE,
                1
            ))),
            &[Effect::SacrificeSelfThen {
                then: &[Effect::CreateToken {
                    token:
                        &generated_tokens::MARIT_LAGE_LEGENDARY_20_20_BLACK_FLYING_INDESTRUCTIBLE,
                }],
            }],
        ),
    ],
);
