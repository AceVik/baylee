//! Suffocation — {1}{U} — Instant
//! Oracle: Cast this spell only if you were dealt damage this turn by a red instant or sorcery spell.
//! Oracle: Suffocation deals 4 damage to the controller of the last red instant or sorcery spell that dealt damage to you this turn.
//! Oracle: Draw a card at the beginning of the next turn's upkeep.
//! Set: ALL #38 — Alliances | Scryfall ID: d22104df-8147-45fd-897a-f99a815be062 | Oracle ID: 57aeb106-dc4f-4069-b596-f997263dc0cf

// PARTIAL — cast only if a red instant or sorcery spell an opponent controls dealt damage to you this turn; then 4 damage to an opponent and a card at the next turn's upkeep.
// NOT SUPPORTED: "to the controller of the last red instant or sorcery spell that dealt damage to you this turn." Nothing names that player, so the 4 damage goes to an opponent (the only one heads-up). It differs from the printed card only when your own red spell was the last to damage you (the printed card then hurts you), and the cast condition is stricter there: damage from your own red spells does not count.

use baylee_cards_dsl::prelude::*;

/// "A red instant or sorcery spell" that dealt the damage, as it was when it
/// dealt it, controlled by an opponent of the player who casts Suffocation.
static RED_INSTANT_OR_SORCERY_OF_AN_OPPONENT: Filter = Filter::And(&[
    Filter::HasColor(ColorSet::from_slice(&[Color::Red])),
    Filter::INSTANT_OR_SORCERY,
    Filter::ControlledByOpponent,
]);

card!(
    index = index::SUFFOCATION,
    oracle_id = "57aeb106-dc4f-4069-b596-f997263dc0cf",
    scryfall_id = "d22104df-8147-45fd-897a-f99a815be062",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Suffocation",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
    coverage = Coverage::Partial(
        "Cast only if a red instant or sorcery spell an opponent controls dealt damage to you this turn (damage from your own red spells does not count); the 4 damage goes to an opponent of yours, the only one heads-up, rather than to the controller of the last red instant or sorcery that dealt damage to you, which differs only when your own red spell was the last to damage you."
    ),
    abilities = &[spell!(
        &[
            Effect::DealDamage {
                amount: Amount::Fixed(4),
                target: TargetSpec::Player(PlayerRel::Opponent),
            },
            Effect::AtNextTurnsUpkeep {
                effects: &[Effect::draw(1)],
            },
        ],
        condition = Some(Condition::AmountAtLeast(
            Amount::DamageThisTurn {
                to: DamageTo::You,
                by: &RED_INSTANT_OR_SORCERY_OF_AN_OPPONENT,
                combat: None,
            },
            1
        )),
    )],
);
