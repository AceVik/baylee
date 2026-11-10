//! Goblin Caves — {1}{R}{R} — Enchantment — Aura
//! Oracle: Enchant land
//! Oracle: As long as enchanted land is a basic Mountain, Goblin creatures get +0/+2.
//! Set: ME4 #119 — Masters Edition IV | Scryfall ID: bcba2255-b0ae-43b9-b9fd-62da04d5e397 | Oracle ID: 3ab5c3a7-12e7-4394-a495-0bc7c310bf9c

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const BASIC_MOUNTAIN: Filter = Filter::And(&[
    Filter::HasSupertype(SupertypeSet::BASIC),
    Filter::HasSubtype(subtypes::land::MOUNTAIN),
]);

const GOBLIN_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasSubtype(subtypes::creature::GOBLIN),
]);

card!(
    index = index::GOBLIN_CAVES,
    oracle_id = "3ab5c3a7-12e7-4394-a495-0bc7c310bf9c",
    scryfall_id = "bcba2255-b0ae-43b9-b9fd-62da04d5e397",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Goblin Caves",
        mana_cost = mana!("{1}{R}{R}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::LAND),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::LAND))),
        ),
        static_ability!(
            GOBLIN_CREATURE,
            Modifier::ModifyPT(0, 2),
            condition = Some(Condition::AttachedMatches(&BASIC_MOUNTAIN)),
        ),
    ],
);
