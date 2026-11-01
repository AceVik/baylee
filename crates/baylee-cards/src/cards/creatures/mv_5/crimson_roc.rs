//! Crimson Roc — {4}{R} — Creature — Bird
//! Oracle: Flying
//! Oracle: Whenever this creature blocks a creature without flying, this creature gets +1/+0 and gains first strike until end of turn.
//! Set: MIR #168 — Mirage | Scryfall ID: 2e63836c-37af-49c1-84a1-8650ed072805 | Oracle ID: acc7d3b8-60f2-4f3b-9cf9-b6b39627d8f6
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const CREATURE_WITHOUT_FLYING: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Not(&Filter::HasKeyword(KeywordSet::FLYING)),
]);

card!(
    index = index::CRIMSON_ROC,
    oracle_id = "acc7d3b8-60f2-4f3b-9cf9-b6b39627d8f6",
    scryfall_id = "2e63836c-37af-49c1-84a1-8650ed072805",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Crimson Roc",
        mana_cost = mana!("{4}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BIRD],
        power = Some(2),
        toughness = Some(2),
        keywords = KeywordSet::FLYING,
    ),],
    abilities = &[triggered!(
        Trigger::BlocksCreature(&CREATURE_WITHOUT_FLYING),
        &[
            Effect::ModifySource {
                modifier: Modifier::ModifyPT(1, 0),
                duration: Duration::UntilEndOfTurn,
            },
            Effect::ModifySource {
                modifier: Modifier::AddKeyword(KeywordSet::FIRST_STRIKE),
                duration: Duration::UntilEndOfTurn,
            },
        ],
    )],
);
