//! Bounded search over public combat positions, never over an engine clone.
//!
//! Each attack set is answered by a tree of blocking assignments. Scores are
//! updated for the one attacker whose block changed; the rest of the board is
//! not rescanned at each node. The second horizon prices a greedy retaliation
//! from surviving creatures. This is a tactical model: spells, triggers,
//! protection and damage-replacement effects are not simulated.

use crate::combat::{Fighter, could_block};
use baylee_cards_dsl::KeywordSet;
use baylee_core::ids::{ObjectId, PlayerId};
use baylee_core::preset::AIProfile;
use baylee_core::types::TypeSet;
use baylee_view::{ObjectStatus, PlayerView};

const MAX: usize = 16;
const WIN: i64 = 1_000_000;

#[derive(Clone, Copy, Default)]
struct Result {
    damage: i32,
    material: i64,
    dead: u32,
    attacker_dead: bool,
}

fn worth(f: Fighter) -> i64 {
    // Tokens have mana value zero but are not worthless. Losing a 6/6 token
    // to a one-mana creature is still an expensive exchange.
    100 + i64::from(f.worth) * 120 + i64::from(f.power.max(0) + f.toughness.max(0)) * 30
}

fn has(f: Fighter, keyword: KeywordSet) -> bool {
    f.keywords & keyword.bits() != 0
}
fn first(f: Fighter) -> bool {
    has(f, KeywordSet::FIRST_STRIKE.union(KeywordSet::DOUBLE_STRIKE))
}
fn lethal(damage: i32, touch: bool, f: Fighter) -> bool {
    !has(f, KeywordSet::INDESTRUCTIBLE) && damage > 0 && (touch || damage >= f.toughness)
}

/// Resolve the two damage steps with simultaneous damage within each step.
/// Blocker order is stable, most valuable first. No heap allocation is made
/// by an exchange, including a gang block.
fn fight(a: Fighter, blockers: &[Fighter], mask: u32) -> Result {
    if mask == 0 {
        return Result {
            damage: a.power.max(0)
                * if has(a, KeywordSet::DOUBLE_STRIKE) {
                    2
                } else {
                    1
                },
            ..Result::default()
        };
    }
    let mut result = Result::default();
    let mut marked = [0_i32; MAX];
    let mut a_damage = 0;
    for early in [true, false] {
        let attacks = !result.attacker_dead
            && if early {
                first(a)
            } else {
                !first(a) || has(a, KeywordSet::DOUBLE_STRIKE)
            };
        let mut incoming = 0;
        let mut touch = false;
        let live = mask & !result.dead;
        for (i, b) in blockers.iter().enumerate() {
            if live & (1 << i) == 0 {
                continue;
            }
            let strikes = if early {
                first(*b)
            } else {
                !first(*b) || has(*b, KeywordSet::DOUBLE_STRIKE)
            };
            if strikes {
                incoming += b.power.max(0);
                touch |= b.power > 0 && has(*b, KeywordSet::DEATHTOUCH);
            }
        }
        if attacks {
            let mut damage = a.power.max(0);
            for (i, b) in blockers.iter().enumerate() {
                if live & (1 << i) == 0 {
                    continue;
                }
                let needed = if has(a, KeywordSet::DEATHTOUCH) {
                    1
                } else {
                    (b.toughness - marked[i]).max(0)
                };
                let assigned = damage.min(needed);
                marked[i] += assigned;
                damage -= assigned;
                if lethal(
                    marked[i],
                    assigned > 0 && has(a, KeywordSet::DEATHTOUCH),
                    *b,
                ) {
                    result.dead |= 1 << i;
                }
            }
            if has(a, KeywordSet::TRAMPLE) {
                result.damage += damage;
            }
        }
        a_damage += incoming;
        result.attacker_dead |= lethal(a_damage, touch, a);
    }
    result.material = blockers
        .iter()
        .enumerate()
        .filter(|(i, _)| result.dead & (1 << i) != 0)
        .map(|(_, f)| worth(*f))
        .sum::<i64>()
        - if result.attacker_dead { worth(a) } else { 0 };
    result
}

