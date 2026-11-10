//! Council's Judgment — {1}{W}{W} — Sorcery
//! Oracle: Will of the council — Starting with you, each player votes for a nonland permanent you don't control. Exile each permanent with the most votes or tied for most votes.
//! Set: OTC #79 — Outlaws of Thunder Junction Commander | Scryfall ID: 4005e82f-0b30-4165-87e2-51250c00adb0 | Oracle ID: acf4d685-a718-42f7-9a7a-dee069ae8db2
// IMPLEMENTED — sequential nonland vote, including every permanent tied for most.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::COUNCIL_S_JUDGMENT,
    oracle_id = "acf4d685-a718-42f7-9a7a-dee069ae8db2",
    scryfall_id = "4005e82f-0b30-4165-87e2-51250c00adb0",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Council's Judgment",
        mana_cost = mana!("{1}{W}{W}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[Effect::ExileMostVotedNotControlledByYou {
        filter: &Filter::NONLAND,
    },])],
);
