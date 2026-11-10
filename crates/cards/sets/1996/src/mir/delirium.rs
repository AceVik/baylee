//! Delirium — {1}{B}{R} — Instant
//! Oracle: Cast this spell only during an opponent's turn.
//! Oracle: Tap target creature that player controls. That creature deals damage equal to its power to the player. Prevent all combat damage that would be dealt to and dealt by the creature this turn.
//! Set: MIR #260 — Mirage | Scryfall ID: c52981ea-1173-4f4b-a929-705a11c6e381 | Oracle ID: a00e5eb6-b916-4cd6-8ff2-c02f4929abc7
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

static ACTIVE_PLAYER_CREATURE: Filter =
    Filter::And(&[Filter::CREATURE, Filter::ControlledByActivePlayer]);

card!(
    index = index::DELIRIUM,
    oracle_id = "a00e5eb6-b916-4cd6-8ff2-c02f4929abc7",
    scryfall_id = "c52981ea-1173-4f4b-a929-705a11c6e381",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Delirium",
        mana_cost = mana!("{1}{B}{R}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::TapTarget,
            // "That creature deals damage equal to its power to the player":
            // the creature is the source (lifelink, protection and
            // prevention read it) and its controller is the player.
            Effect::ForEachMatching {
                filter: &Filter::Reference,
                effects: &[Effect::EventObjectDealsDamageEqualToPower {
                    target: TargetSpec::Player(PlayerRel::EventPlayer),
                }],
            },
            Effect::ModifyTargets {
                modifier: Modifier::PreventDamageToIt,
                duration: Duration::UntilEndOfTurn,
            },
            Effect::ModifyTargets {
                modifier: Modifier::PreventDamageFromIt,
                duration: Duration::UntilEndOfTurn,
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&ACTIVE_PLAYER_CREATURE))),
        condition = Some(Condition::OpponentsTurn),
    )],
);
