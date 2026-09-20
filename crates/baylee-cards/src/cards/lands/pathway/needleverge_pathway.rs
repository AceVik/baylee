//! Needleverge Pathway // Pillarverge Pathway — (no cost) — Land // Land
//! Oracle: {T}: Add {R}.
//! Oracle: {T}: Add {W}.
//! Set: ZNR #263 — Zendikar Rising | Scryfall ID: 6559047e-6ede-4815-a3a0-389062094f9d | Oracle ID: a9b8d020-4d72-4934-8942-df29ef19fc1d
//! Face: Needleverge Pathway —  — Land
//! Face: Pillarverge Pathway —  — Land
// IMPLEMENTED — MDFC land-face choice on play (CR 712.12) + per-face
// mana abilities ({R} front, {W} back).

use baylee_cards_dsl::prelude::*;

static BACK_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::White, 1)])];

card!(
    index = index::NEEDLEVERGE_PATHWAY,
    oracle_id = "a9b8d020-4d72-4934-8942-df29ef19fc1d",
    scryfall_id = "6559047e-6ede-4815-a3a0-389062094f9d",
    color_identity = ColorSet::from_slice(&[Color::Red, Color::White]),
    faces = &[
        face!(name = "Needleverge Pathway", types = TypeSet::LAND,),
        face!(
            name = "Pillarverge Pathway",
            types = TypeSet::LAND,
            abilities = BACK_MANA,
        ),
    ],
    coverage = Coverage::Implemented,
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Red, 1)])],
);
