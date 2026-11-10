//! Herald of War — {3}{W}{W} — Creature — Angel
//! Oracle: Flying
//! Oracle: Whenever this creature attacks, put a +1/+1 counter on it.
//! Oracle: Angel spells and Human spells you cast cost {1} less to cast for each +1/+1 counter on this creature.
//! Set: MIC #86 — Midnight Hunt Commander | Scryfall ID: 18f7c44c-a21f-4b61-8f08-122a75accb00 | Oracle ID: 1228bd6f-565d-4ad5-bd24-e5aa1ead5dee
// IMPLEMENTED — attack counter and counter-scaled Angel/Human cost reduction.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ANGEL_OR_HUMAN_SPELL: Filter = Filter::And(&[
    Filter::Or(&[
        Filter::HasSubtype(subtypes::creature::ANGEL),
        Filter::HasSubtype(subtypes::creature::HUMAN),
    ]),
    Filter::ControlledByYou,
]);

card!(
    index = index::HERALD_OF_WAR,
    oracle_id = "1228bd6f-565d-4ad5-bd24-e5aa1ead5dee",
    scryfall_id = "18f7c44c-a21f-4b61-8f08-122a75accb00",
    color_identity = ColorSet::from_slice(&[Color::White]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Herald of War",
        mana_cost = mana!("{3}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ANGEL],
        power = Some(3),
        toughness = Some(3),
    ),],
    abilities = &[
        triggered!(
            Trigger::Attacks(&Filter::This),
            &[Effect::AddCounterSource {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(1),
            }]
        ),
        static_ability!(
            ANGEL_OR_HUMAN_SPELL,
            Modifier::SpellsCostLessPerCounter(CounterKind::P1P1)
        ),
    ],
);
