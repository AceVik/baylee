//! Deranged Hermit — {3}{G}{G} — Creature — Elf
//! Oracle: Echo {3}{G}{G} (At the beginning of your upkeep, if this came under your control since the beginning of your last upkeep, sacrifice it unless you pay its echo cost.)
//! Oracle: When this creature enters, create four 1/1 green Squirrel creature tokens.
//! Oracle: Squirrel creatures get +1/+1.
//! Set: VMA #202 — Vintage Masters | Scryfall ID: c25df202-0dd4-448d-8cb3-11fd7df0d505 | Oracle ID: b9cd714b-2ad8-4fdb-a8aa-82b17730e071
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const SQUIRREL_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasSubtype(subtypes::creature::SQUIRREL),
]);

card!(
    index = index::DERANGED_HERMIT,
    oracle_id = "b9cd714b-2ad8-4fdb-a8aa-82b17730e071",
    scryfall_id = "c25df202-0dd4-448d-8cb3-11fd7df0d505",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Deranged Hermit",
        mana_cost = mana!("{3}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELF],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[
        AbilityDef::Echo {
            cost: mana!("{3}{G}{G}"),
        },
        triggered!(
            Trigger::ETB,
            &[Effect::CreateTokenN {
                token: &crate::generated_tokens::SQUIRREL_1_1_GREEN,
                amount: Amount::Fixed(4),
            }],
        ),
        static_ability!(SQUIRREL_CREATURE, Modifier::ModifyPT(1, 1)),
    ],
);
