//! Dauthi Jackal — {2}{B} — Creature — Dauthi Jackal
//! Oracle: Shadow (This creature can block or be blocked by only creatures with shadow.)
//! Oracle: {B}{B}, Sacrifice this creature: Destroy target blocking creature.
//! Set: TPR #95 — Tempest Remastered | Scryfall ID: 37b3667e-c7e5-4304-bc73-5b3ec7ac224a | Oracle ID: 8847b057-8896-4c72-a11f-3c183b1cc60e
// IMPLEMENTED — all printed abilities are represented.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static BLOCKING_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::Blocking]);

card!(
    index = index::DAUTHI_JACKAL,
    oracle_id = "8847b057-8896-4c72-a11f-3c183b1cc60e",
    scryfall_id = "37b3667e-c7e5-4304-bc73-5b3ec7ac224a",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    keywords = KeywordSet::SHADOW,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Dauthi Jackal",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DAUTHI, subtypes::creature::JACKAL],
        power = Some(2),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!("{B}{B}", SacrificeSelf),
        &[Effect::destroy(TargetSpec::Object(&BLOCKING_CREATURE))],
        targets = Some(TargetReq::one(TargetSpec::Object(&BLOCKING_CREATURE))),
    )],
);
