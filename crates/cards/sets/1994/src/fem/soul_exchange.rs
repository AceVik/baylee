//! Soul Exchange — {B}{B} — Sorcery
//! Oracle: As an additional cost to cast this spell, exile a creature you control.
//! Oracle: Return target creature card from your graveyard to the battlefield. Put a +2/+2 counter on that creature if the exiled creature was a Thrull.
//! Set: ME2 #111 — Masters Edition II | Scryfall ID: 6666dca1-5e33-4f5b-b449-76bcb0e15296 | Oracle ID: 366ea3b4-5e7e-4693-9f4b-1e549972609e
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const THRULL: Filter = Filter::HasSubtype(subtypes::creature::THRULL);

card!(
    index = index::SOUL_EXCHANGE,
    oracle_id = "366ea3b4-5e7e-4693-9f4b-1e549972609e",
    scryfall_id = "6666dca1-5e33-4f5b-b449-76bcb0e15296",
    coverage = Coverage::Implemented,
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Soul Exchange",
        mana_cost = mana!("{B}{B}"),
        types = TypeSet::SORCERY,
        mandatory_additional_costs = &[CostPart::ExilePermanent(&Filter::YOUR_CREATURE)],
    ),],
    abilities = &[spell!(
        &[
            Effect::reanimate(TargetSpec::CardInGraveyard(
                &Filter::CREATURE,
                PlayerRel::You,
            )),
            Effect::IfCondition {
                condition: Condition::SacrificedForCost(&THRULL),
                then: &[Effect::AddCounter {
                    kind: CounterKind::Plus {
                        power: 2,
                        toughness: 2,
                    },
                    amount: Amount::Fixed(1),
                }],
                otherwise: &[],
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(
            &Filter::CREATURE,
            PlayerRel::You,
        ))),
    ),],
);