struct Position {
    attackers: Vec<Fighter>,
    blockers: Vec<Fighter>,
    // Includes tapped opponents: they untap before the retaliation.
    retaliation: Vec<Fighter>,
    can_block: Vec<u32>,
    life: i32,
    enemy_life: i32,
    horizon: u8,
}

impl Position {
    fn score(&self, going: u32, outcomes: &[Result; MAX], damage: i32, material: i64) -> i64 {
        if damage >= self.enemy_life {
            return WIN + material;
        }
        let mut value = material + i64::from(damage) * 45;
        if self.horizon < 2 {
            return value;
        }
        let dead = outcomes.iter().fold(0, |m, r| m | r.dead);
        let mut incoming = 0;
        let mut used = 0_u32;
        // Retaliation is a legal-model estimate, not another full minimax:
        // each surviving defender attacks our surviving untapped creatures.
        for (j, enemy) in self.retaliation.iter().enumerate() {
            if j < self.blockers.len() && dead & (1 << j) != 0 {
                continue;
            }
            if has(*enemy, KeywordSet::DEFENDER) {
                continue;
            }
            let block = self
                .attackers
                .iter()
                .enumerate()
                .filter(|(i, f)| {
                    used & (1 << i) == 0
                        && !outcomes[*i].attacker_dead
                        && (going & (1 << i) == 0 || has(**f, KeywordSet::VIGILANCE))
                        && could_block(*enemy, **f)
                })
                .max_by_key(|(_, f)| worth(**f));
            if let Some((i, blocker)) = block {
                used |= 1 << i;
                let r = fight(*enemy, &[*blocker], 1);
                incoming += r.damage;
            } else {
                incoming += enemy.power.max(0)
                    * if has(*enemy, KeywordSet::DOUBLE_STRIKE) {
                        2
                    } else {
                        1
                    };
            }
        }
        if incoming >= self.life {
            return -WIN + value;
        }
        value -= i64::from(incoming) * 50;
        value
    }
}

struct Reply<'a> {
    position: &'a Position,
    going: u32,
    groups: [u32; MAX],
    outcomes: [Result; MAX],
    worst: i64,
    cutoff: i64,
    best_groups: [u32; MAX],
    nodes: u32,
    limit: u32,
    complete: bool,
}

impl Reply<'_> {
    fn visit(&mut self, blocker: usize, damage: i32, material: i64) {
        // This reply already disproves an improvement over the incumbent.
        // Remaining replies can only lower the attacking player's score.
        if self.worst <= self.cutoff {
            return;
        }
        if self.nodes >= self.limit {
            self.complete = false;
            return;
        }
        self.nodes += 1;
        if blocker == self.position.blockers.len() {
            // A menace block with exactly one creature is not a legal leaf.
            if self
                .position
                .attackers
                .iter()
                .enumerate()
                .any(|(i, a)| has(*a, KeywordSet::MENACE) && self.groups[i].is_power_of_two())
            {
                return;
            }
            let score = self
                .position
                .score(self.going, &self.outcomes, damage, material);
            if score < self.worst {
                self.worst = score;
                self.best_groups = self.groups;
            }
            return;
        }
        self.visit(blocker + 1, damage, material);
        let choices = self.position.can_block[blocker] & self.going;
        for i in 0..self.position.attackers.len() {
            if choices & (1 << i) == 0 || !self.complete {
                continue;
            }
            let old = self.outcomes[i];
            self.groups[i] |= 1 << blocker;
            let new = fight(
                self.position.attackers[i],
                &self.position.blockers,
                self.groups[i],
            );
            self.outcomes[i] = new;
            self.visit(
                blocker + 1,
                damage - old.damage + new.damage,
                material - old.material + new.material,
            );
            self.groups[i] &= !(1 << blocker);
            self.outcomes[i] = old;
        }
    }
}

/// Search diagnostics also serve the benchmark: time is observed externally,
/// never read by a decision. Incomplete opponent trees cannot promote a move.
#[derive(Clone, Debug)]
pub struct AttackSearch {
    /// Best fully evaluated attack set, or the greedy fallback.
    pub attackers: Vec<ObjectId>,
    /// Blocking-tree nodes visited, at most the supplied budget.
    pub nodes: u32,
    /// Attack sets evaluated completely or refuted by a blocking reply.
    pub completed: u32,
}

