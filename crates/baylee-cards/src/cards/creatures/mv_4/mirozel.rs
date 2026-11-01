//! Mirozel — {3}{U} — Creature — Illusion
//! Oracle: Flying
//! Oracle: When this creature becomes the target of a spell or ability, return this creature to its owner's hand.
//! Set: EXO #41 — Exodus | Scryfall ID: 16417e94-e33f-4ed4-bb3e-52f29f7d441b | Oracle ID: c35111ce-4325-4547-b954-08275a268060
// IMPLEMENTED — all printed abilities are represented.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MIROZEL,
    oracle_id = "c35111ce-4325-4547-b954-08275a268060",
    scryfall_id = "16417e94-e33f-4ed4-bb3e-52f29f7d441b",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Mirozel",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ILLUSION],
        power = Some(2),
        toughness = Some(3),
    ),],
    abilities = &[triggered!(
        Trigger::BecomesTarget,
        &[Effect::bounce(TargetSpec::ThisObject)],
    )],
);
