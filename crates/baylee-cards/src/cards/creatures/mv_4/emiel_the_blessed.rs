//! Emiel the Blessed — {2}{W}{W} — Legendary Creature — Unicorn
//! Oracle: {3}: Exile another target creature you control, then return it to the battlefield under its owner's control.
//! Oracle: Whenever another creature you control enters, you may pay {G/W}. If you do, put a +1/+1 counter on it. If it's a Unicorn, put two +1/+1 counters on it instead. ({G/W} can be paid with either {G} or {W}.)
//! Set: 2X2 #10 — Double Masters 2022 | Scryfall ID: 0f594562-7e9f-47e6-a033-fb70e3cf1e10 | Oracle ID: b11c250c-f191-4c52-ba02-a9176f163447
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::EMIEL_THE_BLESSED,
    oracle_id = "b11c250c-f191-4c52-ba02-a9176f163447",
    scryfall_id = "0f594562-7e9f-47e6-a033-fb70e3cf1e10",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Emiel the Blessed",
        mana_cost = mana!("{2}{W}{W}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::UNICORN],
        power = Some(4),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
