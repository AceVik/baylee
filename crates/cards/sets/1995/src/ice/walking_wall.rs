//! Walking Wall — {4} — Artifact Creature — Wall
//! Oracle: Defender
//! Oracle: {3}: This creature gets +3/-1 until end of turn and can attack this turn as though it didn't have defender. Activate only once each turn.
//! Set: ME1 #172 — Masters Edition | Scryfall ID: 4a837483-ae0f-4060-910d-1aa272e679ed | Oracle ID: 909201e5-5599-442a-9e4f-ff61f187e957

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WALKING_WALL,
    oracle_id = "909201e5-5599-442a-9e4f-ff61f187e957",
    scryfall_id = "4a837483-ae0f-4060-910d-1aa272e679ed",
    coverage = Coverage::Implemented,
    keywords = KeywordSet::DEFENDER,
    faces = &[face!(
        name = "Walking Wall",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::WALL],
        power = Some(0),
        toughness = Some(6),
    ),],
    abilities = &[activated!(
        cost!("{3}"),
        &[
            Effect::continuous(
                &Filter::This,
                Modifier::ModifyPT(3, -1),
                Duration::UntilEndOfTurn,
            ),
            Effect::continuous(
                &Filter::This,
                Modifier::AttacksDespiteDefender,
                Duration::UntilEndOfTurn,
            ),
        ],
        limit = ActivationLimit::PerTurn(1)
    )],
);
