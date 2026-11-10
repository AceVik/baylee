//! When an instant is cast ([`AIProfile::INSTANT_TIMING`]).
//!
//! The house used to cast every instant at the first priority it held one
//! it could pay for: a Titanic Growth in the opponent's upkeep on whatever
//! creature ranked first, a Plummet in its own draw step. And `steady`
//! kept two mana up for that Titanic Growth instead of casting a creature,
//! only to throw the card away in the next upkeep: the ablation without the
//! hold-up beat the level with it (`docs/proposals/house-ai-strength.md`).
//!
//! A level that manages its open mana ([`HoldUp`] other than `None`, which
//! the profile already describes as "how carefully an AI manages open mana
//! and instant-speed plays") now asks what an instant is for, and casts it
//! when that can happen:
//!
//! - a **counter** when something worth countering is on the stack, as
//!   before (`policy::spell_score`);
//! - a **combat trick**, a boon to one creature until the turn or the
//!   combat ends, once blocks are declared and only where it turns an
//!   exchange: a creature of this side that would die survives, or one of
//!   the other side's that would live dies, or the attack becomes lethal.
//!   Its target is the creature it turns ([`HeuristicAgent::trick_target`]);
//! - anything **lasting** (removal, damage, a draw, a token, a flash
//!   creature) in this seat's own main phase, or in an opponent's turn once
//!   they attack or in their end step; never in an upkeep or a draw step,
//!   where nothing has happened that it could answer.
//!
//! Only instant-speed cards are timed; the engine offers a sorcery-speed
//! one in a main phase alone. Levels that tap out (`novice`, `casual`) keep
//! casting at the first chance, and the frozen reference never meets this.

use baylee_cards_dsl::{AbilityDef, Duration, Effect, KeywordSet};
use baylee_core::ids::{ObjectId, PlayerId};
use baylee_core::preset::{AIProfile, HoldUp};
use baylee_view::{Phase, PlayerView, PublicObject, Step};

use crate::HeuristicAgent;
use crate::combat::{Fighter, exchange};
use crate::tactics::{amount, material};

/// What an instant is for, read off what it prints.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Purpose {
    /// It counters a spell or an ability: its moment is a target on the
    /// stack, which the spell's own score already asks for.
    Counter,
    /// It gives one creature a boon until the turn or the combat ends, and
    /// nothing that outlasts it but a card drawn or a look at the library.
    Trick(Boon),
    /// Anything else: what it does stays done.
    Lasting,
}

/// What a combat trick adds to the creature it targets.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Boon {
    pub power: i32,
    pub toughness: i32,
    pub keywords: u128,
}

impl Boon {
    fn is_some(self) -> bool {
        self.power > 0 || self.toughness > 0 || self.keywords != 0
    }

    fn on(self, f: Fighter) -> Fighter {
        Fighter {
            power: f.power + self.power,
            toughness: f.toughness + self.toughness,
            keywords: f.keywords | self.keywords,
            worth: f.worth,
        }
    }
}

/// A trick has to turn at least this much to be cast: about a one-mana
/// creature saved or killed (`tactics::material` of a 1/1 for one is 410).
/// Damage alone never reaches it unless it ends the game.
const TRICK_FLOOR: i64 = 300;

/// What turning an attack lethal is worth.
const LETHAL: i64 = 100_000;

/// The purpose of a spell that prints `effects`, at `x`.
pub(crate) fn purpose(effects: &[Effect], x: u32) -> Purpose {
    let mut boon = Boon::default();
    let mut lasting = false;
    let mut counter = false;
    read(effects, x, &mut boon, &mut lasting, &mut counter);
    if counter {
        Purpose::Counter
    } else if boon.is_some() && !lasting {
        Purpose::Trick(boon)
    } else {
        Purpose::Lasting
    }
}

fn read(effects: &[Effect], x: u32, boon: &mut Boon, lasting: &mut bool, counter: &mut bool) {
    for effect in effects {
        match effect {
            Effect::CounterTargetSpell
            | Effect::CounterTargetSpellToExile
            | Effect::CounterTargetAbility
            | Effect::CounterTargetSpellOrAbility => *counter = true,
            Effect::PumpTarget {
                power,
                toughness,
                keywords,
                duration: Duration::UntilEndOfTurn | Duration::UntilEndOfCombat,
            } => {
                let (power, toughness) = (amount(*power, x), amount(*toughness, x));
                if power < 0 || toughness < 0 {
                    *lasting = true;
                } else {
                    boon.power += power;
                    boon.toughness += toughness;
                    boon.keywords |= keywords.bits();
                }
            }
            // What a trick may also do: a card back, a look at the library,
            // the creature untapped. None of it asks for a moment of its own.
            Effect::DrawCards { .. }
            | Effect::Scry { .. }
            | Effect::Surveil { .. }
            | Effect::UntapTarget => {}
            other => {
                let (then, otherwise) = other.branches();
                if then.is_empty() && otherwise.is_empty() {
                    *lasting = true;
                } else {
                    read(then, x, boon, lasting, counter);
                    read(otherwise, x, boon, lasting, counter);
                }
            }
        }
    }
}

