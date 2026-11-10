//! Apocalypse — {2}{R}{R}{R} — Sorcery
//! Oracle: Exile all permanents. You discard your hand.
//! Set: TMP #162 — Tempest | Scryfall ID: 7ff23780-d183-4cca-ad0c-448ef325bf36 | Oracle ID: 82c8f5dd-563d-4fd0-bd43-7ee2001d3777
// IMPLEMENTED — Exiles all permanents, then you discard your hand.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::APOCALYPSE,
    oracle_id = "82c8f5dd-563d-4fd0-bd43-7ee2001d3777",
    scryfall_id = "7ff23780-d183-4cca-ad0c-448ef325bf36",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Apocalypse",
        mana_cost = mana!("{2}{R}{R}{R}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[
        Effect::ExileAll {
            filter: &Filter::Any,
            linked: false,
        },
        Effect::DiscardHand {
            who: PlayerRel::You,
        },
    ])],
);
