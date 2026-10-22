//! Shadow Guildmage — {B} — Creature — Human Wizard
//! Oracle: {U}, {T}: Put target creature you control on top of its owner's library.
//! Oracle: {R}, {T}: This creature deals 1 damage to any target and 1 damage to you.
//! Set: TSB #46 — Time Spiral Timeshifted | Scryfall ID: 960c2137-f2de-46c6-9b63-bbb208d3cfa8 | Oracle ID: cdda63d7-e3ea-455c-b7d4-c6cfdda2703c
// PARTIAL: Putting target creature you control unconditionally on top of its owner's library from the battlefield uses Effect::OwnerPutsOnTopOrBottom, which lets its owner choose top or bottom; the {R}, {T} ability deals 1 damage to any target and 1 damage to you as printed.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SHADOW_GUILDMAGE,
    oracle_id = "cdda63d7-e3ea-455c-b7d4-c6cfdda2703c",
    scryfall_id = "960c2137-f2de-46c6-9b63-bbb208d3cfa8",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red, Color::Blue]),
    coverage = Coverage::Partial(
        "putting target creature you control unconditionally on top of its owner's library from the battlefield uses Effect::OwnerPutsOnTopOrBottom, which lets its owner choose top or bottom; the {R}, {T} ability deals 1 damage to any target and 1 damage to you as printed",
    ),
    faces = &[face!(
        name = "Shadow Guildmage",
        mana_cost = mana!("{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[
        activated!(
            cost!("{U}", TapSelf),
            &[Effect::OwnerPutsOnTopOrBottom {
                target: TargetSpec::Object(&Filter::YOUR_CREATURE),
            }],
            target = Some(TargetSpec::Object(&Filter::YOUR_CREATURE)),
        ),
        activated!(
            cost!("{R}", TapSelf),
            &[
                Effect::DealDamage {
                    amount: Amount::Fixed(1),
                    target: TargetSpec::AnyTarget,
                },
                Effect::DealDamage {
                    amount: Amount::Fixed(1),
                    target: TargetSpec::Player(PlayerRel::You),
                },
            ],
            target = Some(TargetSpec::AnyTarget),
        ),
    ],
);