fn attack_position(
    view: &PlayerView,
    squad: &[ObjectId],
    victim: PlayerId,
    profile: AIProfile,
) -> Option<Position> {
    let fighters: Vec<_> = squad
        .iter()
        .filter_map(|&id| Fighter::of(view, id))
        .collect();
    // Keep the exact offered-index mapping. A hidden identity is fine (P/T
    // remain public); a missing projected P/T is not a creature to invent.
    if fighters.len() != squad.len()
        || fighters.len() > MAX
        || fighters.is_empty()
        || profile.lookahead == 0
    {
        return None;
    }
    let mut defending: Vec<_> = view
        .battlefield_of(victim)
        .filter(|o| {
            o.types.contains(TypeSet::CREATURE) && !o.status.contains(ObjectStatus::PHASED_OUT)
        })
        .filter_map(|o| Fighter::of(view, o.id).map(|f| (o, f)))
        .collect();
    defending.sort_by_key(|(o, f)| {
        (
            o.status.contains(ObjectStatus::TAPPED),
            std::cmp::Reverse(worth(*f)),
            o.id,
        )
    });
    let blockers: Vec<_> = defending
        .iter()
        .filter(|(o, _)| !o.status.contains(ObjectStatus::TAPPED))
        .map(|(_, f)| *f)
        .collect();
    if blockers.len() > MAX {
        return None;
    }
    Some(Position {
        can_block: blockers
            .iter()
            .map(|b| {
                fighters.iter().enumerate().fold(0, |mask, (i, a)| {
                    mask | if could_block(*a, *b) { 1 << i } else { 0 }
                })
            })
            .collect(),
        attackers: fighters,
        blockers,
        retaliation: defending.iter().map(|(_, f)| *f).collect(),
        life: view.seat(view.seat).map_or(20, |s| s.life),
        enemy_life: view.seat(victim).map_or(20, |s| s.life),
        horizon: profile.lookahead.min(2),
    })
}

/// Search attack declarations from public characteristics alone.
#[must_use]
pub fn attackers(
    view: &PlayerView,
    squad: &[ObjectId],
    victim: PlayerId,
    profile: AIProfile,
) -> AttackSearch {
    let fallback = crate::combat::choose_attackers(view, squad, victim);
    let mut result = AttackSearch {
        attackers: fallback,
        nodes: 0,
        completed: 0,
    };
    let Some(position) = attack_position(view, squad, victim, profile) else {
        return result;
    };
    let all = (1_u32 << squad.len()) - 1;
    let greedy = squad.iter().enumerate().fold(0, |mask, (i, id)| {
        mask | if result.attackers.contains(id) {
            1 << i
        } else {
            0
        }
    });
    let mut best = i64::MIN;
    let mut chosen = greedy;
    let mut fallback_upper = i64::MAX;
    // First establish an incumbent, then try the greedy choice and an alpha
    // strike before subset order. Huge boards still see useful candidates.
    let candidates = std::iter::once(0)
        .chain(std::iter::once(greedy).filter(|&m| m != 0))
        .chain(std::iter::once(all).filter(|&m| m != greedy))
        .chain((1..all).filter(|&m| m != greedy));
    for going in candidates {
        if result.nodes >= profile.node_budget() {
            break;
        }
        let mut reply = Reply {
            position: &position,
            going,
            groups: [0; MAX],
            outcomes: [Result::default(); MAX],
            worst: i64::MAX,
            cutoff: best,
            best_groups: [0; MAX],
            nodes: 0,
            limit: (profile.node_budget() - result.nodes).min(2048),
            complete: true,
        };
        let mut damage = 0;
        for (i, a) in position.attackers.iter().enumerate() {
            if going & (1 << i) != 0 {
                reply.outcomes[i] = fight(*a, &position.blockers, 0);
                damage += reply.outcomes[i].damage;
            }
        }
        reply.visit(0, damage, 0);
        result.nodes += reply.nodes;
        if going == greedy {
            fallback_upper = reply.worst;
        }
        if reply.complete {
            result.completed += 1;
            if reply.worst > best {
                best = reply.worst;
                chosen = going;
            }
        }
    }
    // A partial reply tree gives an upper bound, never proof that standing
    // still beats the greedy attack. Preserve the fallback until a real
    // reply refutes it or a completed candidate beats that upper bound.
    if result.completed > 0 && best >= fallback_upper {
        result.attackers = squad
            .iter()
            .enumerate()
            .filter(|(i, _)| chosen & (1 << i) != 0)
            .map(|(_, id)| *id)
            .collect();
    }
    result
}

