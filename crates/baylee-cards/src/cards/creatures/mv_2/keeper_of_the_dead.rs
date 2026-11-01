//! Keeper of the Dead — {B}{B} — Creature — Human Wizard
//! Oracle: {B}, {T}: Choose target opponent who has at least two fewer creature cards in their graveyard than you do as you activate this ability. Destroy target nonblack creature that player controls.
//! Set: EXO #65 — Exodus | Scryfall ID: 6b641171-35bc-4945-ada9-3ea28ea9fabf | Oracle ID: 50ddd513-e06e-4728-b6cd-981839d0fba6
// PARTIAL — Targeting only an opponent with at least two fewer creature cards in their graveyard is unsupported; the activated ability is omitted.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::KEEPER_OF_THE_DEAD,
    oracle_id = "50ddd513-e06e-4728-b6cd-981839d0fba6",
    scryfall_id = "6b641171-35bc-4945-ada9-3ea28ea9fabf",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "Targeting only an opponent with at least two fewer creature cards in their graveyard is unsupported; the activated ability is omitted."
    ),
    faces = &[face!(
        name = "Keeper of the Dead",
        mana_cost = mana!("{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(2),
    ),],
    // NOT SUPPORTED: Targeting only an opponent with at least two fewer creature cards in their graveyard is unsupported; the activated ability is omitted.
    abilities = &[],
);
