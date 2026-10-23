//! Prismatic Boon — {X}{W}{U} — Instant
//! Oracle: Choose a color. X target creatures gain protection from the chosen color until end of turn.
//! Set: MIR #274 — Mirage | Scryfall ID: fb774a9b-d29f-4f41-9fb8-a0189205e16f | Oracle ID: aee5dda1-2181-40d5-9846-85d154e1d5da
// PARTIAL: Effect::ProtectionFromChosenColor gives the protection to the FIRST target only, so the spell takes one target whatever X is (TargetReq::x_targets exists, but the other X-1 targets would get nothing); casting this spell chooses a color and grants one target creature protection from the chosen color until end of turn

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PRISMATIC_BOON,
    oracle_id = "aee5dda1-2181-40d5-9846-85d154e1d5da",
    scryfall_id = "fb774a9b-d29f-4f41-9fb8-a0189205e16f",
    color_identity = ColorSet::from_slice(&[Color::Blue, Color::White]),
    coverage = Coverage::Partial(
        "Effect::ProtectionFromChosenColor gives the protection to the first target only, so the spell takes one target whatever X is (TargetReq::x_targets exists, but the other X-1 targets would get nothing); casting this spell chooses a color and grants one target creature protection from the chosen color until end of turn"
    ),
    faces = &[face!(
        name = "Prismatic Boon",
        mana_cost = mana!("{X}{W}{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::ProtectionFromChosenColor {
            duration: Duration::UntilEndOfTurn,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )],
);
