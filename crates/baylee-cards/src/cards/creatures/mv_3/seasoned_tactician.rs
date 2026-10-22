//! Seasoned Tactician — {2}{W} — Creature — Human Advisor
//! Oracle: {3}, Exile the top four cards of your library: The next time a source of your choice would deal damage to you this turn, prevent that damage.
//! Set: ALL #17 — Alliances | Scryfall ID: f8be4b6b-23a2-42d2-911d-fa14f7f5a95b | Oracle ID: d7c2f27c-8c82-4283-a17d-f979ee636542

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SEASONED_TACTICIAN,
    oracle_id = "d7c2f27c-8c82-4283-a17d-f979ee636542",
    scryfall_id = "f8be4b6b-23a2-42d2-911d-fa14f7f5a95b",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Seasoned Tactician",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::ADVISOR],
        power = Some(1),
        toughness = Some(3),
    ),],
    coverage = Coverage::Partial(
        "The {3} activation prevents the next damage a chosen source would deal to you this turn, without requiring you to exile the top four cards of your library as part of the activation cost."
    ),
    abilities = &[activated!(
        cost!("{3}"),
        &[Effect::PreventNextFromChosenSource {
            sources: &Filter::Any,
            combat_only: false,
            all_but: 0,
            gain_life: false,
        }]
    )],
);
