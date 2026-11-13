//! Grasping Shadows // Shadows' Lair — {3}{B} — Enchantment // Land — Cave
//! Oracle: Whenever a creature you control attacks alone, it gains deathtouch and lifelink until end of turn. Put a dread counter on this enchantment. Then if there are three or more dread counters on it, transform it.
//! Oracle: (Transforms from Grasping Shadows.)
//! Oracle: {T}: Add {B}.
//! Oracle: {B}, {T}, Remove a dread counter from this land: You draw a card and you lose 1 life.
//! Set: LCI #108 — The Lost Caverns of Ixalan | Scryfall ID: 81b8b9c9-725d-476d-a3cf-55e3dc3e433d | Oracle ID: 522a4b02-24c7-45d2-9097-2803cc9fffad
//! Face: Grasping Shadows — {3}{B} — Enchantment
//! Face: Shadows' Lair —  — Land — Cave

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static BACK_ABILITIES: &[AbilityDef] = &[
    mana_ability!(&[Effect::mana(ManaColor::Black, 1)]),
    activated!(
        cost!(
            "{B}",
            TapSelf,
            RemoveCounterSelf {
                kind: counters::DREAD,
                n: 1
            }
        ),
        &[
            Effect::draw(1),
            Effect::LoseLife {
                amount: Amount::Fixed(1),
                target: PlayerRel::You,
            },
        ]
    ),
];

card!(
    index = index::GRASPING_SHADOWS,
    oracle_id = "522a4b02-24c7-45d2-9097-2803cc9fffad",
    scryfall_id = "81b8b9c9-725d-476d-a3cf-55e3dc3e433d",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[
        face!(
            name = "Grasping Shadows",
            mana_cost = mana!("{3}{B}"),
            types = TypeSet::ENCHANTMENT,
        ),
        face!(
            name = "Shadows' Lair",
            castable_from_hand = false,
            types = TypeSet::LAND,
            subtypes = &[subtypes::land::CAVE],
            abilities = BACK_ABILITIES,
        ),
    ],
    coverage = Coverage::Implemented,
    abilities = &[triggered!(
        Trigger::AttacksAlone(&Filter::YOUR_CREATURE),
        &[
            Effect::continuous(
                &Filter::This,
                Modifier::AddKeyword(KeywordSet::DEATHTOUCH.union(KeywordSet::LIFELINK)),
                Duration::UntilEndOfTurn
            ),
            Effect::AddCounterFilter {
                filter: &Filter::This,
                kind: counters::DREAD,
                amount: Amount::Fixed(1),
            },
            Effect::IfCondition {
                condition: Condition::CountersOnSelf(counters::DREAD, 3),
                then: &[Effect::TransformSource],
                otherwise: &[],
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::EventObject)),
    )],
);
