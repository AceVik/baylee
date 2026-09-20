//! Blightstep Pathway // Searstep Pathway — (no cost) — Land // Land
//! Oracle: {T}: Add {B}.
//! Oracle: {T}: Add {R}.
//! Set: KHM #252 — Kaldheim | Scryfall ID: 0ce39a19-f51d-4a35-ae80-5b82eb15fcff | Oracle ID: e580a229-e800-4746-9d37-c32fcef8de28
//! Face: Blightstep Pathway —  — Land
//! Face: Searstep Pathway —  — Land
// IMPLEMENTED — MDFC land-face choice on play (CR 712.12) + per-face
// mana abilities.

use baylee_cards_dsl::prelude::*;

static BACK_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::Red, 1)])];

card!(
    index = index::BLIGHTSTEP_PATHWAY,
    oracle_id = "e580a229-e800-4746-9d37-c32fcef8de28",
    scryfall_id = "0ce39a19-f51d-4a35-ae80-5b82eb15fcff",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    faces = &[
        face!(name = "Blightstep Pathway", types = TypeSet::LAND,),
        face!(
            name = "Searstep Pathway",
            types = TypeSet::LAND,
            abilities = BACK_MANA,
        ),
    ],
    coverage = Coverage::Implemented,
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Black, 1)])],
);
