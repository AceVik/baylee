//! Raging Gorilla — {2}{R} — Creature — Ape
//! Oracle: Whenever this creature blocks or becomes blocked, it gets +2/-2 until end of turn.
//! Set: VIS #90 — Visions | Scryfall ID: 07c284ce-33b8-4fb2-9dd9-4c477bedc774 | Oracle ID: 1dd6df3f-eac4-4f30-b832-4a3f3a97632f
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RAGING_GORILLA,
    oracle_id = "1dd6df3f-eac4-4f30-b832-4a3f3a97632f",
    scryfall_id = "07c284ce-33b8-4fb2-9dd9-4c477bedc774",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Raging Gorilla",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::APE],
        power = Some(2),
        toughness = Some(3),
    ),],
    abilities = &[triggered!(
        Trigger::BlocksOrBecomesBlockedByOneOrMore(&Filter::CREATURE),
        &[Effect::ModifySource {
            modifier: Modifier::ModifyPT(2, -2),
            duration: Duration::UntilEndOfTurn,
        }],
    )],
);
