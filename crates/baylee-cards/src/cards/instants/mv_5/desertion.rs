//! Desertion — {3}{U}{U} — Instant
//! Oracle: Counter target spell. If an artifact or creature spell is countered this way, put that card onto the battlefield under your control instead of into its owner's graveyard.
//! Set: CN2 #107 — Conspiracy: Take the Crown | Scryfall ID: b362151e-4ad6-4ae9-91e5-0e4a69d53ca5 | Oracle ID: ee3d1f44-e0ca-4ce9-be76-4b675a115156
// PARTIAL — counters target spell; no replacement effect putting an artifact or creature spell countered this way onto the battlefield under your control instead of into its owner's graveyard exists yet.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DESERTION,
    oracle_id = "ee3d1f44-e0ca-4ce9-be76-4b675a115156",
    scryfall_id = "b362151e-4ad6-4ae9-91e5-0e4a69d53ca5",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "counters target spell; no replacement effect putting an artifact or creature spell countered this way onto the battlefield under your control instead of into its owner's graveyard exists yet",
    ),
    faces = &[face!(
        name = "Desertion",
        mana_cost = mana!("{3}{U}{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::CounterTargetSpell],
        targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::Any))),
    )],
);
