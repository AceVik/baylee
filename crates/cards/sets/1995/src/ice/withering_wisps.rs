//! Withering Wisps — {1}{B}{B} — Enchantment
//! Oracle: At the beginning of the end step, if no creatures are on the battlefield, sacrifice this enchantment.
//! Oracle: {B}: This enchantment deals 1 damage to each creature and each player. Activate no more times each turn than the number of snow Swamps you control.
//! Set: ME2 #114 — Masters Edition II | Scryfall ID: 595228da-39ab-4272-bcf4-afd8d229fb01 | Oracle ID: ce65c6ed-e5d4-48d8-b569-4e2774b2912c
// PARTIAL — the {B} ability is limited to one activation per turn, and only while you control a snow Swamp; printed, the cap is one per snow Swamp you control.
// NOT SUPPORTED: "Activate no more times each turn than the number of snow Swamps you control." ActivationLimit::PerTurn takes a fixed number, not a count; with two or more snow Swamps the card allows fewer activations than printed.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static SNOW_SWAMP: Filter = Filter::And(&[
    Filter::LAND,
    Filter::HasSupertype(SupertypeSet::SNOW),
    Filter::HasSubtype(subtypes::land::SWAMP),
]);

card!(
    index = index::WITHERING_WISPS,
    oracle_id = "ce65c6ed-e5d4-48d8-b569-4e2774b2912c",
    scryfall_id = "595228da-39ab-4272-bcf4-afd8d229fb01",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "the per-turn activation cap is the number of snow Swamps you control, but ActivationLimit::PerTurn is a fixed number: the card allows one activation per turn while you control a snow Swamp, which is fewer than printed with two or more",
    ),
    faces = &[face!(
        name = "Withering Wisps",
        mana_cost = mana!("{1}{B}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::End,
                whose: PlayerRel::EachPlayer,
            },
            &[Effect::SacrificeSelf],
            condition = Some(Condition::BattlefieldCountAtMost(&Filter::CREATURE, 0))
        ),
        activated!(
            cost!("{B}"),
            &[
                Effect::DealDamageEach {
                    amount: Amount::Fixed(1),
                    filter: &Filter::CREATURE,
                },
                Effect::DealDamage {
                    amount: Amount::Fixed(1),
                    target: TargetSpec::Player(PlayerRel::EachPlayer),
                },
            ],
            condition = Some(Condition::ControlCount(&SNOW_SWAMP, 1)),
            limit = ActivationLimit::PerTurn(1)
        ),
    ],
);
