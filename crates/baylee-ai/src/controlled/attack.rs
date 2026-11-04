//! Which of a controlled player's creatures attack, and at whom (CR 723.5).

use baylee_cards_dsl::{AbilityDef, KeywordSet, Trigger, ZoneRef};
use baylee_core::ids::{Defender, ObjectId, PlayerId};
use baylee_core::types::TypeSet;
use baylee_engine::choice::AttackLimit;
use baylee_view::PlayerView;

use crate::HeuristicAgent;
use crate::board::Board;
use crate::combat::{self, Fighter};

/// What lets an attacker past a block the [`Fighter`] model does not see
/// (fear, shadow, landwalk, protection), or makes the block cost the
/// blocker more than the exchange says (infect and wither leave their
/// counters, flanking shrinks it, menace needs two): an attacker carrying
/// one is never sent to be blocked.
const UNREAD: u128 = KeywordSet::MENACE
    .union(KeywordSet::FEAR)
    .union(KeywordSet::INTIMIDATE)
    .union(KeywordSet::SHADOW)
    .union(KeywordSet::HORSEMANSHIP)
    .union(KeywordSet::SKULK)
    .union(KeywordSet::PLAINSWALK)
    .union(KeywordSet::ISLANDWALK)
    .union(KeywordSet::SWAMPWALK)
    .union(KeywordSet::MOUNTAINWALK)
    .union(KeywordSet::FORESTWALK)
    .union(KeywordSet::PROTECTION_BLACK)
    .union(KeywordSet::FLANKING)
    .union(KeywordSet::BANDING)
    .union(KeywordSet::INFECT)
    .union(KeywordSet::WITHER)
    .bits();

/// What makes an unblocked attacker cost this seat more than its power in
/// life, or its attack buy nothing: life gained, poison, and vigilance,
/// under which it never taps (CR 702.20b).
const COSTLY: u128 = KeywordSet::LIFELINK
    .union(KeywordSet::INFECT)
    .union(KeywordSet::VIGILANCE)
    .bits();

/// How many of their creatures the tap-down pass weighs, each against the
/// whole board: enough for any table a person plays at, and a bound on a
/// board of ten thousand tokens.
const WEIGHED: usize = 16;

impl HeuristicAgent {
    /// The attack this seat declares for `subject`, the player whose turn it
    /// controls, before the rules add what must attack
    /// ([`combat::obey_attack_rules`]).
    ///
    /// Two kinds of creature go, both at this seat, which then blocks with
    /// its own judgement (CR 723.8):
    ///
    /// - **One this seat eats.** An untapped creature of this seat's can
    ///   block it, kill it and live, by the same exchange its own attacks
    ///   are judged by; their best creatures are matched first, each
    ///   blocker once.
    /// - **One worth tapping.** Their creatures untap only in their own
    ///   untap step (CR 502.3), so one that attacks is not there to block
    ///   this seat's next attack. It goes when that lets more of this seat's
    ///   next attack through than it deals now, or lets enough through to
    ///   kill them, and only while everything sent this way leaves this
    ///   seat above ten life and loses it no more than a quarter of it.
    ///   Nothing with lifelink, infect or vigilance, which tapping does not
    ///   buy.
    ///
    /// Nothing goes whose attack sets off a trigger, its own or another of
    /// theirs, since what the trigger does is not read here; and nothing at
    /// one of this seat's planeswalkers. What must attack and is in neither
    /// group goes where it costs this seat least: at another opponent of
    /// this seat's where it may, else at this seat, where it can be blocked.
    pub(crate) fn attack_against(
        &self,
        view: &PlayerView,
        subject: PlayerId,
        squad: &[ObjectId],
        defenders: &[Defender],
        required: &[ObjectId],
        limits: &[AttackLimit],
    ) -> Vec<(ObjectId, Defender)> {
        let me = view.seat;
        let home = Defender::Player(me);
        let allowed = |creature: ObjectId| -> &[Defender] {
            limits
                .iter()
                .find(|l| l.creature == creature)
                .map_or(defenders, |l| l.defenders.as_slice())
        };
        let board = Board::new(view);
        let mut candidates: Vec<(ObjectId, Fighter)> = squad
            .iter()
            .copied()
            .filter(|&id| allowed(id).contains(&home) && !self.attack_sets_off(view, id))
            .filter_map(|id| board.fighter(id).map(|f| (id, f)))
            .collect();
        candidates.sort_by_key(|(id, f)| {
            (
                std::cmp::Reverse(f.worth),
                std::cmp::Reverse(f.power),
                id.slot(),
            )
        });
        let eaten = eaten(view, &candidates);
        let tapped = Self::tapped_down(view, subject, &candidates, &eaten);
        let mut going: Vec<(ObjectId, Defender)> = eaten
            .into_iter()
            .chain(tapped)
            .map(|id| (id, home))
            .collect();
        for &creature in required {
            if going.iter().any(|(id, _)| *id == creature) {
                continue;
            }
            let may = allowed(creature);
            let elsewhere = may
                .iter()
                .copied()
                .find(|d| matches!(d, Defender::Player(p) if *p != me && self.hostile(*p, me)));
            if let Some(at) = elsewhere
                .or_else(|| may.contains(&home).then_some(home))
                .or_else(|| may.first().copied())
            {
                going.push((creature, at));
            }
        }
        going
    }

