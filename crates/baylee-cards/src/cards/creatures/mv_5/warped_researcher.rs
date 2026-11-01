//! Warped Researcher — {4}{U} — Creature — Human Wizard Mutant
//! Oracle: Whenever a player cycles a card, this creature gains flying and shroud until end of turn. (It can't be the target of spells or abilities.)
//! Set: LGN #56 — Legions | Scryfall ID: 5df94a4e-1371-4b75-a557-eeb83c23cf9d | Oracle ID: 069862f9-f051-40cc-84f2-c0208873a34c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WARPED_RESEARCHER,
    oracle_id = "069862f9-f051-40cc-84f2-c0208873a34c",
    scryfall_id = "5df94a4e-1371-4b75-a557-eeb83c23cf9d",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Warped Researcher",
        mana_cost = mana!("{4}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[
            subtypes::creature::HUMAN,
            subtypes::creature::WIZARD,
            subtypes::creature::MUTANT
        ],
        power = Some(3),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
