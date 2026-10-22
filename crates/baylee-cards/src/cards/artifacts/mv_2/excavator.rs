//! Excavator — {2} — Artifact
//! Oracle: {T}, Sacrifice a basic land: Target creature gains landwalk of each of the land types of the sacrificed land until end of turn. (It can't be blocked as long as defending player controls a land of any of those types.)
//! Set: TMP #287 — Tempest | Scryfall ID: 6dc3d5b4-b04f-4b34-afd2-72fb3de0a33b | Oracle ID: 2d5ebcd7-07c2-412e-b0d9-54e3a88895fd
// PARTIAL

use baylee_cards_dsl::prelude::*;

const BASIC_LANDWALKS: KeywordSet = KeywordSet::PLAINSWALK
    .union(KeywordSet::ISLANDWALK)
    .union(KeywordSet::SWAMPWALK)
    .union(KeywordSet::MOUNTAINWALK)
    .union(KeywordSet::FORESTWALK);

card!(
    index = index::EXCAVATOR,
    oracle_id = "2d5ebcd7-07c2-412e-b0d9-54e3a88895fd",
    scryfall_id = "6dc3d5b4-b04f-4b34-afd2-72fb3de0a33b",
    coverage = Coverage::Partial(
        "{T}, Sacrifice a basic land grants target creature plainswalk, islandwalk, swampwalk, mountainwalk, and forestwalk until end of turn; granting only the landwalk abilities corresponding to the sacrificed land's specific land types is not yet modeled."
    ),
    faces = &[face!(
        name = "Excavator",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!(TapSelf, Sacrifice(&Filter::BASIC_LAND)),
        &[Effect::PumpTarget {
            power: Amount::Fixed(0),
            toughness: Amount::Fixed(0),
            keywords: BASIC_LANDWALKS,
            duration: Duration::UntilEndOfTurn,
        }],
        target = Some(TargetSpec::Object(&Filter::CREATURE)),
    )],
);