/// The effects a card prints when it is cast (its `Spell` abilities); a
/// modal spell has none here and so reads as lasting.
pub(crate) fn cast_effects(spells: &[AbilityDef]) -> Option<&'static [Effect]> {
    spells.iter().find_map(|ability| match ability {
        AbilityDef::Spell { effects, .. } => Some(*effects),
        _ => None,
    })
}

/// One attacker and what blocks it, as the view shows them.
struct Clash<'a> {
    attacker: &'a PublicObject,
    blockers: Vec<&'a PublicObject>,
}

/// Who dies in a clash: the attacker, and each blocker in order. A lone
/// blocker meets the attacker as [`exchange`] has it, first strike
/// included; several are read as one damage pool against the attacker,
/// which assigns its own damage to kill as many of them as it can, the
/// toughest last (CR 510.1c).
fn deaths(attacker: Fighter, blockers: &[Fighter]) -> (bool, Vec<bool>) {
    if let [blocker] = blockers {
        let result = exchange(attacker, *blocker);
        return (result.attacker_dies, vec![result.blocker_dies]);
    }
    let indestructible = |f: Fighter| f.has(KeywordSet::INDESTRUCTIBLE);
    let pool: i32 = blockers.iter().map(|b| b.power.max(0)).sum();
    let touch = blockers
        .iter()
        .any(|b| b.power > 0 && b.has(KeywordSet::DEATHTOUCH));
    let attacker_dies =
        !indestructible(attacker) && (touch || (pool > 0 && pool >= attacker.toughness));
    let mut order: Vec<usize> = (0..blockers.len()).collect();
    order.sort_by_key(|&i| (blockers[i].toughness, i));
    let mut left = attacker.power.max(0);
    let mut dies = vec![false; blockers.len()];
    for i in order {
        let blocker = blockers[i];
        let needed = if attacker.has(KeywordSet::DEATHTOUCH) {
            1
        } else {
            blocker.toughness.max(1)
        };
        if left >= needed && !indestructible(blocker) {
            dies[i] = true;
            left -= needed;
        }
    }
    (attacker_dies, dies)
}

impl HeuristicAgent {
    /// Whether this level times its instants: one that manages its open
    /// mana, from [`AIProfile::INSTANT_TIMING`].
    pub(crate) fn times_instants(&self) -> bool {
        self.profile.hold_up != HoldUp::None && self.profile.plays(AIProfile::INSTANT_TIMING)
    }

    /// Whether now is a moment to cast an instant-speed card that prints
    /// `effects` (see the module).
    pub(crate) fn its_moment(&self, view: &PlayerView, effects: &[Effect], x: u32) -> bool {
        match purpose(effects, x) {
            Purpose::Counter => true,
            Purpose::Trick(boon) => self
                .trick_turns(view, boon)
                .is_some_and(|(gain, _)| gain >= TRICK_FLOOR),
            Purpose::Lasting => {
                if view.active == view.seat {
                    matches!(view.phase, Phase::FirstMain | Phase::SecondMain)
                } else {
                    view.combat.is_active() || view.step == Step::End
                }
            }
        }
    }

    /// The creature of this side a trick with `boon` turns the most for,
    /// and what it turns, once blocks are declared; `None` before them or
    /// where it turns nothing.
    pub(crate) fn trick_turns(&self, view: &PlayerView, boon: Boon) -> Option<(i64, ObjectId)> {
        self.trick_gains(view, boon)
            .into_iter()
            .max_by_key(|&(gain, id)| (gain, std::cmp::Reverse(id)))
            .filter(|&(gain, _)| gain > 0)
    }

    /// The target a trick is aimed at among `objects`: the creature it
    /// turns the most for, when it turns anything.
    pub(crate) fn trick_target(
        &self,
        view: &PlayerView,
        objects: &[ObjectId],
        boon: Boon,
    ) -> Option<ObjectId> {
        self.trick_gains(view, boon)
            .into_iter()
            .filter(|(gain, id)| *gain > 0 && objects.contains(id))
            .max_by_key(|&(gain, id)| (gain, std::cmp::Reverse(id)))
            .map(|(_, id)| id)
    }

