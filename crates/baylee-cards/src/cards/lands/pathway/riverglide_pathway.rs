//! Riverglide Pathway // Lavaglide Pathway — (no cost) — Land // Land
//! Oracle: {T}: Add {U}.
//! Oracle: {T}: Add {R}.
//! Set: ZNR #264 — Zendikar Rising | Scryfall ID: 2668ac91-6cda-4f81-a08d-4fc5f9cb35b2 | Oracle ID: 4924b3a4-a218-4783-8a4d-82361fdecc78
//! Face: Riverglide Pathway —  — Land
//! Face: Lavaglide Pathway —  — Land
// IMPLEMENTED — MDFC land-face choice on play (CR 712.12) + per-face mana
// abilities: the front taps for {U}, the back for {R}.

use baylee_cards_dsl::prelude::*;

static BACK_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::Red, 1)])];

card!(
    index = index::RIVERGLIDE_PATHWAY,
    oracle_id = "4924b3a4-a218-4783-8a4d-82361fdecc78",
    scryfall_id = "2668ac91-6cda-4f81-a08d-4fc5f9cb35b2",
    color_identity = ColorSet::from_slice(&[Color::Red, Color::Blue]),
    faces = &[
        face!(name = "Riverglide Pathway", types = TypeSet::LAND,),
        face!(
            name = "Lavaglide Pathway",
            types = TypeSet::LAND,
            abilities = BACK_MANA,
        ),
    ],
    coverage = Coverage::Implemented,
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Blue, 1)])],
);
