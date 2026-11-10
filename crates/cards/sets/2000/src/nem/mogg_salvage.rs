//! Mogg Salvage — {2}{R} — Instant
//! Oracle: If an opponent controls an Island and you control a Mountain, you may cast this spell without paying its mana cost.
//! Oracle: Destroy target artifact.
//! Set: MH2 #282 — Modern Horizons 2 | Scryfall ID: 2b93e65e-8a10-444f-8b0a-9327dd30cce2 | Oracle ID: 0923ad85-2ea4-4463-8f71-b7d0a162b486
// PARTIAL — destroy target artifact.
// NOT SUPPORTED: conditional free alternative cost when an opponent controls an Island and you control a Mountain (`AltCondition` has no two-player land-subtype conjunction).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MOGG_SALVAGE,
    oracle_id = "0923ad85-2ea4-4463-8f71-b7d0a162b486",
    scryfall_id = "2b93e65e-8a10-444f-8b0a-9327dd30cce2",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "AltCondition has no two-player land-subtype conjunction for free casting when an opponent controls an Island and you control a Mountain",
    ),
    faces = &[face!(
        name = "Mogg Salvage",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::destroy(TargetSpec::Object(&Filter::ARTIFACT))],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::ARTIFACT)))
    ),],
);
