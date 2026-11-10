//! Cinder Crawler — {1}{R} — Creature — Salamander
//! Oracle: {R}: This creature gets +1/+0 until end of turn. Activate only if this creature is blocked.
//! Set: EXO #80 — Exodus | Scryfall ID: a9406050-d76b-4569-a463-e21acaf84166 | Oracle ID: 3850479f-1c1e-4a72-ae47-6da2997202e2
// PARTIAL — Checking whether this creature is blocked after all blockers have left combat is not supported; requires at least one creature currently blocking this creature on the battlefield.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// NOT SUPPORTED: Checking whether this creature is blocked after all blockers have left combat is not supported; requires at least one creature currently blocking this creature on the battlefield.
card!(
    index = index::CINDER_CRAWLER,
    oracle_id = "3850479f-1c1e-4a72-ae47-6da2997202e2",
    scryfall_id = "a9406050-d76b-4569-a463-e21acaf84166",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "Checking whether this creature is blocked after all blockers have left combat is not supported; requires at least one creature currently blocking this creature on the battlefield."
    ),
    faces = &[face!(
        name = "Cinder Crawler",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SALAMANDER],
        power = Some(1),
        toughness = Some(2),
    ),],
    abilities = &[activated!(
        cost!("{R}"),
        &[Effect::PumpFilter {
            filter: &Filter::This,
            controlled_by: None,
            power: Amount::Fixed(1),
            toughness: Amount::Fixed(0),
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        }],
        condition = Some(Condition::BattlefieldCount(&Filter::BlockingSource, 1)),
    )],
);
