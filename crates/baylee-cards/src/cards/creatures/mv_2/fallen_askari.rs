//! Fallen Askari — {1}{B} — Creature — Human Knight
//! Oracle: Flanking (Whenever a creature without flanking blocks this creature, the blocking creature gets -1/-1 until end of turn.)
//! Oracle: This creature can't block.
//! Set: VMA #119 — Vintage Masters | Scryfall ID: a99c4859-038b-4dd0-80d0-28a881f4a564 | Oracle ID: 4956008e-d912-48ba-9b82-176258281060
// IMPLEMENTED — flanking and can't block.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FALLEN_ASKARI,
    oracle_id = "4956008e-d912-48ba-9b82-176258281060",
    scryfall_id = "a99c4859-038b-4dd0-80d0-28a881f4a564",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    keywords = KeywordSet::FLANKING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Fallen Askari",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::KNIGHT],
        power = Some(2),
        toughness = Some(2),
        keywords = KeywordSet::FLANKING,
    ),],
    abilities = &[static_ability!(
        Filter::This,
        Modifier::CantBlock(&Filter::Any)
    ),],
);
