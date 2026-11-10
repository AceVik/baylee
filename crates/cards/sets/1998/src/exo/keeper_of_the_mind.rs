//! Keeper of the Mind — {U}{U} — Creature — Human Wizard
//! Oracle: {U}, {T}: Choose target opponent who has at least two more cards in hand than you do as you activate this ability. Draw a card.
//! Set: EXO #36 — Exodus | Scryfall ID: 7bc232d4-ab4f-4d88-a9ec-72403d05ec04 | Oracle ID: 45393fba-b1cc-4db7-b7e0-3851bf421a65
// PARTIAL — Targeting only an opponent with at least two more cards in hand is unsupported; the activated ability is omitted.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::KEEPER_OF_THE_MIND,
    oracle_id = "45393fba-b1cc-4db7-b7e0-3851bf421a65",
    scryfall_id = "7bc232d4-ab4f-4d88-a9ec-72403d05ec04",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "Targeting only an opponent with at least two more cards in hand is unsupported; the activated ability is omitted."
    ),
    faces = &[face!(
        name = "Keeper of the Mind",
        mana_cost = mana!("{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(2),
    ),],
    // NOT SUPPORTED: Targeting only an opponent with at least two more cards in hand is unsupported; the activated ability is omitted.
    abilities = &[],
);
