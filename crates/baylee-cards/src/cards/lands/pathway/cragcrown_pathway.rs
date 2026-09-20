//! Cragcrown Pathway // Timbercrown Pathway — (no cost) — Land // Land
//! Oracle: {T}: Add {R}.
//! Oracle: {T}: Add {G}.
//! Set: ZNR #261 — Zendikar Rising | Scryfall ID: da57eb54-5199-4a56-95f7-f6ac432876b1 | Oracle ID: 727ca426-f4cc-4218-8ae5-8c427af2e816
//! Face: Cragcrown Pathway —  — Land
//! Face: Timbercrown Pathway —  — Land
// IMPLEMENTED — MDFC land-face choice on play (CR 712.12) + per-face mana
// abilities: {R} on the front, {G} on the back.

use baylee_cards_dsl::prelude::*;

static BACK_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::Green, 1)])];

card!(
    index = index::CRAGCROWN_PATHWAY,
    oracle_id = "727ca426-f4cc-4218-8ae5-8c427af2e816",
    scryfall_id = "da57eb54-5199-4a56-95f7-f6ac432876b1",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Red]),
    faces = &[
        face!(name = "Cragcrown Pathway", types = TypeSet::LAND,),
        face!(
            name = "Timbercrown Pathway",
            types = TypeSet::LAND,
            abilities = BACK_MANA,
        ),
    ],
    coverage = Coverage::Implemented,
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Red, 1)])],
);
