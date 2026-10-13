//! Maze's End — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {C}.
//! Oracle: {3}, {T}, Return this land to its owner's hand: Search your library for a Gate card, put it onto the battlefield, then shuffle. If you control ten or more Gates with different names, you win the game.
//! Set: FDN #727 — Foundations | Scryfall ID: ea9a4d1a-79dd-4b15-8e3b-f111f16d6bfc | Oracle ID: 49479778-c4c0-43ba-a7b7-45f00d067462
// IMPLEMENTED — enters tapped, {T}: Add {C}, and the {3} self-bounce Gate
// tutor, then you win the game if you control ten Gates with different names.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

/// "Gates you control" (`ControlDistinctNames` counts the controller's
/// permanents; the filter says so too, as the printed text does).
static YOUR_GATE: Filter = Filter::And(&[Filter::HasSubtype(land::GATE), Filter::ControlledByYou]);

card!(
    index = index::MAZE_S_END,
    oracle_id = "49479778-c4c0-43ba-a7b7-45f00d067462",
    scryfall_id = "ea9a4d1a-79dd-4b15-8e3b-f111f16d6bfc",
    faces = &[face!(
        name = "Maze's End",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{3}", TapSelf, ReturnSelfToHand),
            &[
                Effect::SearchLibrary {
                    filter: &Filter::HasSubtype(land::GATE),
                    finds: &[Find::BATTLEFIELD],
                    optional: false,
                },
                // "If you control ten or more Gates with different names, you
                // win the game." Read after the search, even if it found nothing.
                Effect::IfCondition {
                    condition: Condition::ControlDistinctNames(&YOUR_GATE, 10),
                    then: &[Effect::WinGame],
                    otherwise: &[],
                },
            ]
        ),
    ],
);
