//! Presentation events for the two combat-damage steps. Never changes rules.
use crate::{board::KeywordBadge, combat::LineEnd};
use baylee_core::ids::{Defender, ObjectId};
use baylee_view::{PlayerView, Step};

/// A creature's strike and its declared destination.
#[derive(Clone, Copy, Debug)]
pub struct Strike {
    /// Creature performing the gesture.
    pub source: ObjectId,
    /// A blocker, attacker, planeswalker or player.
    pub target: LineEnd,
    /// The swift first-strike gesture rather than the heavier normal hit.
    pub first: bool,
}

/// Damage is dealt by the engine on entering the step. Read each edge once,
/// using the previous battlefield so lethal damage still has a source.
#[must_use]
pub fn between(before: Option<&PlayerView>, after: &PlayerView) -> Vec<Strike> {
    let Some(before) = before else {
        return vec![];
    };
    if after.seq <= before.seq
        || (after.turn == before.turn && after.step == before.step)
        || !matches!(after.step, Step::CombatDamageFirst | Step::CombatDamage)
    {
        return vec![];
    }
    let first = after.step == Step::CombatDamageFirst;
    let eligible = |id| {
        before
            .battlefield
            .iter()
            .find(|o| o.id == id)
            .is_some_and(|o| {
                let fast = o.keywords & KeywordBadge::FirstStrike.bit() != 0;
                let double = o.keywords & KeywordBadge::DoubleStrike.bit() != 0;
                o.power.is_some_and(|p| p > 0)
                    && if first {
                        fast || double
                    } else {
                        before.step != Step::CombatDamageFirst || !fast || double
                    }
            })
    };
    let mut strikes = Vec::new();
    for attacker in &before.combat.attackers {
        if !eligible(attacker.creature) {
            continue;
        }
        let blockers: Vec<_> = before
            .combat
            .blockers
            .iter()
            .filter(|b| b.attacker == attacker.creature)
            .collect();
        if blockers.is_empty() {
            if !attacker.blocked
                || before
                    .object(attacker.creature)
                    .is_some_and(|o| o.keywords & KeywordBadge::Trample.bit() != 0)
            {
                strikes.push(Strike {
                    source: attacker.creature,
                    target: match attacker.defending {
                        Defender::Player(id) => LineEnd::Seat(id),
                        Defender::Planeswalker(id) => LineEnd::Object(id),
                    },
                    first,
                });
            }
        } else {
            for blocker in blockers {
                strikes.push(Strike {
                    source: attacker.creature,
                    target: LineEnd::Object(blocker.blocker),
                    first,
                });
            }
        }
    }
    for blocker in &before.combat.blockers {
        if eligible(blocker.blocker) {
            strikes.push(Strike {
                source: blocker.blocker,
                target: LineEnd::Object(blocker.attacker),
                first,
            });
        }
    }
    strikes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{ViewBuilder, token};
    use baylee_core::ids::PlayerId;
    use baylee_view::AttackerView;
    fn fight() -> PlayerView {
        let mut v = ViewBuilder::new(2)
            .with_battlefield(0, vec![token(1, 0, "Knight", 2, 2)])
            .with_combat(
                vec![AttackerView {
                    creature: ObjectId::new(1, 0),
                    defending: Defender::Player(PlayerId::new(1)),
                    blocked: false,
                }],
                vec![],
            )
            .build();
        v.step = Step::DeclareBlockers;
        v
    }
    #[test]
    fn entering_damage_is_one_gesture_and_a_reconnect_is_silent() {
        let before = fight();
        let mut after = before.clone();
        after.seq += 1;
        after.step = Step::CombatDamage;
        assert_eq!(between(Some(&before), &after).len(), 1);
        assert!(between(None, &after).is_empty());
        let mut repeat = after.clone();
        repeat.seq += 1;
        assert!(between(Some(&after), &repeat).is_empty());
    }
    #[test]
    fn first_and_double_strike_are_separate_and_dead_sources_still_strike() {
        let mut before = fight();
        before.battlefield[0].keywords = KeywordBadge::FirstStrike.bit();
        let mut first = before.clone();
        first.seq += 1;
        first.step = Step::CombatDamageFirst;
        assert!(between(Some(&before), &first)[0].first);
        let mut normal = first.clone();
        normal.seq += 1;
        normal.step = Step::CombatDamage;
        assert!(between(Some(&first), &normal).is_empty());
        first.battlefield[0].keywords |= KeywordBadge::DoubleStrike.bit();
        normal.battlefield.clear();
        assert_eq!(between(Some(&first), &normal).len(), 1);
    }
    #[test]
    fn a_blocked_attacker_does_not_hit_a_player_after_its_blocker_left() {
        let mut before = fight();
        before.combat.attackers[0].blocked = true;
        let mut after = before.clone();
        after.seq += 1;
        after.step = Step::CombatDamage;
        assert!(between(Some(&before), &after).is_empty());
    }
}
