//! Keeper of the Beasts — {G}{G} — Creature — Human Wizard
//! Oracle: {G}, {T}: Choose target opponent who controls more creatures than you do as you activate this ability. Create a 2/2 green Beast creature token.
//! Set: EXO #112 — Exodus | Scryfall ID: cccccf78-8b00-406b-a2b7-0e6ba76703d0 | Oracle ID: 86dcdffd-c14f-4e06-a74f-d2c727f55b1c
// PARTIAL — Targeting only an opponent with more creatures is unsupported; the activated ability is omitted.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::KEEPER_OF_THE_BEASTS,
    oracle_id = "86dcdffd-c14f-4e06-a74f-d2c727f55b1c",
    scryfall_id = "cccccf78-8b00-406b-a2b7-0e6ba76703d0",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "Targeting only an opponent with more creatures is unsupported; the activated ability is omitted."
    ),
    faces = &[face!(
        name = "Keeper of the Beasts",
        mana_cost = mana!("{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(2),
    ),],
    // NOT SUPPORTED: Targeting only an opponent with more creatures is unsupported; the activated ability is omitted.
    abilities = &[],
);
