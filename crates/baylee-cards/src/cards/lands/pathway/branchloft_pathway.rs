//! Branchloft Pathway // Boulderloft Pathway — (no cost) — Land // Land
//! Oracle: {T}: Add {G}.
//! Oracle: {T}: Add {W}.
//! Set: ZNR #258 — Zendikar Rising | Scryfall ID: 0511e232-2a72-40f5-a400-4f7ebc442d17 | Oracle ID: 7c304547-a4b1-46c9-baed-16d2bfbe16eb
//! Face: Branchloft Pathway —  — Land
//! Face: Boulderloft Pathway —  — Land
// IMPLEMENTED — MDFC land-face choice on play (CR 712.12) + per-face mana
// abilities: {T}: Add {G} on the front, {T}: Add {W} on the back.

use baylee_cards_dsl::prelude::*;

static BACK_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::White, 1)])];

card!(
    index = index::BRANCHLOFT_PATHWAY,
    oracle_id = "7c304547-a4b1-46c9-baed-16d2bfbe16eb",
    scryfall_id = "0511e232-2a72-40f5-a400-4f7ebc442d17",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    faces = &[
        face!(name = "Branchloft Pathway", types = TypeSet::LAND,),
        face!(
            name = "Boulderloft Pathway",
            types = TypeSet::LAND,
            abilities = BACK_MANA,
        ),
    ],
    coverage = Coverage::Implemented,
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Green, 1)])],
);
