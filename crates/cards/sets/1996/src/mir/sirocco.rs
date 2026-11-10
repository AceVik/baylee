//! Sirocco — {1}{R} — Instant
//! Oracle: Target player reveals their hand. For each blue instant card revealed this way, that player discards that card unless they pay 4 life.
//! Set: MIR #192 — Mirage | Scryfall ID: ce37492e-4f07-4171-97d4-84f28fb4e2be | Oracle ID: a9bdf857-0c6b-4afa-afbb-aff7e4b61b09
// PARTIAL: target player reveals their hand; the per-card choice to pay 4 life or discard each revealed blue instant card is not in the engine, so nothing is discarded.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SIROCCO,
    oracle_id = "a9bdf857-0c6b-4afa-afbb-aff7e4b61b09",
    scryfall_id = "ce37492e-4f07-4171-97d4-84f28fb4e2be",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "target player reveals their hand; the per-card choice to pay 4 life or discard each revealed blue instant card is not in the engine, so nothing is discarded",
    ),
    faces = &[face!(
        name = "Sirocco",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        // NOT SUPPORTED: "For each blue instant card revealed this way, that
        // player discards that card unless they pay 4 life." A forced discard
        // would take away the player's choice to pay 4 life, and the engine
        // has no unless-cost that runs once per revealed card.
        &[Effect::RevealHand {
            who: PlayerRel::Chosen,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyPlayer)),
    )],
);
