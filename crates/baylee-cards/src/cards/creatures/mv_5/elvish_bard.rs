//! Elvish Bard — {3}{G}{G} — Creature — Elf Shaman Bard
//! Oracle: All creatures able to block this creature do so.
//! Set: 9ED #236 — Ninth Edition | Scryfall ID: e2fda362-63ee-4f0b-8faf-a9073e0d5fa0 | Oracle ID: 094b778f-95a0-436f-a9d0-20a46a674486

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ELVISH_BARD,
    oracle_id = "094b778f-95a0-436f-a9d0-20a46a674486",
    scryfall_id = "e2fda362-63ee-4f0b-8faf-a9073e0d5fa0",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Elvish Bard",
        mana_cost = mana!("{3}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[
            subtypes::creature::ELF,
            subtypes::creature::SHAMAN,
            subtypes::creature::BARD,
        ],
        power = Some(2),
        toughness = Some(4),
    ),],
    abilities = &[static_ability!(
        Filter::This,
        Modifier::MustBeBlockedByAllAble
    )],
);
