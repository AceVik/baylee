//! Tendrils of Despair — {B} — Sorcery
//! Oracle: As an additional cost to cast this spell, sacrifice a creature.
//! Oracle: Target opponent discards two cards.
//! Set: WTH #83 — Weatherlight | Scryfall ID: b5d73ddb-bd3c-4625-9f75-ba2079553915 | Oracle ID: 3b00f415-2496-4333-be89-601fd837fe7c
// IMPLEMENTED — Sacrifices a creature as an additional cost and makes target opponent discard two cards.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TENDRILS_OF_DESPAIR,
    oracle_id = "3b00f415-2496-4333-be89-601fd837fe7c",
    scryfall_id = "b5d73ddb-bd3c-4625-9f75-ba2079553915",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Tendrils of Despair",
        mana_cost = mana!("{B}"),
        types = TypeSet::SORCERY,
        mandatory_additional_costs = &[CostPart::Sacrifice(&Filter::CREATURE)],
    ),],
    abilities = &[spell!(
        &[Effect::DiscardForPlayers {
            who: PlayerRel::TargetPlayerOrController,
            count: 2,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyOpponent)),
    ),],
);
