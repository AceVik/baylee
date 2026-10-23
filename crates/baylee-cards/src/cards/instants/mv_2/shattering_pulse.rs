//! Shattering Pulse — {1}{R} — Instant
//! Oracle: Buyback {3} (You may pay an additional {3} as you cast this spell. If you do, put this card into your hand as it resolves.)
//! Oracle: Destroy target artifact.
//! Set: EXO #102 — Exodus | Scryfall ID: 89d3b846-6071-4d65-86ba-da08c4bd0aa1 | Oracle ID: 77953b7a-813e-45f5-ad7f-ab72a79a0112
// PARTIAL — Destroys target artifact; Buyback {3} is not supported.

use baylee_cards_dsl::prelude::*;

// NOT SUPPORTED: Destroys target artifact; Buyback {3} is not supported.
card!(
    index = index::SHATTERING_PULSE,
    oracle_id = "77953b7a-813e-45f5-ad7f-ab72a79a0112",
    scryfall_id = "89d3b846-6071-4d65-86ba-da08c4bd0aa1",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial("Destroys target artifact; Buyback {3} is not supported."),
    faces = &[face!(
        name = "Shattering Pulse",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::destroy(TargetSpec::Object(&Filter::ARTIFACT))],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::ARTIFACT)))
    ),],
);
