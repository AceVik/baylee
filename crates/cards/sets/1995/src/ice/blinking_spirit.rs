//! Blinking Spirit — {3}{W} — Creature — Spirit
//! Oracle: {0}: Return this creature to its owner's hand.
//! Set: 9ED #8 — Ninth Edition | Scryfall ID: 692ac387-be1f-48e0-945e-cbb1254d395c | Oracle ID: a26f6958-9bc8-46f8-8ca7-7f55098e1291

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BLINKING_SPIRIT,
    oracle_id = "a26f6958-9bc8-46f8-8ca7-7f55098e1291",
    scryfall_id = "692ac387-be1f-48e0-945e-cbb1254d395c",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Blinking Spirit",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SPIRIT],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[activated!(
        cost!("{0}"),
        &[Effect::bounce(TargetSpec::ThisObject)]
    )],
);
