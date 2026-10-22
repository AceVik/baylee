//! Soldevi Digger — {2} — Artifact
//! Oracle: {2}: Put the top card of your graveyard on the bottom of your library.
//! Set: ME2 #221 — Masters Edition II | Scryfall ID: 473d6e22-1184-4d29-a7d1-f6773666ff29 | Oracle ID: b526962e-be8a-4803-b038-57869bb96751

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SOLDEVI_DIGGER,
    oracle_id = "b526962e-be8a-4803-b038-57869bb96751",
    scryfall_id = "473d6e22-1184-4d29-a7d1-f6773666ff29",
    faces = &[face!(
        name = "Soldevi Digger",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Partial(
        "The {2} ability puts all cards in your graveyard on the bottom of your library rather than only the top card of your graveyard."
    ),
    abilities = &[activated!(
        cost!("{2}"),
        &[Effect::GraveyardToLibraryBottom {
            player: PlayerRel::You,
        }]
    )],
);
