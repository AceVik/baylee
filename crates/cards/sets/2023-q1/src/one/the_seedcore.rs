//! The Seedcore — (no cost) — Land — Sphere
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add one mana of any color. Spend this mana only to cast Phyrexian creature spells.
//! Oracle: Corrupted — {T}: Target 1/1 creature gets +2/+1 until end of turn. Activate only if an opponent has three or more poison counters.
//! Set: ONE #259 — Phyrexia: All Will Be One | Scryfall ID: 29c91aad-bf33-448e-b122-65940fb2e33b | Oracle ID: 249fdd3e-376c-4ec2-a612-4353e0e61ee2

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static PHYREXIAN_CREATURE_SPELL: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasSubtype(subtypes::creature::PHYREXIAN),
]);

card!(
    index = index::THE_SEEDCORE,
    oracle_id = "249fdd3e-376c-4ec2-a612-4353e0e61ee2",
    scryfall_id = "29c91aad-bf33-448e-b122-65940fb2e33b",
    faces = &[face!(
        name = "The Seedcore",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::SPHERE],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(&[
            Effect::mana_of_any_color().restricted(&PHYREXIAN_CREATURE_SPELL, SpendRider::None)
        ]),
        activated!(
            cost!(TapSelf),
            &[Effect::PumpTarget {
                power: Amount::Fixed(2),
                toughness: Amount::Fixed(1),
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn
            }],
            target = Some(TargetSpec::Object(&Filter::And(&[
                Filter::CREATURE,
                Filter::PowerAtLeast(1),
                Filter::PowerAtMost(1),
                Filter::ToughnessAtLeast(1),
                Filter::ToughnessAtMost(1)
            ]))),
            condition = Some(Condition::OpponentPoisonAtLeast(3))
        ),
    ],
);
