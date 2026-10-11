//! Marhault Elsdragon — {3}{R}{R}{G} — Legendary Creature — Elf Warrior
//! Oracle: Rampage 1 (Whenever this creature becomes blocked, it gets +1/+1 until end of turn for each creature blocking it beyond the first.)
//! Set: ME3 #161 — Masters Edition III | Scryfall ID: 2e805883-081b-478a-aa58-172b659571c2 | Oracle ID: c202f124-4283-48f6-aef1-470ad9ded22b

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MARHAULT_ELSDRAGON,
    oracle_id = "c202f124-4283-48f6-aef1-470ad9ded22b",
    scryfall_id = "2e805883-081b-478a-aa58-172b659571c2",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Red]),
    commander = CommanderRule::Legendary,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Marhault Elsdragon",
        mana_cost = mana!("{3}{R}{R}{G}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::ELF, subtypes::creature::WARRIOR],
        power = Some(4),
        toughness = Some(6),
    ),],
    abilities = &[triggered!(
        Trigger::Rampage,
        &[Effect::PumpFilter {
            filter: &Filter::This,
            controlled_by: None,
            power: Amount::Times {
                base: &Amount::BlockersBeyondFirst,
                factor: 1
            },
            toughness: Amount::Times {
                base: &Amount::BlockersBeyondFirst,
                factor: 1
            },
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        }]
    ),],
);
