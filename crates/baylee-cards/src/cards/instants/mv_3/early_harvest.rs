//! Early Harvest — {1}{G}{G} — Instant
//! Oracle: Target player untaps all basic lands they control.
//! Set: 9ED #235 — Ninth Edition | Scryfall ID: 22113d08-ecc9-4efd-90f1-00d699582ec3 | Oracle ID: 32001382-3079-4bbe-8d86-05da654c0d1a
// PARTIAL — untaps all basic lands you control; targeting a player to untap basic lands they control requires an UntapAllOf effect.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::EARLY_HARVEST,
    oracle_id = "32001382-3079-4bbe-8d86-05da654c0d1a",
    scryfall_id = "22113d08-ecc9-4efd-90f1-00d699582ec3",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "untaps all basic lands you control; targeting a player to untap all basic lands they control requires an UntapAllOf effect",
    ),
    faces = &[face!(
        name = "Early Harvest",
        mana_cost = mana!("{1}{G}{G}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(&[Effect::UntapAll {
        filter: &Filter::YOUR_BASIC_LAND,
    }])],
);
