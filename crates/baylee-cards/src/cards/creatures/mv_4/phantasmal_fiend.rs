//! Phantasmal Fiend — {3}{B} — Creature — Illusion
//! Oracle: {B}: This creature gets +1/-1 until end of turn.
//! Oracle: {1}{U}: Switch this creature's power and toughness until end of turn.
//! Set: ME2 #108 — Masters Edition II | Scryfall ID: 98394059-4d87-4583-b1c2-9f74334e1567 | Oracle ID: 821ce349-5379-4415-89e2-224e21dda24b
// IMPLEMENTED — {B} +1/-1 pump until end of turn and {1}{U} power/toughness switch until end of turn.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PHANTASMAL_FIEND,
    oracle_id = "821ce349-5379-4415-89e2-224e21dda24b",
    scryfall_id = "98394059-4d87-4583-b1c2-9f74334e1567",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Phantasmal Fiend",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ILLUSION],
        power = Some(1),
        toughness = Some(5),
    ),],
    abilities = &[
        activated!(
            cost!("{B}"),
            &[Effect::continuous(
                &Filter::This,
                Modifier::ModifyPT(1, -1),
                Duration::UntilEndOfTurn,
            )],
        ),
        activated!(
            cost!("{1}{U}"),
            &[Effect::continuous(
                &Filter::This,
                Modifier::SwitchPT,
                Duration::UntilEndOfTurn,
            )],
        ),
    ],
);
