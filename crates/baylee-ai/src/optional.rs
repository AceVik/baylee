//! The optional and combat-trick answers beta.6 testing found played badly:
//! a "may" taken whatever it did, a Camouflage never cast, a False Orders
//! re-block always declined, and Raging River's and Camouflage's piles split
//! by the order the engine listed them.
//!
//! Each answer reads only the view and the question, and is cheap: a pass
//! over the battlefield at most.

use baylee_cards_dsl::{AbilityDef, Condition, Effect, KeywordSet, ReplacementRule, StepKind};
use baylee_core::ids::{AbilityRef, ObjectId, PlayerId};
use baylee_core::types::TypeSet;
use baylee_view::{PlayerView, PublicObject, Step};

use crate::HeuristicAgent;
use crate::worth::{Aim, Origin};

fn has(o: &PublicObject, k: KeywordSet) -> bool {
    o.keywords & k.bits() != 0
}

fn untapped_creature(o: &PublicObject) -> bool {
    o.types.contains(TypeSet::CREATURE) && !o.status.contains(baylee_view::ObjectStatus::TAPPED)
}

fn power(o: &PublicObject) -> i64 {
    i64::from(o.power.unwrap_or(0)).max(0)
}

/// Whether `a` deals `b` enough damage in a fight between them to destroy it.
fn kills(a: &PublicObject, b: &PublicObject) -> bool {
    power(a) > 0
        && (has(a, KeywordSet::DEATHTOUCH) || power(a) >= crate::tactics::damage_to_finish(b))
        && !has(b, KeywordSet::INDESTRUCTIBLE)
}

/// What a creature is worth as a blocker, for splitting piles: its power
/// and toughness, untapped ones only (a tapped creature blocks nothing).
fn blocker_strength(o: &PublicObject) -> i64 {
    if !untapped_creature(o) {
        return 0;
    }
    1 + power(o) + i64::from(o.toughness.unwrap_or(0)).max(0)
}

/// Whether a spell condition's timing half is false now. Only the parts the
/// view answers for certain are read (whose turn, which step); anything
/// else is assumed to hold, as before, because the engine is the one that
/// knows.
fn closed(view: &PlayerView, condition: &Condition) -> bool {
    match condition {
        Condition::YourTurn => view.active != view.seat,
        Condition::DuringStep(step) => !during(view.step, *step),
        Condition::All(parts) => parts.iter().any(|c| closed(view, c)),
        _ => false,
    }
}

fn during(now: Step, step: StepKind) -> bool {
    match step {
        StepKind::Upkeep => now == Step::Upkeep,
        StepKind::Draw => now == Step::Draw,
        StepKind::CombatBegin => now == Step::CombatBegin,
        StepKind::DeclareAttackers => now == Step::DeclareAttackers,
        StepKind::DeclareBlockers => now == Step::DeclareBlockers,
        StepKind::CombatDamage => matches!(now, Step::CombatDamage | Step::CombatDamageFirst),
        StepKind::CombatEnd => now == Step::CombatEnd,
        StepKind::End => now == Step::End,
    }
}

/// Whether every spell ability of the card is held to a timing that is not
/// now (Camouflage outside its controller's declare attackers step, False
/// Orders outside the declare blockers step). Such a card is not a cast to
/// tap mana for: the engine will not take it, and the mana empties.
pub(crate) fn timing_closed(view: &PlayerView, spells: &[AbilityDef]) -> bool {
    let mut any = false;
    for ability in spells {
        if let AbilityDef::Spell { condition, .. } = ability {
            any = true;
            if !condition.as_ref().is_some_and(|c| closed(view, c)) {
                return false;
            }
        }
    }
    any
}

/// Camouflage ("this turn, instead of declaring blockers, each defending
/// player chooses any number of creatures they control and divides them
/// into a number of piles equal to the number of attacking creatures…"):
/// worth casting only once this seat is attacking, into an opponent who has
/// an untapped creature to block with. Then the blocks are made at random
/// and not chosen, which is what the attacker wants against a defender who
/// would block well.
pub(crate) fn camouflage_worth(view: &PlayerView, hostile: impl Fn(PlayerId) -> bool) -> bool {
    view.active == view.seat
        && view.step == Step::DeclareAttackers
        && view.combat.attackers.iter().any(|a| {
            view.object(a.creature)
                .is_some_and(|o| o.controller == view.seat)
        })
        && view
            .battlefield
            .iter()
            .any(|o| hostile(o.controller) && untapped_creature(o))
}

