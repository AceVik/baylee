//! Darkbore Pathway // Slitherbore Pathway — (no cost) — Land // Land
//! Oracle: {T}: Add {B}.
//! Oracle: {T}: Add {G}.
//! Set: KHM #254 — Kaldheim | Scryfall ID: 87a4e5fe-161f-42da-9ca2-67c8e8970e94 | Oracle ID: 868e6e68-4367-4073-a864-235d5961ae56
//! Face: Darkbore Pathway —  — Land
//! Face: Slitherbore Pathway —  — Land
// IMPLEMENTED — MDFC land-face choice on play (CR 712.12) + per-face
// mana abilities.

use baylee_cards_dsl::prelude::*;

static BACK_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::Green, 1)])];

card!(
    index = index::DARKBORE_PATHWAY,
    oracle_id = "868e6e68-4367-4073-a864-235d5961ae56",
    scryfall_id = "87a4e5fe-161f-42da-9ca2-67c8e8970e94",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green]),
    faces = &[
        face!(name = "Darkbore Pathway", types = TypeSet::LAND,),
        face!(
            name = "Slitherbore Pathway",
            types = TypeSet::LAND,
            abilities = BACK_MANA,
        ),
    ],
    coverage = Coverage::Implemented,
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Black, 1)])],
);