    /// One share of their creature's combat damage divided among this
    /// seat's creatures (CR 510.1c–d): as much as `recipient` survives, so
    /// the fewest die, and the last one, which the engine does not ask
    /// about, takes the rest. An indestructible creature takes everything it
    /// is offered; nothing survives a deathtouch share (CR 702.2c), so it is
    /// given none. `None` for a share to a creature of their own side.
    pub(crate) fn share_against(
        &self,
        view: &PlayerView,
        source: ObjectId,
        recipient: ObjectId,
        min: u32,
        max: u32,
    ) -> Option<u32> {
        let o = view.object(recipient)?;
        if self.hostile(o.controller, view.seat) {
            return None;
        }
        let deathtouch = view
            .object(source)
            .is_some_and(|s| s.keywords & KeywordSet::DEATHTOUCH.bits() != 0);
        let spare = if o.keywords & KeywordSet::INDESTRUCTIBLE.bits() != 0 {
            max
        } else if deathtouch {
            0
        } else {
            u32::try_from(crate::tactics::damage_to_finish(o) - 1).unwrap_or(0)
        };
        Some(spare.clamp(min, max))
    }

    /// Their creatures worth tapping by an attack this seat lets through,
    /// out of `candidates` that are not `eaten`.
    fn tapped_down(
        view: &PlayerView,
        subject: PlayerId,
        candidates: &[(ObjectId, Fighter)],
        eaten: &[ObjectId],
    ) -> Vec<ObjectId> {
        let me = view.seat;
        let (Some(life), Some(theirs)) = (
            view.seat(me).map(|s| s.life),
            view.seat(subject).map(|s| s.life),
        ) else {
            return Vec::new();
        };
        let budget = (life - 10).min(life / 4).max(0);
        // What this seat swings with next turn: everything it has, untapped
        // by then (CR 502.3), none of it summoning sick (CR 302.6).
        let next: Vec<Fighter> = combat::creatures(view, me, |_| true)
            .into_iter()
            .filter(|f| {
                !f.has(KeywordSet::DEFENDER) && !f.has(KeywordSet::CANT_ATTACK) && f.power > 0
            })
            .collect();
        if budget == 0 || next.is_empty() {
            return Vec::new();
        }
        // What stands in front of it: theirs that are untapped now, less
        // the ones this seat's blocks kill.
        let mut blocking: Vec<(ObjectId, Fighter)> = view
            .battlefield_of(subject)
            .filter(|o| {
                o.types.contains(TypeSet::CREATURE) && combat::ready(o) && !eaten.contains(&o.id)
            })
            .filter_map(|o| {
                Fighter::from_object(o).map(|mut f| {
                    f.toughness += i32::from(o.damage);
                    (o.id, f)
                })
            })
            .collect();
        let through = |blocking: &[(ObjectId, Fighter)]| {
            let walls: Vec<Fighter> = blocking.iter().map(|(_, f)| *f).collect();
            combat::through(&next, &walls)
        };
        let mut weighed: Vec<(ObjectId, Fighter)> = candidates
            .iter()
            .copied()
            .filter(|(id, f)| !eaten.contains(id) && f.keywords & COSTLY == 0)
            .take(WEIGHED)
            .collect();
        weighed.sort_by_key(|(id, f)| (f.power, f.worth, id.slot()));
        let mut spent = 0;
        let mut tapped = Vec::new();
        for (id, attacker) in weighed {
            let damage = if attacker.has(KeywordSet::DOUBLE_STRIKE) {
                attacker.power * 2
            } else {
                attacker.power
            }
            .max(0);
            if spent + damage > budget {
                continue;
            }
            let before = through(&blocking);
            let rest: Vec<(ObjectId, Fighter)> =
                blocking.iter().copied().filter(|(b, _)| *b != id).collect();
            let after = through(&rest);
            if after - before > damage || after >= theirs && before < theirs {
                spent += damage;
                tapped.push(id);
                blocking = rest;
            }
        }
        tapped
    }

