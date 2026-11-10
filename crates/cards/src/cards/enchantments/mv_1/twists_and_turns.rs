//! Twists and Turns // Mycoid Maze — {G} — Enchantment // Land — Cave
//! Oracle: If a creature you control would explore, instead you scry 1, then that creature explores.
//! Oracle: When this enchantment enters, target creature you control explores.
//! Oracle: When a land you control enters, if you control seven or more lands, transform this enchantment.
//! Oracle: (Transforms from Twists and Turns.)
//! Oracle: {T}: Add {G}.
//! Oracle: {3}{G}, {T}: Look at the top four cards of your library. You may reveal a creature card from among them and put that card into your hand. Put the rest on the bottom of your library in a random order.
//! Set: LCI #217 — The Lost Caverns of Ixalan | Scryfall ID: 3cdf691e-96a5-45c7-9b94-6f04af81c8e4 | Oracle ID: 740aa9d9-91a9-431e-8bf9-1344e5273e27
//! Face: Twists and Turns — {G} — Enchantment
//! Face: Mycoid Maze —  — Land — Cave
// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static MYCOID_MAZE_ABILITIES: &[AbilityDef] = &[
    mana_ability!(&[Effect::mana(ManaColor::Green, 1)]),
    activated!(
        cost!("{3}{G}", TapSelf),
        &[Effect::LookAtTopFind {
            found: Find::HAND,
            count: 4,
            filter: &Filter::CREATURE,
            random: true,
        }]
    ),
];

card!(
    index = index::TWISTS_AND_TURNS,
    oracle_id = "740aa9d9-91a9-431e-8bf9-1344e5273e27",
    scryfall_id = "3cdf691e-96a5-45c7-9b94-6f04af81c8e4",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[
        face!(
            name = "Twists and Turns",
            mana_cost = mana!("{G}"),
            types = TypeSet::ENCHANTMENT,
        ),
        face!(
            name = "Mycoid Maze",
            types = TypeSet::LAND,
            subtypes = &[subtypes::land::CAVE],
            castable_from_hand = false,
            abilities = MYCOID_MAZE_ABILITIES,
        ),
    ],
    abilities = &[
        AbilityDef::Replacement(ReplacementRule::ExploreScry1First),
        triggered!(
            Trigger::ETB,
            &[Effect::Explore {
                target: TargetSpec::Object(&Filter::YOUR_CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::YOUR_CREATURE))),
        ),
        triggered!(
            Trigger::EntersBattlefield(&Filter::YOUR_LAND),
            &[Effect::TransformSource],
            condition = Some(Condition::ControlCount(&Filter::LAND, 7)),
        ),
    ],
);
