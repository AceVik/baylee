//! Evil Eye of Orms-by-Gore — {4}{B} — Creature — Eye
//! Oracle: Non-Eye creatures you control can't attack.
//! Oracle: This creature can't be blocked except by Walls.
//! Set: DMR #83 — Dominaria Remastered | Scryfall ID: e3b3dbd4-7c3d-49df-98b0-068db5399083 | Oracle ID: 3f79780a-accd-4782-94a8-a6e71fb3ada7
// IMPLEMENTED — non-Eye creatures you control can't attack, and the Eye can
// only be blocked by Walls.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::EVIL_EYE_OF_ORMS_BY_GORE,
    oracle_id = "3f79780a-accd-4782-94a8-a6e71fb3ada7",
    scryfall_id = "e3b3dbd4-7c3d-49df-98b0-068db5399083",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Evil Eye of Orms-by-Gore",
        mana_cost = mana!("{4}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::EYE],
        power = Some(3),
        toughness = Some(6),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        // The restriction is a restriction, so the Eye itself — an Eye —
        // keeps attacking while every other creature you control does not.
        static_ability!(
            Filter::And(&[
                Filter::CREATURE,
                Filter::ControlledByYou,
                Filter::Not(&Filter::HasSubtype(subtypes::creature::EYE)),
            ]),
            Modifier::AddKeyword(KeywordSet::CANT_ATTACK)
        ),
        // "except by Walls" is `CantBeBlockedBy` reading the blocker:
        // everything that is a creature and not a Wall (Invisibility's
        // shape).
        static_ability!(
            Filter::This,
            Modifier::CantBeBlockedBy(&Filter::And(&[
                Filter::CREATURE,
                Filter::Not(&Filter::HasSubtype(subtypes::creature::WALL)),
            ]))
        ),
    ],
);
