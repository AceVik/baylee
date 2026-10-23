//! Debt of Loyalty — {1}{W}{W} — Instant
//! Oracle: Regenerate target creature. You gain control of that creature if it regenerates this way.
//! Set: WTH #11 — Weatherlight | Scryfall ID: d19ed33b-42d4-4a5d-a763-cfb43348769c | Oracle ID: 7a5cfc97-89cf-43bc-8e48-f8bf7a0c5fcd
// PARTIAL — the regeneration shield is given; "You gain control of that creature if it regenerates this way" is missing (nothing reacts to the shield being used), so the target stays with its controller.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DEBT_OF_LOYALTY,
    oracle_id = "7a5cfc97-89cf-43bc-8e48-f8bf7a0c5fcd",
    scryfall_id = "d19ed33b-42d4-4a5d-a763-cfb43348769c",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "regenerates target creature; gaining control of it when the shield is used (\"if it regenerates this way\") is missing because nothing can react to a regeneration shield being used",
    ),
    faces = &[face!(
        name = "Debt of Loyalty",
        mana_cost = mana!("{1}{W}{W}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::regenerate(TargetSpec::Object(&Filter::CREATURE))],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )],
);
