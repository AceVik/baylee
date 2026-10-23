//! Early Harvest — {1}{G}{G} — Instant
//! Oracle: Target player untaps all basic lands they control.
//! Set: 9ED #235 — Ninth Edition | Scryfall ID: 22113d08-ecc9-4efd-90f1-00d699582ec3 | Oracle ID: 32001382-3079-4bbe-8d86-05da654c0d1a
// PARTIAL — the spell has no target and untaps only the basic lands its caster controls; no filter says "controlled by the target player", so another player's lands cannot be untapped.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::EARLY_HARVEST,
    oracle_id = "32001382-3079-4bbe-8d86-05da654c0d1a",
    scryfall_id = "22113d08-ecc9-4efd-90f1-00d699582ec3",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "has no player target and untaps only the basic lands its caster controls; no filter says \"controlled by the target player\", so it cannot untap another player's basic lands",
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
