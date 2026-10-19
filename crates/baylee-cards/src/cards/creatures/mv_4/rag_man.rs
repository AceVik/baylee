//! Rag Man — {2}{B}{B} — Creature — Human Minion
//! Oracle: {B}{B}{B}, {T}: Target opponent reveals their hand and discards a creature card at random. Activate only during your turn.
//! Set: 7ED #156 — Seventh Edition | Scryfall ID: 8e3809e6-41ac-47e8-80dc-9e9c8be1ed7a | Oracle ID: 8132c521-44e2-4fc0-b3de-e5b92d04463a

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RAG_MAN,
    oracle_id = "8132c521-44e2-4fc0-b3de-e5b92d04463a",
    scryfall_id = "8e3809e6-41ac-47e8-80dc-9e9c8be1ed7a",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Rag Man",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::MINION],
        power = Some(2),
        toughness = Some(1),
    ),],
    abilities = &[AbilityDef::ActivatedConditional {
        cost: cost!("{B}{B}{B}", TapSelf),
        effects: &[Effect::RevealHandDiscardRandom {
            filter: &Filter::CREATURE,
        }],
        targets: Some(TargetReq::one(TargetSpec::AnyOpponent)),
        second_targets: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: false,
        zone: ActivationZone::Battlefield,
        condition: Condition::YourTurn,
        limit: ActivationLimit::Unlimited,
        x_value: ActivationX::Any,
        cost_reduction: None,
    }],
);