/// Whether a spell is no cast to make now: held to a step or a turn that is
/// not now ([`timing_closed`]; the engine will not take the cast, and mana
/// tapped for it empties, CR 500.5), or Camouflage with nothing for it to
/// do ([`camouflage_worth`]).
pub(crate) fn not_now(
    view: &PlayerView,
    spells: &[AbilityDef],
    hostile: impl Fn(PlayerId) -> bool,
) -> bool {
    timing_closed(view, spells)
        || spells.iter().any(|a| {
            matches!(a, AbilityDef::Spell { effects, .. }
                if effects.iter().any(|e| matches!(e, Effect::BlockInPilesAtRandomThisTurn)))
        }) && !camouflage_worth(view, hostile)
}

/// The first "you may" among `effects`, looking inside sequences and
/// branches: what a yes would do.
fn may_branch(effects: &[Effect]) -> Option<&'static [Effect]> {
    effects.iter().find_map(|effect| match effect {
        Effect::MayDo { effects } | Effect::MayDoOnceEachTurn { effects } => Some(*effects),
        other => {
            let (then, otherwise) = other.branches();
            may_branch(then).or_else(|| may_branch(otherwise))
        }
    })
}

/// Whether the ability `source` names is Island Sanctuary's "you may skip
/// that draw" replacement.
fn is_draw_skip(source: Option<AbilityRef>) -> bool {
    let Some(source) = source else {
        return false;
    };
    baylee_cards::by_index(source.card).is_some_and(|def| {
        matches!(
            def.abilities.get(source.index as usize),
            Some(AbilityDef::Replacement(
                ReplacementRule::MaySkipDrawStepDraw
            ))
        )
    })
}

/// Island Sanctuary: a card is worth more than the protection, unless the
/// creatures the restriction would keep home (no flying, no islandwalk)
/// threaten this seat's life: their power is half of it or more.
fn skips_draw(view: &PlayerView, hostile: impl Fn(PlayerId) -> bool) -> bool {
    let life = i64::from(view.seat(view.seat).map_or(0, |s| s.life));
    let ground: i64 = view
        .battlefield
        .iter()
        .filter(|o| {
            hostile(o.controller)
                && o.types.contains(TypeSet::CREATURE)
                && !has(o, KeywordSet::FLYING)
                && !has(o, KeywordSet::ISLANDWALK)
        })
        .map(power)
        .sum();
    ground > 0 && ground * 2 >= life
}

impl HeuristicAgent {
    /// A "you may" (`YesNoPrompt::MayDo`): the known prompts by what they
    /// are, the rest by what the yes would do on this board — declined when
    /// it costs this seat, taken when it is free or helps, and taken when
    /// the measure cannot read it (a card this seat chose to play).
    pub(crate) fn may_do(
        &self,
        view: &PlayerView,
        source: Option<AbilityRef>,
        context: &baylee_engine::engine::DecisionContext<'_>,
    ) -> bool {
        let hostile = |seat| self.hostile(seat, view.seat);
        if is_draw_skip(source) {
            return skips_draw(view, hostile);
        }
        let Some(then) = may_branch(context.effects) else {
            return true;
        };
        let Some(id) = context.source else {
            return true;
        };
        // The resolving object's own target, when it has one: what "it" is
        // in the yes.
        let aim = view
            .object(id)
            .and_then(|o| o.targets.first())
            .and_then(|t| match t {
                baylee_core::ids::TargetRef::Object(r) => Some(r.object),
                baylee_core::ids::TargetRef::Player(_) => None,
            })
            .map_or(Aim::Source, Aim::Object);
        self.effects_worth(view, Origin::of(view, id), then, aim, context.x)
            .is_none_or(|worth| worth >= 0)
    }

