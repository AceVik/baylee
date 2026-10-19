//! Scarecrow — {5} — Artifact Creature — Scarecrow
//! Oracle: {6}, {T}: Prevent all damage that would be dealt to you this turn by creatures with flying.
//! Set: ME4 #225 — Masters Edition IV | Scryfall ID: 4b0dcd50-7403-48e5-b928-5d35d7304f51 | Oracle ID: 58616e15-531f-4680-8294-35ab7e962c96

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const CREATURE_WITH_FLYING: Filter =
    Filter::And(&[Filter::CREATURE, Filter::HasKeyword(KeywordSet::FLYING)]);

card!(
    index = index::SCARECROW,
    oracle_id = "58616e15-531f-4680-8294-35ab7e962c96",
    scryfall_id = "4b0dcd50-7403-48e5-b928-5d35d7304f51",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Scarecrow",
        mana_cost = mana!("{5}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::SCARECROW],
        power = Some(2),
        toughness = Some(2),
        abilities = &[activated!(
            cost!("{6}", TapSelf),
            &[Effect::ModifySource {
                modifier: Modifier::PreventDamageToYouFrom(&CREATURE_WITH_FLYING),
                duration: Duration::UntilEndOfTurn,
            }],
        )],
    ),],
);
