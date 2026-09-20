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
    first_damage: i32,
    material: i64,
    gain: i32,
    enemy_gain: i32,
    first_enemy_gain: i32,
    dead: u32,
    attacker_dead: bool,
    commander_lethal: bool,
}

#[derive(Clone, Copy, Default)]
struct Balance {
    commander_lethals: u8,
    damage: i32,
    first_damage: i32,
    material: i64,
    gain: i32,
    enemy_gain: i32,
    first_enemy_gain: i32,
}

impl Balance {
    fn replacing(self, old: Result, new: Result) -> Self {
        Self {
            commander_lethals: self.commander_lethals - u8::from(old.commander_lethal)
                + u8::from(new.commander_lethal),
            damage: self.damage - old.damage + new.damage,
            first_damage: self.first_damage - old.first_damage + new.first_damage,
            material: self.material - old.material + new.material,
            gain: self.gain - old.gain + new.gain,
            enemy_gain: self.enemy_gain - old.enemy_gain + new.enemy_gain,
            first_enemy_gain: self.first_enemy_gain - old.first_enemy_gain + new.first_enemy_gain,
        }
    }

    fn kills(self, life: i32) -> bool {
        // State-based actions happen between damage steps. Lifelink in the
        // normal step cannot rescue a player who lost in first strike.
        self.commander_lethals > 0
            || self.first_damage >= life + self.first_enemy_gain
            || self.damage >= life + self.enemy_gain
    }
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
        let damage = a.power.max(0)
            * if has(a, KeywordSet::DOUBLE_STRIKE) {
                2
            } else {
                1
            };
        return Result {
            damage,
            first_damage: if first(a) { a.power.max(0) } else { 0 },
            gain: if has(a, KeywordSet::LIFELINK) {
                damage
            } else {
                0
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
            if strikes && !result.attacker_dead {
                incoming += b.power.max(0);
                touch |= b.power > 0 && has(*b, KeywordSet::DEATHTOUCH);
                if has(*b, KeywordSet::LIFELINK) {
                    result.enemy_gain += b.power.max(0);
                }
            }
        }
        if attacks {
            let mut damage = a.power.max(0);
            let mut dealt = 0;
            for (i, b) in blockers.iter().enumerate() {
                if live & (1 << i) == 0 {
                    continue;
                }
                let needed = if has(a, KeywordSet::DEATHTOUCH) {
                    1
                } else {
                    (b.toughness - marked[i]).max(0)
                };
                // Without trample the last blocker receives the excess too.
                // Lifelink counts damage dealt, not just lethal assignment.
                let assigned = if !has(a, KeywordSet::TRAMPLE) && live >> (i + 1) == 0 {
                    damage
                } else {
                    damage.min(needed)
                };
                marked[i] += assigned;
                damage -= assigned;
                dealt += assigned;
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
                dealt += damage;
            }
            if has(a, KeywordSet::LIFELINK) {
                result.gain += dealt;
            }
        }
        a_damage += incoming;
        result.attacker_dead |= lethal(a_damage, touch, a);
        if early {
            result.first_damage = result.damage;
            result.first_enemy_gain = result.enemy_gain;
        }
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

struct Counterblock {
    used: u32,
    // Commander lethal, net damage, material lost, then material committed.
    value: (bool, i32, i64, i64),
}

struct Counterattack {
    casualty: u32,
    choices: Vec<Counterblock>,
}

impl Counterattack {
    /// These exchanges depend on public characteristics, not on the root
    /// attack. Cache them once; a leaf only filters out unavailable blockers.
    fn new(casualty: u32, enemy: Fighter, defenders: &[Fighter], commander_remaining: i32) -> Self {
        let mut choices = vec![Counterblock {
            used: 0,
            value: (
                fight(enemy, &[], 0).damage >= commander_remaining,
                fight(enemy, &[], 0).damage,
                0,
                0,
            ),
        }];
        for (i, &a) in defenders.iter().enumerate() {
            if !could_block(enemy, a) {
                continue;
            }
            if !has(enemy, KeywordSet::MENACE) {
                let r = fight(enemy, &[a], 1);
                choices.push(Counterblock {
                    used: 1 << i,
                    value: (
                        r.damage >= commander_remaining,
                        r.damage - r.enemy_gain,
                        r.material,
                        worth(a),
                    ),
                });
            }
            if has(enemy, KeywordSet::MENACE.union(KeywordSet::TRAMPLE)) {
                for (j, &b) in defenders.iter().enumerate().skip(i + 1) {
                    if !could_block(enemy, b) {
                        continue;
                    }
                    let r = fight(enemy, &[a, b], 3);
                    choices.push(Counterblock {
                        used: (1 << i) | (1 << j),
                        value: (
                            r.damage >= commander_remaining,
                            r.damage - r.enemy_gain,
                            r.material,
                            worth(a) + worth(b),
                        ),
                    });
                }
            }
        }
        // Stable ties preserve the uncached evaluator's declaration order.
        choices.sort_by_key(|choice| choice.value);
        Self { casualty, choices }
    }
}

/// CR 903.10a counts each commander's combat damage independently. The
/// view's history is keyed by the commander's persistent object handle.
fn commander_remaining(view: &PlayerView, victim: PlayerId, source: ObjectId) -> i32 {
    if !view.object(source).is_some_and(|o| o.commander) {
        return i32::MAX;
    }
    21 - view
        .seat(victim)
        .and_then(|s| s.commander_damage.iter().find(|d| d.source == source))
        .map_or(0, |d| i32::from(d.amount))
}

struct Position {
    attackers: Vec<Fighter>,
    blockers: Vec<Fighter>,
    // Includes tapped opponents: they untap before the retaliation.
    retaliation: Vec<Counterattack>,
    exchanges: Vec<Result>,
    player_damage: u32,
    commander_remaining: Vec<i32>,
    can_block: Vec<u32>,
    life: i32,
    enemy_life: i32,
    horizon: u8,
}

impl Position {
    /// Up to 16 * 256 exchanges, built once. Reply trees reuse the same
    /// attacker/blocker subset many times; visiting a node needs one lookup.
    fn cached(mut self) -> Self {
        if self.blockers.len() <= 8 {
            let count = 1 << self.blockers.len();
            self.exchanges.reserve(self.attackers.len() * count);
            for attacker in 0..self.attackers.len() {
                for mask in 0..count {
                    self.exchanges
                        .push(self.uncached_exchange(attacker, u32::try_from(mask).unwrap_or(0)));
                }
            }
        }
        self
    }

    fn exchange(&self, attacker: usize, mask: u32) -> Result {
        if self.exchanges.is_empty() {
            self.uncached_exchange(attacker, mask)
        } else {
            self.exchanges[(attacker << self.blockers.len()) + usize::try_from(mask).unwrap_or(0)]
        }
    }

    fn uncached_exchange(&self, attacker: usize, mask: u32) -> Result {
        let mut result = fight(self.attackers[attacker], &self.blockers, mask);
        // The defending player and commander history are fixed for the whole
        // decision. Cache these along with the exchange, not at every node.
        if self.player_damage & (1 << attacker) == 0 {
            result.damage = 0;
            result.first_damage = 0;
        }
        result.commander_lethal = result.damage >= self.commander_remaining[attacker];
        result
    }

    fn score(&self, going: u32, outcomes: &[Result; MAX], balance: Balance) -> i64 {
        if balance.kills(self.enemy_life) {
            return WIN + balance.material;
        }
        let mut value = balance.material
            + i64::from(balance.damage) * 45
            + i64::from(balance.gain - balance.enemy_gain) * 35;
        if self.horizon < 2 {
            return value;
        }
        let (commander_lethal, incoming, losses) = self.retaliation(going, outcomes);
        if commander_lethal || incoming >= self.life + balance.gain {
            return -WIN + value;
        }
        value -= i64::from(incoming) * 50 + losses / 2;
        value
    }

    /// A conservative continuation: enemies may decline exchanges that would
    /// feed us life. Evasion goes first, and menace needs two actual blockers.
    fn retaliation(&self, going: u32, outcomes: &[Result; MAX]) -> (bool, i32, i64) {
        let dead = outcomes.iter().fold(0, |m, r| m | r.dead);
        let mut used = 0_u32;
        for (i, f) in self.attackers.iter().enumerate() {
            if outcomes[i].attacker_dead
                || (going & (1 << i) != 0 && !has(*f, KeywordSet::VIGILANCE))
            {
                used |= 1 << i;
            }
        }
        let mut incoming = 0;
        let mut losses = 0;
        let mut commander_lethal = false;
        for enemy in &self.retaliation {
            if dead & enemy.casualty != 0 {
                continue;
            }
            // Not blocking is always present, so at least one choice fits.
            if let Some(best) = enemy.choices.iter().find(|choice| choice.used & used == 0) {
                used |= best.used;
                commander_lethal |= best.value.0;
                incoming += best.value.1.max(0);
                losses += best.value.2.max(0);
            }
        }
        (commander_lethal, incoming, losses)
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
    all_lethal: bool,
}

impl Reply<'_> {
    fn visit(&mut self, blocker: usize, balance: Balance) {
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
            self.all_lethal &= balance.kills(self.position.enemy_life);
            let score = self.position.score(self.going, &self.outcomes, balance);
            if score < self.worst {
                self.worst = score;
                self.best_groups = self.groups;
            }
            return;
        }
        self.visit(blocker + 1, balance);
        let choices = self.position.can_block[blocker] & self.going;
        for i in 0..self.position.attackers.len() {
            if choices & (1 << i) == 0 || !self.complete {
                continue;
            }
            let old = self.outcomes[i];
            self.groups[i] |= 1 << blocker;
            let new = self.position.exchange(i, self.groups[i]);
            self.outcomes[i] = new;
            self.visit(blocker + 1, balance.replacing(old, new));
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
    /// Every modeled blocking reply to the chosen attack loses the defender.
    pub lethal: bool,
}

#[allow(clippy::too_many_lines)] // construct one bounded combat position and its caches
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
    if defending.len() > MAX {
        return None;
    }
    let refreshed = |o: &baylee_view::PublicObject, mut f: Fighter| {
        f.toughness += i32::from(o.damage);
        f
    };
    // Offered attackers keep their indices, followed by untapped reserves.
    // Summoning sickness and defender do not stop a creature from blocking.
    let mut defenders: Vec<_> = squad
        .iter()
        .zip(&fighters)
        .filter_map(|(&id, &f)| view.object(id).map(|o| refreshed(o, f)))
        .collect();
    defenders.extend(
        view.battlefield_of(view.seat)
            .filter(|o| {
                o.types.contains(TypeSet::CREATURE)
                    && !o.status.contains(ObjectStatus::TAPPED)
                    && !o.status.contains(ObjectStatus::PHASED_OUT)
                    && !squad.contains(&o.id)
            })
            .filter_map(|o| Fighter::of(view, o.id).map(|f| refreshed(o, f))),
    );
    if defenders.len() > MAX * 2 {
        return None;
    }
    let mut retaliation: Vec<_> = defending
        .iter()
        .enumerate()
        .map(|(i, (o, f))| {
            (
                if i < blockers.len() { 1 << i } else { 0 },
                refreshed(o, *f),
                commander_remaining(view, view.seat, o.id),
            )
        })
        .collect();
    retaliation.sort_by_key(|(_, f, _)| {
        (
            defenders.iter().filter(|&&d| could_block(*f, d)).count(),
            std::cmp::Reverse(f.power),
        )
    });
    Some(
        Position {
            player_damage: (1 << fighters.len()) - 1,
            commander_remaining: squad
                .iter()
                .map(|&id| commander_remaining(view, victim, id))
                .collect(),
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
            retaliation: if profile.lookahead >= 2 {
                retaliation
                    .into_iter()
                    .filter(|(_, f, _)| !has(*f, KeywordSet::DEFENDER))
                    .map(|(mask, f, remaining)| Counterattack::new(mask, f, &defenders, remaining))
                    .collect()
            } else {
                Vec::new()
            },
            exchanges: Vec::new(),
            life: view.seat(view.seat).map_or(20, |s| s.life),
            enemy_life: view.seat(victim).map_or(20, |s| s.life),
            horizon: profile.lookahead.min(2),
        }
        .cached(),
    )
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
        lethal: false,
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
    let mut lethal = false;
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
            limit: (profile.node_budget() - result.nodes).min(if profile.lookahead >= 2 {
                profile.node_budget() * 3 / 4
            } else {
                profile.node_budget() / 4
            }),
            complete: true,
            all_lethal: true,
        };
        let mut balance = Balance::default();
        for i in 0..position.attackers.len() {
            if going & (1 << i) != 0 {
                reply.outcomes[i] = position.exchange(i, 0);
                balance = balance.replacing(Result::default(), reply.outcomes[i]);
            }
        }
        reply.visit(0, balance);
        result.nodes += reply.nodes;
        if going == greedy {
            fallback_upper = reply.worst;
        }
        if reply.complete {
            result.completed += 1;
            if reply.worst > best {
                best = reply.worst;
                chosen = going;
                lethal = reply.all_lethal;
            }
        }
    }
    // A partial reply tree gives an upper bound, never proof that standing
    // still beats the greedy attack. Preserve the fallback until a real
    // reply refutes it or a completed candidate beats that upper bound.
    if result.completed > 0 && best >= fallback_upper {
        result.lethal = lethal;
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
    // Out of the engine's own pairings, not out of the view's combat block:
    // the offer is the authority on what is attacking this seat, and it is
    // the one source that survives a view the agent cannot read the attack
    // out of. In a healthy game the two name the same creatures.
    let ids = crate::combat::deduped(options.iter().flat_map(|o| o.attackers.iter().copied()));
    let attackers: Vec<_> = ids.iter().filter_map(|&id| Fighter::of(view, id)).collect();
    let defenders: Vec<_> = options
        .iter()
        .filter_map(|o| Fighter::of(view, o.blocker))
        .collect();
    // The search needs a body for every attacker and needs to know what each
    // one is aiming at; `position` below reads both. When the view supplies
    // neither, this is not a smaller search but a different question, and
    // `choose_blocks` is the half that answers it — which is what the length
    // compare was always for, and could not do while the fallback shared this
    // function's blind spot.
    let described = ids
        .iter()
        .all(|id| view.combat.attackers.iter().any(|a| a.creature == *id));
    if ids.len() > MAX
        || !described
        || attackers.len() != ids.len()
        || defenders.len() != options.len()
    {
        return crate::combat::choose_blocks(view, options, life);
    }
    let unblockable = view
        .combat
        .attackers
        .iter()
        .filter(|a| {
            !ids.contains(&a.creature)
                && a.defending == baylee_core::ids::Defender::Player(view.seat)
        })
        .filter_map(|a| Fighter::of(view, a.creature).map(|f| (a.creature, f)))
        .fold(Balance::default(), |balance, (id, f)| {
            let mut result = fight(f, &[], 0);
            result.commander_lethal = result.damage >= commander_remaining(view, view.seat, id);
            balance.replacing(Result::default(), result)
        });
    let position = Position {
        commander_remaining: ids
            .iter()
            .map(|&id| commander_remaining(view, view.seat, id))
            .collect(),
        player_damage: ids.iter().enumerate().fold(0, |mask, (i, id)| {
            mask | if view.combat.attackers.iter().any(|a| {
                a.creature == *id && a.defending == baylee_core::ids::Defender::Player(view.seat)
            }) {
                1 << i
            } else {
                0
            }
        }),
        attackers,
        blockers: defenders,
        retaliation: vec![],
        exchanges: Vec::new(),
        can_block: options
            .iter()
            .map(|o| {
                ids.iter().enumerate().fold(0, |mask, (i, id)| {
                    mask | if o.attackers.contains(id) { 1 << i } else { 0 }
                })
            })
            .collect(),
        life: i32::MAX,
        enemy_life: life,
        horizon: 0,
    }
    .cached();
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
        all_lethal: true,
    };
    let mut balance = unblockable;
    for i in 0..position.attackers.len() {
        reply.outcomes[i] = position.exchange(i, 0);
        balance = balance.replacing(Result::default(), reply.outcomes[i]);
    }
    reply.visit(0, balance);
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

    #[test]
    fn lifelink_counts_excess_damage_but_needs_a_living_recipient() {
        let result = fight(
            fighter(8, 8, KeywordSet::LIFELINK),
            &[fighter(1, 1, KeywordSet::EMPTY)],
            1,
        );
        assert_eq!(result.damage, 0);
        assert_eq!(result.gain, 8);
        let result = fight(
            fighter(1, 1, KeywordSet::EMPTY),
            &[fighter(
                8,
                8,
                KeywordSet::LIFELINK.union(KeywordSet::DOUBLE_STRIKE),
            )],
            1,
        );
        assert!(result.attacker_dead);
        assert_eq!(
            result.enemy_gain, 8,
            "no attacker remains for the normal damage step"
        );
        assert_eq!(result.first_enemy_gain, 8);
    }
}
