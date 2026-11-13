//! Snow Fortress — {5} — Artifact Creature — Wall
//! Oracle: Defender (This creature can't attack.)
//! Oracle: {1}: This creature gets +1/+0 until end of turn.
//! Oracle: {1}: This creature gets +0/+1 until end of turn.
//! Oracle: {3}: This creature deals 1 damage to target creature without flying that's attacking you.
//! Set: ME2 #220 — Masters Edition II | Scryfall ID: e283874b-a2c8-46a2-94ad-17d1fc6cc1e5 | Oracle ID: 7709ed84-ed71-4dd2-b002-b257418dd79e
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static NONFLYING_ATTACKER_OF_YOU: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Not(&Filter::HasKeyword(KeywordSet::FLYING)),
    Filter::AttackingYou,
]);

card!(
    index = index::SNOW_FORTRESS,
    oracle_id = "7709ed84-ed71-4dd2-b002-b257418dd79e",
    scryfall_id = "e283874b-a2c8-46a2-94ad-17d1fc6cc1e5",
    keywords = KeywordSet::DEFENDER,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Snow Fortress",
        mana_cost = mana!("{5}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::WALL],
        power = Some(0),
        toughness = Some(4),
    ),],
    abilities = &[
        activated!(
            cost!("{1}"),
            &[Effect::PumpFilter {
                filter: &Filter::This,
                controlled_by: None,
                power: Amount::Fixed(1),
                toughness: Amount::Fixed(0),
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn,
            }],
        ),
        activated!(
            cost!("{1}"),
            &[Effect::PumpFilter {
                filter: &Filter::This,
                controlled_by: None,
                power: Amount::Fixed(0),
                toughness: Amount::Fixed(1),
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn,
            }],
        ),
        activated!(
            cost!("{3}"),
            &[Effect::DealDamage {
                amount: Amount::Fixed(1),
                target: TargetSpec::Object(&NONFLYING_ATTACKER_OF_YOU),
            }],
            target = Some(TargetSpec::Object(&NONFLYING_ATTACKER_OF_YOU)),
        ),
    ],
);
