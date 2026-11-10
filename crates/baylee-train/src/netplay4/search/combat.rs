//! Combat declarations as roots ([`SearchConfig4::with_combat`], v4).
//!
//! The engine forks at a declaration of attackers or blockers as at
//! priority (`docs/engine-internals.md` §"Forking a simulation"): the offer
//! is public. A v4 search takes such a declaration as a root when the
//! policy's own answer is its own (not forced, not a fallback), and
//! compares the policy's declaration with a handful of others built from
//! it: none, all in (attacks), and each creature's part changed alone.
//! What the engine refuses in a world is dropped before the rounds, so a
//! declaration a requirement forbids costs nothing.
//!
//! The policy's subset and pair heads give no ranking over declarations,
//! so every other candidate's prior is zero and the policy's one: under
//! [`Selection4::Prior`] another declaration must beat the policy's mean
//! by `c / (1 + n)` over `n` worlds.
//!
//! [`SearchConfig4::with_combat`]: super::SearchConfig4::with_combat
//! [`Selection4::Prior`]: super::Selection4::Prior

use baylee_core::ids::{Defender, ObjectId, PlayerId};
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_engine::engine::ForkRefusal;

use super::rollout::world;
use super::{Candidate4, ForkSource4, Ranked4};

/// The declarations [`candidates`] builds that the engine takes in the
/// world forked from `seed`, as search candidates, the policy's first. A
/// declaration a requirement or restriction forbids is refused there, and
/// so never refuses a round.
///
/// # Errors
/// The table's refusal to fork.
pub(super) fn playable<T: ForkSource4>(
    table: &T,
    seat: PlayerId,
    pending: &Pending,
    own: &PlayerAction,
    limit: usize,
    seed: u64,
) -> Result<Vec<Candidate4>, ForkRefusal> {
    let mut out = Vec::new();
    for (at, action) in candidates(pending, own, limit).into_iter().enumerate() {
        let mut probe = world(table, seat, seed)?;
        if probe.act_slice(seat, action.clone(), 0).is_err() {
            if at == 0 {
                // The policy's own declaration is the table's to refuse.
                return Ok(Vec::new());
            }
            continue;
        }
        out.push(Candidate4::plain(Ranked4 {
            index: 0,
            action,
            logit: f32::NEG_INFINITY,
        }));
    }
    Ok(out)
}

/// Whether `pending` is a declaration a v4 search may take as its root.
pub(super) const fn is_declaration(pending: &Pending) -> bool {
    matches!(
        pending,
        Pending::ChooseAttackers { .. } | Pending::ChooseBlockers { .. }
    )
}

/// The declarations compared at `pending`, `own` (the policy's) first, then
/// the others in a fixed order, at most `limit` in all; empty when `own` is
/// not a declaration of `pending`'s kind.
pub(super) fn candidates(pending: &Pending, own: &PlayerAction, limit: usize) -> Vec<PlayerAction> {
    let mut out = match (pending, own) {
        (
            Pending::ChooseAttackers {
                attackers,
                defenders,
                alone,
                limits,
                ..
            },
            PlayerAction::DeclareAttackers {
                attackers: declared,
            },
        ) => attacks(attackers, defenders, alone, limits, declared),
        (
            Pending::ChooseBlockers { blockers, .. },
            PlayerAction::DeclareBlockers { blockers: declared },
        ) => blocks(blockers, declared),
        _ => return Vec::new(),
    };
    out.truncate(limit.max(1));
    out
}

/// One declaration in a canonical order, so two orderings of the same
/// declaration count once.
fn canonical<T: Ord + Clone>(pairs: &[T]) -> Vec<T> {
    let mut pairs = pairs.to_vec();
    pairs.sort_unstable();
    pairs.dedup();
    pairs
}

/// Pushes `candidate` unless an equal declaration is already there.
fn push_new<T: Ord + Clone>(out: &mut Vec<Vec<T>>, candidate: &[T]) {
    let candidate = canonical(candidate);
    if !out.contains(&candidate) {
        out.push(candidate);
    }
}

fn attacks(
    offered: &[ObjectId],
    defenders: &[Defender],
    alone: &[ObjectId],
    limits: &[baylee_engine::choice::AttackLimit],
    own: &[(ObjectId, Defender)],
) -> Vec<PlayerAction> {
    // A creature's defender: the policy's where it attacks, else the first
    // it may attack.
    let defender_of = |creature: ObjectId| -> Option<Defender> {
        own.iter()
            .find(|(attacker, _)| *attacker == creature)
            .map(|(_, defender)| *defender)
            .or_else(|| {
                limits
                    .iter()
                    .find(|limit| limit.creature == creature)
                    .and_then(|limit| limit.defenders.first().copied())
            })
            .or_else(|| defenders.first().copied())
    };
    // Whether `creature` may attack `defender` (CR 508.1b).
    let may_attack = |creature: ObjectId, defender: Defender| {
        limits
            .iter()
            .find(|limit| limit.creature == creature)
            .is_none_or(|limit| limit.defenders.contains(&defender))
    };
    let mut out: Vec<Vec<(ObjectId, Defender)>> = vec![canonical(own)];
    push_new(&mut out, &[]);
    let all: Vec<(ObjectId, Defender)> = offered
        .iter()
        .filter(|creature| !alone.contains(creature))
        .filter_map(|&creature| defender_of(creature).map(|defender| (creature, defender)))
        .collect();
    push_new(&mut out, &all);
    // With more than one defender (a team game, a planeswalker): the policy's
    // attackers all at each other defender, so that focusing one is tried.
    for &defender in defenders {
        let focused: Vec<(ObjectId, Defender)> = own
            .iter()
            .map(|&(creature, _)| (creature, defender))
            .filter(|&(creature, defender)| may_attack(creature, defender))
            .collect();
        if focused.len() == own.len() {
            push_new(&mut out, &focused);
        }
    }
    for &creature in offered {
        let mut toggled: Vec<(ObjectId, Defender)> = own
            .iter()
            .copied()
            .filter(|(attacker, _)| *attacker != creature)
            .collect();
        if toggled.len() == own.len() {
            let Some(defender) = defender_of(creature) else {
                continue;
            };
            if alone.contains(&creature) {
                toggled.clear();
            }
            toggled.push((creature, defender));
        }
        push_new(&mut out, &toggled);
    }
    out.into_iter()
        .map(|attackers| PlayerAction::DeclareAttackers { attackers })
        .collect()
}

