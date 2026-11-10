//! Tireless Provisioner — {2}{G} — Creature — Elf Scout
//! Oracle: Landfall — Whenever a land you control enters, create a Food token or a Treasure token. (Food is an artifact with "{2}, {T}, Sacrifice this token: You gain 3 life." Treasure is an artifact with "{T}, Sacrifice this token: Add one mana of any color.")
//! Set: MOC #313 — March of the Machine Commander | Scryfall ID: a1e048e0-19d2-4076-892d-f8b3104dee37 | Oracle ID: ab8d5f5c-1976-4f77-8ed2-8d28ee666741
// IMPLEMENTED — the landfall trigger as a modal triggered ability: a land
// you control entering offers the Food/Treasure choice (the tokens carry
// their own printed abilities, so the reminder text needs nothing here).

use crate::tokens::{FOOD, TREASURE};
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TIRELESS_PROVISIONER,
    oracle_id = "ab8d5f5c-1976-4f77-8ed2-8d28ee666741",
    scryfall_id = "a1e048e0-19d2-4076-892d-f8b3104dee37",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Tireless Provisioner",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELF, subtypes::creature::SCOUT],
        power = Some(3),
        toughness = Some(2),
    ),],
    abilities = &[modal_triggered!(
        Trigger::EntersBattlefield(&Filter::YOUR_LAND),
        &[
            mode!(&[Effect::CreateToken { token: &FOOD }]),
            mode!(&[Effect::CreateToken { token: &TREASURE }]),
        ]
    )],
);
