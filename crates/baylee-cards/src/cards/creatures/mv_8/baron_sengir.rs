//! Baron Sengir — {5}{B}{B}{B} — Legendary Creature — Vampire Noble
//! Oracle: Flying
//! Oracle: Whenever a creature dealt damage by Baron Sengir this turn dies, put a +2/+2 counter on Baron Sengir.
//! Oracle: {T}: Regenerate another target Vampire.
//! Set: ME1 #58 — Masters Edition | Scryfall ID: e5fb44d7-43c1-4fa2-9d55-aea6154a676d | Oracle ID: fc728cd7-2b5b-45f3-896a-a45648bcb61d
// IMPLEMENTED — flying, +2/+2 counter when a creature dealt damage by Baron Sengir this turn dies, and {T} to regenerate another target Vampire.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ANOTHER_VAMPIRE: Filter = Filter::And(&[
    Filter::Another,
    Filter::HasSubtype(subtypes::creature::VAMPIRE),
]);

card!(
    index = index::BARON_SENGIR,
    oracle_id = "fc728cd7-2b5b-45f3-896a-a45648bcb61d",
    scryfall_id = "e5fb44d7-43c1-4fa2-9d55-aea6154a676d",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    commander = CommanderRule::Legendary,
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Baron Sengir",
        mana_cost = mana!("{5}{B}{B}{B}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::VAMPIRE, subtypes::creature::NOBLE],
        power = Some(5),
        toughness = Some(5),
    ),],
    abilities = &[
        triggered!(
            Trigger::DiesAfterDamageByThis(&Filter::CREATURE),
            &[Effect::AddCounter {
                kind: CounterKind::Plus {
                    power: 2,
                    toughness: 2,
                },
                amount: Amount::Fixed(1),
            }]
        ),
        activated!(
            cost!(TapSelf),
            &[Effect::Regenerate {
                target: TargetSpec::Object(&ANOTHER_VAMPIRE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&ANOTHER_VAMPIRE)))
        ),
    ],
);