fn blocks(
    options: &[baylee_engine::choice::BlockOption],
    own: &[(ObjectId, ObjectId)],
) -> Vec<PlayerAction> {
    let mut out: Vec<Vec<(ObjectId, ObjectId)>> = vec![canonical(own)];
    push_new(&mut out, &[]);
    for option in options {
        let without: Vec<(ObjectId, ObjectId)> = own
            .iter()
            .copied()
            .filter(|(blocker, _)| *blocker != option.blocker)
            .collect();
        if without.len() != own.len() {
            push_new(&mut out, &without);
        }
        for &attacker in &option.attackers {
            let mut moved = without.clone();
            moved.push((option.blocker, attacker));
            push_new(&mut out, &moved);
        }
    }
    out.into_iter()
        .map(|blockers| PlayerAction::DeclareBlockers { blockers })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_core::ids::PlayerId;
    use baylee_engine::choice::BlockOption;

    const THEM: PlayerId = PlayerId::new(1);

    fn id(n: u32) -> ObjectId {
        ObjectId::new(n, 0)
    }

    fn ask_attackers(offered: &[u32], alone: &[u32]) -> Pending {
        Pending::ChooseAttackers {
            player: PlayerId::new(0),
            attackers: offered.iter().map(|&n| id(n)).collect(),
            defenders: vec![Defender::Player(THEM)],
            required: vec![],
            alone: alone.iter().map(|&n| id(n)).collect(),
            minimum_required: None,
            limits: vec![],
        }
    }

    fn attack(creatures: &[u32]) -> PlayerAction {
        PlayerAction::DeclareAttackers {
            attackers: creatures
                .iter()
                .map(|&n| (id(n), Defender::Player(THEM)))
                .collect(),
        }
    }

    #[test]
    fn the_policys_attack_first_then_none_all_and_each_creature_changed() {
        let pending = ask_attackers(&[1, 2, 3], &[]);
        let found = candidates(&pending, &attack(&[2]), 16);
        assert_eq!(
            found,
            [
                attack(&[2]),
                attack(&[]),
                attack(&[1, 2, 3]),
                attack(&[1, 2]),
                attack(&[2, 3]),
            ]
        );
        // The policy's own toggled away is "none", already there.
        assert_eq!(candidates(&pending, &attack(&[2]), 3).len(), 3);
    }

    /// At a table of two opponents the policy's attack is also tried with
    /// every attacker at the other one; a creature that may not attack
    /// there keeps that focus from being tried.
    #[test]
    fn a_team_attack_is_tried_focused_on_each_defender() {
        let other = PlayerId::new(3);
        let mut pending = ask_attackers(&[1, 2], &[]);
        if let Pending::ChooseAttackers { defenders, .. } = &mut pending {
            defenders.push(Defender::Player(other));
        }
        let own = attack(&[1, 2]);
        let focused = PlayerAction::DeclareAttackers {
            attackers: vec![
                (id(1), Defender::Player(other)),
                (id(2), Defender::Player(other)),
            ],
        };
        assert!(candidates(&pending, &own, 16).contains(&focused));
        if let Pending::ChooseAttackers { limits, .. } = &mut pending {
            limits.push(baylee_engine::choice::AttackLimit {
                creature: id(2),
                defenders: vec![Defender::Player(THEM)],
            });
        }
        assert!(!candidates(&pending, &own, 16).contains(&focused));
    }

    #[test]
    fn a_creature_that_attacks_alone_is_tried_alone() {
        let pending = ask_attackers(&[1, 2], &[2]);
        let found = candidates(&pending, &attack(&[1]), 16);
        assert!(found.contains(&attack(&[2])), "{found:?}");
        assert!(!found.contains(&attack(&[1, 2])), "{found:?}");
    }

    #[test]
    fn blocks_are_the_policys_then_none_then_each_blocker_moved_or_dropped() {
        let pending = Pending::ChooseBlockers {
            player: PlayerId::new(0),
            attacker: THEM,
            blockers: vec![
                BlockOption {
                    blocker: id(10),
                    attackers: vec![id(1), id(2)],
                },
                BlockOption {
                    blocker: id(11),
                    attackers: vec![id(1)],
                },
            ],
            capacity: vec![],
            obeying: vec![],
            demands: vec![],
            bounds: vec![],
        };
        let block = |pairs: &[(u32, u32)]| PlayerAction::DeclareBlockers {
            blockers: pairs.iter().map(|&(b, a)| (id(b), id(a))).collect(),
        };
        let found = candidates(&pending, &block(&[(10, 1)]), 16);
        assert_eq!(
            found,
            [
                block(&[(10, 1)]),
                block(&[]),
                block(&[(10, 2)]),
                block(&[(10, 1), (11, 1)]),
            ]
        );
    }

    #[test]
    fn another_question_has_no_candidates() {
        let pending = ask_attackers(&[1], &[]);
        assert!(candidates(&pending, &PlayerAction::PassPriority, 8).is_empty());
    }
}
