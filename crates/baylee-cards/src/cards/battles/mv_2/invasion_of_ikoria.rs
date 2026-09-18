//! Invasion of Ikoria // Zilortha, Apex of Ikoria — {X}{G}{G} — Battle — Siege // Legendary Creature — Dinosaur
//! Oracle: (As a Siege enters, choose an opponent to protect it. You and others can attack it. When it's defeated, exile it, then cast it transformed.)
//! Oracle: When this Siege enters, search your library and/or graveyard for a non-Human creature card with mana value X or less and put it onto the battlefield. If you search your library this way, shuffle.
//! Oracle: Reach
//! Oracle: For each non-Human creature you control, you may have that creature assign its combat damage as though it weren't blocked.
//! Set: MOM #190 — March of the Machine | Scryfall ID: 5d59c8f2-f6af-40a6-8dfe-8cc45bf231ce | Oracle ID: 21d90735-5e04-4a8c-a546-ff26d4d5998d
//! Face: Invasion of Ikoria — {X}{G}{G} — Battle — Siege
//! Face: Zilortha, Apex of Ikoria —  — Legendary Creature — Dinosaur
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::INVASION_OF_IKORIA,
    oracle_id = "21d90735-5e04-4a8c-a546-ff26d4d5998d",
    scryfall_id = "5d59c8f2-f6af-40a6-8dfe-8cc45bf231ce",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[
        face!(
            name = "Invasion of Ikoria",
            mana_cost = mana!("{X}{G}{G}"),
            types = TypeSet::BATTLE,
            subtypes = &[subtypes::battle::SIEGE],
        ),
        face!(
            name = "Zilortha, Apex of Ikoria",
            types = TypeSet::CREATURE,
            supertypes = SupertypeSet::LEGENDARY,
            subtypes = &[subtypes::creature::DINOSAUR],
            power = Some(8),
            toughness = Some(8),
            castable_from_hand = false,
        ),
    ],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