    /// False Orders' re-block: the attacker `blocker` blocks, if any.
    ///
    /// A creature of this seat's side blocks what it kills and survives, or
    /// what it kills when that attacker is worth more than it. An
    /// opponent's creature (this seat attacked and cast it) is handed an
    /// attacker that kills it and survives and was blocked anyway, so no
    /// damage that was getting through is given up.
    pub(crate) fn reblock(
        &self,
        view: &PlayerView,
        blocker: ObjectId,
        options: &[ObjectId],
    ) -> Vec<ObjectId> {
        let Some(b) = view.object(blocker) else {
            return Vec::new();
        };
        let mine = !self.hostile(b.controller, view.seat);
        let blocked = |a: ObjectId| {
            view.combat
                .attackers
                .iter()
                .any(|v| v.creature == a && v.blocked)
        };
        let mut best: Option<(i64, ObjectId)> = None;
        for &id in options {
            let Some(a) = view.object(id) else { continue };
            let score = if mine {
                if kills(b, a) && !kills(a, b) {
                    1000 + power(a)
                } else if kills(b, a) && power(a) > power(b) {
                    power(a) - power(b)
                } else {
                    continue;
                }
            } else if kills(a, b) && !kills(b, a) && blocked(id) {
                1000 + power(b)
            } else {
                continue;
            };
            if best.is_none_or(|(s, _)| score > s) {
                best = Some((score, id));
            }
        }
        best.map(|(_, id)| vec![id]).unwrap_or_default()
    }
}

/// Raging River's label, answered by the attacking seat: the pile that
/// stops `attacker` the least. A pile scores what its untapped creatures
/// could do to the attacker — a creature that kills it most, then one that
/// survives it — and the lesser score is the side to name; ties go to the
/// pile with less power.
pub(crate) fn river_label(view: &PlayerView, piles: &[Vec<ObjectId>], attacker: ObjectId) -> usize {
    let a = view.object(attacker);
    let threat = |pile: &Vec<ObjectId>| -> (i64, i64) {
        let mut score = 0;
        let mut total = 0;
        for o in pile.iter().filter_map(|&id| view.object(id)) {
            total += power(o);
            if !untapped_creature(o) {
                continue;
            }
            score += 1;
            if let Some(a) = a {
                if kills(o, a) {
                    score += 4;
                }
                if !kills(a, o) {
                    score += 2;
                }
            }
        }
        (score, total)
    };
    (0..piles.len())
        .min_by_key(|&i| (threat(&piles[i]), i))
        .unwrap_or(0)
}

/// Raging River's left pile, chosen by a defending seat. The attacker
/// names the weaker side for each creature, so the best the defender can do
/// is two sides as even as possible: strongest first, each to the side
/// with less in it so far.
pub(crate) fn left_pile(view: &PlayerView, options: &[ObjectId], max: u8) -> Vec<ObjectId> {
    let mut ranked: Vec<(i64, ObjectId)> = options
        .iter()
        .map(|&id| (view.object(id).map_or(0, blocker_strength), id))
        .collect();
    ranked.sort_by_key(|&(s, id)| (std::cmp::Reverse(s), id));
    let (mut left, mut right) = (0i64, 0i64);
    let mut chosen = Vec::new();
    for (strength, id) in ranked {
        if left <= right && chosen.len() < usize::from(max) {
            left += strength;
            chosen.push(id);
        } else {
            right += strength;
        }
    }
    chosen
}

/// One of Camouflage's piles, chosen by a defending seat. The piles go to
/// the attackers at random, so the blockers are dealt round-robin by
/// strength over the piles still to fill: each pile gets one of the best
/// that are left, and no pile is all of them.
pub(crate) fn camouflage_pile(
    view: &PlayerView,
    options: &[ObjectId],
    pile: u8,
    of: u8,
    max: u8,
) -> Vec<ObjectId> {
    let left = usize::from(of.saturating_sub(pile).saturating_add(1).max(1));
    let mut ranked: Vec<(i64, ObjectId)> = options
        .iter()
        .map(|&id| (view.object(id).map_or(0, blocker_strength), id))
        .collect();
    ranked.sort_by_key(|&(s, id)| (std::cmp::Reverse(s), id));
    ranked
        .into_iter()
        .step_by(left)
        .take(usize::from(max))
        .map(|(_, id)| id)
        .collect()
}
