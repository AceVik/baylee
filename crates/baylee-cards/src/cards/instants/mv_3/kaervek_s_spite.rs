//! Kaervek's Spite — {B}{B}{B} — Instant
//! Oracle: As an additional cost to cast this spell, sacrifice all permanents you control and discard your hand.
//! Oracle: Target player loses 5 life.
//! Set: VIS #63 — Visions | Scryfall ID: d385b9e5-e13d-4098-ba74-ea55bde164d9 | Oracle ID: 7d69be1e-701a-4bcb-a83e-38b4174f7828
// PARTIAL — Sacrificing all permanents you control as an additional spell cost is not yet supported by cast_wizard::paid_as_a_mandatory_additional_cost; in play, Kaervek's Spite discards your hand as an additional cost and sacrifices all permanents you control and causes target player to lose 5 life on resolution.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::KAERVEK_S_SPITE,
    oracle_id = "7d69be1e-701a-4bcb-a83e-38b4174f7828",
    scryfall_id = "d385b9e5-e13d-4098-ba74-ea55bde164d9",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "Sacrificing all permanents you control as an additional spell cost is not yet supported by cast_wizard::paid_as_a_mandatory_additional_cost; in play, Kaervek's Spite discards your hand as an additional cost and sacrifices all permanents you control and causes target player to lose 5 life on resolution.",
    ),
    faces = &[face!(
        name = "Kaervek's Spite",
        mana_cost = mana!("{B}{B}{B}"),
        types = TypeSet::INSTANT,
        mandatory_additional_costs = &[CostPart::DiscardHand],
    ),],
    abilities = &[spell!(
        &[
            Effect::SacrificeAll {
                filter: &Filter::ControlledByYou,
            },
            Effect::LoseLife {
                amount: Amount::Fixed(5),
                target: PlayerRel::Chosen,
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::AnyPlayer)),
    )],
);
