//! Elvish Piper — {3}{G} — Creature — Elf Shaman
//! Oracle: {G}, {T}: You may put a creature card from your hand onto the battlefield.
//! Set: A25 #168 — Masters 25 | Scryfall ID: 1a8e47b9-6735-4e46-b3fd-ac69edc39745 | Oracle ID: 872ef617-7cdb-4a7a-85f5-efc9e12bece5
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ELVISH_PIPER,
    oracle_id = "872ef617-7cdb-4a7a-85f5-efc9e12bece5",
    scryfall_id = "1a8e47b9-6735-4e46-b3fd-ac69edc39745",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Elvish Piper",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELF, subtypes::creature::SHAMAN],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!("{G}", TapSelf),
        &[Effect::PutFromHandOntoBattlefield {
            tapped: false,
            filter: &Filter::CREATURE,
            mana_value: None,
            optional: true,
        }],
    )],
);
