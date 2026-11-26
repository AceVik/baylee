//! Planar Guide — {W} — Creature — Human Cleric
//! Oracle: {3}{W}, Exile this creature: Exile all creatures. At the beginning of the next end step, return those cards to the battlefield under their owners' control.
//! Set: LGN #18 — Legions | Scryfall ID: 7087cb1e-f2e2-4b75-bacf-bc4153e398e3 | Oracle ID: 75701981-b9fc-471f-a97f-e897bb3dfb77

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PLANAR_GUIDE,
    oracle_id = "75701981-b9fc-471f-a97f-e897bb3dfb77",
    scryfall_id = "7087cb1e-f2e2-4b75-bacf-bc4153e398e3",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Planar Guide",
        mana_cost = mana!("{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!("{3}{W}", ExileSelf),
        &[
            Effect::ExileAll {
                filter: &Filter::CREATURE,
                linked: true,
            },
            Effect::AtNextEndStep {
                effects: &[Effect::ReturnLinkedToBattlefield],
            },
        ],
    )],
);