    /// Whether `attacker` attacking sets off a trigger: one of its own about
    /// attacking or being blocked, or another of its controller's about a
    /// creature like it attacking. An unreadable filter counts as a match.
    fn attack_sets_off(&self, view: &PlayerView, attacker: ObjectId) -> bool {
        let Some(a) = view.object(attacker) else {
            return true;
        };
        view.battlefield_of(a.controller).any(|o| {
            crate::activate::printed_list(o).iter().any(|ability| {
                let (AbilityDef::Triggered { trigger, .. }
                | AbilityDef::ModalTriggered { trigger, .. }) = ability
                else {
                    return false;
                };
                match trigger {
                    Trigger::Attacks(filter)
                    | Trigger::OneOrMoreAttack(filter)
                    | Trigger::AttacksAndIsntBlocked(filter)
                    | Trigger::AttacksAlone(filter) => {
                        self.filter_matches(filter, view, a, ZoneRef::Battlefield, Some(o.id))
                            != Some(false)
                    }
                    Trigger::AttackWithAtLeast(_) | Trigger::AttackWithAtLeastMatching { .. } => {
                        true
                    }
                    Trigger::BecomesBlocked
                    | Trigger::BecomesBlockedBy(_)
                    | Trigger::Rampage
                    | Trigger::BlocksOrBecomesBlockedBy(_)
                    | Trigger::BlocksOrBecomesBlockedByOneOrMore(_) => o.id == a.id,
                    _ => false,
                }
            })
        })
    }
}

/// Their creatures out of `candidates`, best first, that an untapped
/// creature of this seat's can block, kill and live through, each blocker
/// matched once and the cheapest that does it taken.
fn eaten(view: &PlayerView, candidates: &[(ObjectId, Fighter)]) -> Vec<ObjectId> {
    let mut blockers: Vec<Fighter> = view
        .battlefield_of(view.seat)
        .filter(|o| o.types.contains(TypeSet::CREATURE) && combat::ready(o))
        .filter_map(Fighter::from_object)
        .filter(|f| !f.has(KeywordSet::CANT_BLOCK))
        .collect();
    blockers.sort_by_key(|f| (f.worth, f.power));
    let mut eaten = Vec::new();
    for &(id, attacker) in candidates {
        if attacker.keywords & UNREAD != 0 {
            continue;
        }
        if let Some(i) = blockers.iter().position(|b| {
            let e = combat::exchange(attacker, *b);
            combat::could_block(attacker, *b) && e.attacker_dies && !e.blocker_dies
        }) {
            blockers.remove(i);
            eaten.push(id);
        }
    }
    eaten
}
