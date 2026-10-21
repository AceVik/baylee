//! Incinerate — {1}{R} — Instant
//! Oracle: Incinerate deals 3 damage to any target. A creature dealt damage this way can't be regenerated this turn.
//! Set: JVC #51 — Duel Decks Anthology: Jace vs. Chandra | Scryfall ID: f23dff46-e040-4cba-890a-97c2899701f2 | Oracle ID: d8fd7a34-8418-4e98-b79b-119c4348c667
// NOT SUPPORTED: a creature dealt damage this way can't be regenerated this turn checks whether Incinerate dealt any damage and whether its target is a creature, rather than tracking which creature actually received the damage under redirection.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::INCINERATE,
    oracle_id = "d8fd7a34-8418-4e98-b79b-119c4348c667",
    scryfall_id = "f23dff46-e040-4cba-890a-97c2899701f2",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "a creature dealt damage this way can't be regenerated this turn checks whether Incinerate dealt any damage and whether its target is a creature, rather than tracking which creature actually received the damage under redirection",
    ),
    faces = &[face!(
        name = "Incinerate",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::DealDamage {
                amount: Amount::Fixed(3),
                target: TargetSpec::AnyTarget,
            },
            Effect::IfCondition {
                condition: Condition::AmountAtLeast(Amount::ThisWay(ThisWay::DamageDealt), 1),
                then: &[Effect::IfTargetMatches {
                    filter: &Filter::CREATURE,
                    then: &[Effect::CantBeRegeneratedThisTurn {
                        target: TargetSpec::AnyTarget,
                    }],
                }],
                otherwise: &[],
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::AnyTarget)),
    )],
);
