//! Pygmy Pyrosaur — {1}{R} — Creature — Lizard
//! Oracle: This creature can't block.
//! Oracle: {R}: This creature gets +1/+0 until end of turn.
//! Set: DDI #46 — Duel Decks: Venser vs. Koth | Scryfall ID: 750d62f6-8fc4-4e61-b9d2-b85a4eb7cc61 | Oracle ID: 667c39bc-92d6-4fee-bced-95f0e1f21455
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PYGMY_PYROSAUR,
    oracle_id = "667c39bc-92d6-4fee-bced-95f0e1f21455",
    scryfall_id = "750d62f6-8fc4-4e61-b9d2-b85a4eb7cc61",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Pygmy Pyrosaur",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::LIZARD],
        power = Some(1),
        toughness = Some(1),
        keywords = KeywordSet::CANT_BLOCK,
    ),],
    abilities = &[activated!(
        cost!("{R}"),
        &[Effect::continuous(
            &Filter::This,
            Modifier::ModifyPT(1, 0),
            Duration::UntilEndOfTurn,
        )],
    )],
);
