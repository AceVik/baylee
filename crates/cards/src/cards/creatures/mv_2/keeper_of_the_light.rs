//! Keeper of the Light — {W}{W} — Creature — Human Wizard
//! Oracle: {W}, {T}: Choose target opponent who has more life than you do as you activate this ability. You gain 3 life.
//! Set: EXO #8 — Exodus | Scryfall ID: 06eda847-c599-4163-b48b-aa76b153ed86 | Oracle ID: 62d1300f-2b76-42b9-8e6c-f920712cd547
// PARTIAL — Targeting only an opponent with more life is unsupported; the activated ability is omitted.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::KEEPER_OF_THE_LIGHT,
    oracle_id = "62d1300f-2b76-42b9-8e6c-f920712cd547",
    scryfall_id = "06eda847-c599-4163-b48b-aa76b153ed86",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "Targeting only an opponent with more life is unsupported; the activated ability is omitted."
    ),
    faces = &[face!(
        name = "Keeper of the Light",
        mana_cost = mana!("{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(2),
    ),],
    // NOT SUPPORTED: Targeting only an opponent with more life is unsupported; the activated ability is omitted.
    abilities = &[],
);
