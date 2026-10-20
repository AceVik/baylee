//! Elkin Bottle — {3} — Artifact
//! Oracle: {3}, {T}: Exile the top card of your library. Until the beginning of your next upkeep, you may play that card.
//! Set: ME2 #207 — Masters Edition II | Scryfall ID: b4bc0c01-06c4-4d84-ac4d-14ec555c9012 | Oracle ID: 1e381361-7404-4654-bfb4-5ea65f6e4dce
// PARTIAL: play window expires at the start of your next turn instead of the beginning of your next upkeep.

use baylee_cards_dsl::prelude::*;

// NOT SUPPORTED: Until the beginning of your next upkeep, you may play that card (PlayWindow::UntilYourNextTurn expires as your next turn begins).
card!(
    index = index::ELKIN_BOTTLE,
    oracle_id = "1e381361-7404-4654-bfb4-5ea65f6e4dce",
    scryfall_id = "b4bc0c01-06c4-4d84-ac4d-14ec555c9012",
    coverage = Coverage::Partial(
        "play window expires at the start of your next turn instead of the beginning of your next upkeep"
    ),
    faces = &[face!(
        name = "Elkin Bottle",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{3}", TapSelf),
        &[Effect::ExileTopToPlay {
            who: PlayerRel::You,
            play: ExilePlay {
                count: 1,
                window: PlayWindow::UntilYourNextTurn,
                free: false,
                mana_any_type: false,
                spell_types: None,
            },
        }],
    )],
);
