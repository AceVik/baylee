//! Dissipate — {1}{U}{U} — Instant
//! Oracle: Counter target spell. If that spell is countered this way, exile it instead of putting it into its owner's graveyard.
//! Set: MID #49 — Innistrad: Midnight Hunt | Scryfall ID: 4689b3f2-e4b7-448e-b3d4-ab33194aafb2 | Oracle ID: 4b2e9aa9-6f91-41de-84b6-e9a89be06a53
// IMPLEMENTED — counters target spell and exiles it instead of putting it into its owner's graveyard.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DISSIPATE,
    oracle_id = "4b2e9aa9-6f91-41de-84b6-e9a89be06a53",
    scryfall_id = "4689b3f2-e4b7-448e-b3d4-ab33194aafb2",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Dissipate",
        mana_cost = mana!("{1}{U}{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::CounterTargetSpellToExile],
        targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::Any))),
    )],
);
