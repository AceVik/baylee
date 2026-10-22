//! Sirocco — {1}{R} — Instant
//! Oracle: Target player reveals their hand. For each blue instant card revealed this way, that player discards that card unless they pay 4 life.
//! Set: MIR #192 — Mirage | Scryfall ID: ce37492e-4f07-4171-97d4-84f28fb4e2be | Oracle ID: a9bdf857-0c6b-4afa-afbb-aff7e4b61b09
// PARTIAL: Per-card punisher choice to pay 4 life or discard each revealed blue instant card is not in the engine; reveals target player's hand and makes them discard a blue instant card if they have one.

use baylee_cards_dsl::prelude::*;

static BLUE_INSTANT: Filter = Filter::And(&[
    Filter::HasType(TypeSet::INSTANT),
    Filter::HasColor(ColorSet::from_slice(&[Color::Blue])),
]);

card!(
    index = index::SIROCCO,
    oracle_id = "a9bdf857-0c6b-4afa-afbb-aff7e4b61b09",
    scryfall_id = "ce37492e-4f07-4171-97d4-84f28fb4e2be",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "per-card punisher choice to pay 4 life or discard each revealed blue instant card is not in the engine; reveals target player's hand and makes them discard a blue instant card if they have one",
    ),
    faces = &[face!(
        name = "Sirocco",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::RevealHandDiscard {
            filter: &BLUE_INSTANT,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyPlayer)),
    )],
);
