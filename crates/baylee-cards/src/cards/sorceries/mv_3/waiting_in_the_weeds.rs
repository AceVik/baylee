//! Waiting in the Weeds — {1}{G}{G} — Sorcery
//! Oracle: Each player creates a 1/1 green Cat creature token for each untapped Forest they control.
//! Set: 6ED #266 — Classic Sixth Edition | Scryfall ID: 7e2d5a77-6ee8-463d-9e6a-fdb1cc6d70e6 | Oracle ID: 5a61d51e-1a3f-49f6-9a94-a708dd6f69df
// PARTIAL — each player creates a 1/1 green Cat creature token for each untapped Forest they control, exactly in a duel; with three or more players each opponent gets tokens for the untapped Forests of all opponents together, not for their own Forests alone.
// NOT SUPPORTED: "each player … for each untapped Forest they control" per player: no token instruction counts each player's own permanents, so the opponents' Forests are counted as a whole, which is each opponent's own in a duel only.

use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static UNTAPPED_FOREST_YOU_CONTROL: Filter = Filter::And(&[
    Filter::LAND,
    Filter::HasSubtype(subtypes::land::FOREST),
    Filter::Untapped,
    Filter::ControlledByYou,
]);

static UNTAPPED_FOREST_OPPONENTS_CONTROL: Filter = Filter::And(&[
    Filter::LAND,
    Filter::HasSubtype(subtypes::land::FOREST),
    Filter::Untapped,
    Filter::ControlledByOpponent,
]);

card!(
    index = index::WAITING_IN_THE_WEEDS,
    oracle_id = "5a61d51e-1a3f-49f6-9a94-a708dd6f69df",
    scryfall_id = "7e2d5a77-6ee8-463d-9e6a-fdb1cc6d70e6",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "in a game of three or more players each opponent gets Cat tokens for the untapped Forests of all opponents together, not for their own alone (a duel is exact)",
    ),
    faces = &[face!(
        name = "Waiting in the Weeds",
        mana_cost = mana!("{1}{G}{G}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[
        Effect::CreateTokenN {
            token: &generated_tokens::CAT_1_1_GREEN,
            amount: Amount::CountOf {
                filter: &UNTAPPED_FOREST_YOU_CONTROL,
                zone: ZoneSel::Battlefield,
            },
        },
        Effect::CreateTokensFor {
            player: PlayerRel::EachOpponent,
            token: &generated_tokens::CAT_1_1_GREEN,
            amount: Amount::CountOf {
                filter: &UNTAPPED_FOREST_OPPONENTS_CONTROL,
                zone: ZoneSel::Battlefield,
            },
        },
    ])],
);
