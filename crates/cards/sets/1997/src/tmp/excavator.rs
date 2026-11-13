//! Excavator — {2} — Artifact
//! Oracle: {T}, Sacrifice a basic land: Target creature gains landwalk of each of the land types of the sacrificed land until end of turn. (It can't be blocked as long as defending player controls a land of any of those types.)
//! Set: TMP #287 — Tempest | Scryfall ID: 6dc3d5b4-b04f-4b34-afd2-72fb3de0a33b | Oracle ID: 2d5ebcd7-07c2-412e-b0d9-54e3a88895fd
// IMPLEMENTED — the sacrificed basic land's one land type decides which landwalk the target gains.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// A basic land has exactly one land type, so "landwalk of each of the land
// types of the sacrificed land" is one landwalk, chosen by what was paid.
const PLAINS: Filter = Filter::HasSubtype(subtypes::land::PLAINS);
const ISLAND: Filter = Filter::HasSubtype(subtypes::land::ISLAND);
const SWAMP: Filter = Filter::HasSubtype(subtypes::land::SWAMP);
const MOUNTAIN: Filter = Filter::HasSubtype(subtypes::land::MOUNTAIN);
const FOREST: Filter = Filter::HasSubtype(subtypes::land::FOREST);

card!(
    index = index::EXCAVATOR,
    oracle_id = "2d5ebcd7-07c2-412e-b0d9-54e3a88895fd",
    scryfall_id = "6dc3d5b4-b04f-4b34-afd2-72fb3de0a33b",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Excavator",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!(TapSelf, Sacrifice(&Filter::BASIC_LAND)),
        &[
            Effect::IfCondition {
                condition: Condition::SacrificedForCost(&PLAINS),
                then: &[Effect::PumpTarget {
                    power: Amount::Fixed(0),
                    toughness: Amount::Fixed(0),
                    keywords: KeywordSet::PLAINSWALK,
                    duration: Duration::UntilEndOfTurn,
                }],
                otherwise: &[],
            },
            Effect::IfCondition {
                condition: Condition::SacrificedForCost(&ISLAND),
                then: &[Effect::PumpTarget {
                    power: Amount::Fixed(0),
                    toughness: Amount::Fixed(0),
                    keywords: KeywordSet::ISLANDWALK,
                    duration: Duration::UntilEndOfTurn,
                }],
                otherwise: &[],
            },
            Effect::IfCondition {
                condition: Condition::SacrificedForCost(&SWAMP),
                then: &[Effect::PumpTarget {
                    power: Amount::Fixed(0),
                    toughness: Amount::Fixed(0),
                    keywords: KeywordSet::SWAMPWALK,
                    duration: Duration::UntilEndOfTurn,
                }],
                otherwise: &[],
            },
            Effect::IfCondition {
                condition: Condition::SacrificedForCost(&MOUNTAIN),
                then: &[Effect::PumpTarget {
                    power: Amount::Fixed(0),
                    toughness: Amount::Fixed(0),
                    keywords: KeywordSet::MOUNTAINWALK,
                    duration: Duration::UntilEndOfTurn,
                }],
                otherwise: &[],
            },
            Effect::IfCondition {
                condition: Condition::SacrificedForCost(&FOREST),
                then: &[Effect::PumpTarget {
                    power: Amount::Fixed(0),
                    toughness: Amount::Fixed(0),
                    keywords: KeywordSet::FORESTWALK,
                    duration: Duration::UntilEndOfTurn,
                }],
                otherwise: &[],
            },
        ],
        target = Some(TargetSpec::Object(&Filter::CREATURE)),
    )],
);