    /// Every creature of this side in a declared combat, with what `boon`
    /// on it would turn.
    fn trick_gains(&self, view: &PlayerView, boon: Boon) -> Vec<(i64, ObjectId)> {
        if view.step != Step::DeclareBlockers {
            return Vec::new();
        }
        let ours = |o: &PublicObject| !self.hostile(o.controller, view.seat);
        let clashes: Vec<Clash<'_>> = view
            .combat
            .attackers
            .iter()
            .filter_map(|a| {
                Some(Clash {
                    attacker: view.object(a.creature)?,
                    blockers: view
                        .combat
                        .blockers_of(a.creature)
                        .filter_map(|b| view.object(b))
                        .collect(),
                })
            })
            .collect();
        let mut gains = Vec::new();
        for (index, clash) in clashes.iter().enumerate() {
            let Some(attacker) = Fighter::from_object(clash.attacker) else {
                continue;
            };
            let Some(blockers) = clash
                .blockers
                .iter()
                .map(|b| Fighter::from_object(b))
                .collect::<Option<Vec<_>>>()
            else {
                continue;
            };
            let before = deaths(attacker, &blockers);
            if ours(clash.attacker) {
                let gain = if blockers.is_empty() {
                    unblocked_gain(view, &clashes, index, boon)
                } else {
                    let after = deaths(boon.on(attacker), &blockers);
                    turned(clash, &before, &after)
                };
                gains.push((gain, clash.attacker.id));
            }
            for (i, blocker) in clash.blockers.iter().enumerate() {
                if !ours(blocker) {
                    continue;
                }
                let mut boosted = blockers.clone();
                boosted[i] = boon.on(boosted[i]);
                let after = deaths(attacker, &boosted);
                // The attacker's damage goes where the attacking player
                // wants it, so a blocker it can kill is counted dead
                // whatever the pool says.
                let dies = |f: Fighter| {
                    !f.has(KeywordSet::INDESTRUCTIBLE)
                        && attacker.power > 0
                        && (attacker.power >= f.toughness || attacker.has(KeywordSet::DEATHTOUCH))
                };
                let saved = dies(blockers[i]) && !dies(boosted[i]);
                let killed = !before.0 && after.0;
                let gain = if saved { material(blocker) } else { 0 }
                    + if killed { material(clash.attacker) } else { 0 };
                gains.push((gain, blocker.id));
            }
        }
        gains
    }

    /// The target a combat trick goes to when this seat answers a target
    /// question for one (`tactics::targets`): the creature whose exchange
    /// it turns, which is the one it was cast for, not the one ranked
    /// first. `None` for anything that is not a trick, or turns nothing.
    pub(crate) fn trick_answer(
        &self,
        view: &PlayerView,
        objects: &[ObjectId],
        (min, max): (u32, u32),
        context: &baylee_engine::engine::DecisionContext<'_>,
    ) -> Option<baylee_engine::choice::PlayerAction> {
        if !self.times_instants() || min > 1 || max == 0 {
            return None;
        }
        let Purpose::Trick(boon) = purpose(context.effects, context.x) else {
            return None;
        };
        let id = self.trick_target(view, objects, boon)?;
        Some(baylee_engine::choice::PlayerAction::ChooseTargets {
            objects: vec![id],
            players: vec![],
        })
    }
}

/// What `boon` on an unblocked attacker of this side turns: only an attack
/// it makes lethal, since a few points of damage are not worth a card.
fn unblocked_gain(view: &PlayerView, clashes: &[Clash<'_>], index: usize, boon: Boon) -> i64 {
    let Some(victim) = defending_player(view, clashes[index].attacker.id) else {
        return 0;
    };
    let life = view.seat(victim).map_or(i32::MAX, |s| s.life);
    let through: i32 = clashes
        .iter()
        .filter(|c| c.blockers.is_empty())
        .filter(|c| defending_player(view, c.attacker.id) == Some(victim))
        .filter_map(|c| Fighter::from_object(c.attacker))
        .map(|f| {
            if f.has(KeywordSet::DOUBLE_STRIKE) {
                2 * f.power.max(0)
            } else {
                f.power.max(0)
            }
        })
        .sum();
    let added = if Fighter::from_object(clashes[index].attacker)
        .is_some_and(|f| f.has(KeywordSet::DOUBLE_STRIKE))
    {
        2 * boon.power
    } else {
        boon.power
    };
    if through < life && through + added >= life {
        LETHAL
    } else {
        0
    }
}

/// What a boon on this side's attacker in `clash` turned: the attacker
/// saved, and each blocker it now kills.
fn turned(clash: &Clash<'_>, before: &(bool, Vec<bool>), after: &(bool, Vec<bool>)) -> i64 {
    let saved = if before.0 && !after.0 {
        material(clash.attacker)
    } else {
        0
    };
    let killed: i64 = clash
        .blockers
        .iter()
        .enumerate()
        .filter(|&(i, _)| !before.1[i] && after.1[i])
        .map(|(_, b)| material(b))
        .sum();
    saved + killed
}

/// The player an attacker is attacking, or whose planeswalker it attacks.
fn defending_player(view: &PlayerView, attacker: ObjectId) -> Option<PlayerId> {
    let attack = view
        .combat
        .attackers
        .iter()
        .find(|a| a.creature == attacker)?;
    match attack.defending {
        baylee_core::ids::Defender::Player(p) => Some(p),
        baylee_core::ids::Defender::Planeswalker(_) | baylee_core::ids::Defender::Battle(_) => None,
    }
}