/// Search legal blocking assignments, including gang blocks and menace.
#[must_use]
pub fn blockers(
    view: &PlayerView,
    options: &[baylee_engine::choice::BlockOption],
    life: i32,
    profile: AIProfile,
) -> Vec<(ObjectId, ObjectId)> {
    if profile.lookahead == 0 || options.len() > MAX {
        return crate::combat::choose_blocks(view, options, life);
    }
    let ids: Vec<_> = view
        .combat
        .attackers
        .iter()
        .map(|a| a.creature)
        .filter(|id| options.iter().any(|o| o.attackers.contains(id)))
        .collect();
    let attackers: Vec<_> = ids.iter().filter_map(|&id| Fighter::of(view, id)).collect();
    let defenders: Vec<_> = options
        .iter()
        .filter_map(|o| Fighter::of(view, o.blocker))
        .collect();
    if ids.len() > MAX || attackers.len() != ids.len() || defenders.len() != options.len() {
        return crate::combat::choose_blocks(view, options, life);
    }
    let unblockable: i32 = view
        .combat
        .attackers
        .iter()
        .filter(|a| {
            !ids.contains(&a.creature)
                && a.defending == baylee_core::ids::Defender::Player(view.seat)
        })
        .filter_map(|a| Fighter::of(view, a.creature))
        .map(|f| fight(f, &[], 0).damage)
        .sum();
    let position = Position {
        attackers,
        blockers: defenders,
        retaliation: vec![],
        can_block: options
            .iter()
            .map(|o| {
                ids.iter().enumerate().fold(0, |mask, (i, id)| {
                    mask | if o.attackers.contains(id) { 1 << i } else { 0 }
                })
            })
            .collect(),
        life: i32::MAX,
        enemy_life: life.saturating_sub(unblockable),
        horizon: 0,
    };
    let mut reply = Reply {
        position: &position,
        going: (1 << ids.len()) - 1,
        groups: [0; MAX],
        outcomes: [Result::default(); MAX],
        worst: i64::MAX,
        cutoff: i64::MIN,
        best_groups: [0; MAX],
        nodes: 0,
        limit: profile.node_budget(),
        complete: true,
    };
    let mut damage = 0;
    for (i, a) in position.attackers.iter().enumerate() {
        reply.outcomes[i] = fight(*a, &[], 0);
        damage += reply.outcomes[i].damage;
    }
    reply.visit(0, damage, 0);
    let mut result = Vec::new();
    for (i, &id) in ids.iter().enumerate() {
        for (j, option) in options.iter().enumerate() {
            if reply.best_groups[i] & (1 << j) != 0 {
                result.push((option.blocker, id));
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fighter(p: i32, t: i32, k: KeywordSet) -> Fighter {
        Fighter {
            power: p,
            toughness: t,
            keywords: k.bits(),
            worth: 2,
        }
    }
    #[test]
    fn double_strike_and_gang_blocks_use_both_damage_steps() {
        let a = fighter(2, 4, KeywordSet::DOUBLE_STRIKE);
        let b = fighter(3, 3, KeywordSet::EMPTY);
        let result = fight(a, &[b], 1);
        assert_eq!(result.dead, 1);
        assert!(!result.attacker_dead);
        let result = fight(
            fighter(4, 4, KeywordSet::EMPTY),
            &[fighter(2, 2, KeywordSet::EMPTY); 2],
            3,
        );
        assert_eq!(result.dead, 3);
        assert!(result.attacker_dead);
    }
    #[test]
    fn deathtouch_trample_assigns_one_per_blocker() {
        let result = fight(
            fighter(5, 5, KeywordSet::DEATHTOUCH.union(KeywordSet::TRAMPLE)),
            &[fighter(8, 8, KeywordSet::EMPTY)],
            1,
        );
        assert_eq!(result.damage, 4);
        assert_eq!(result.dead, 1);
    }
}
