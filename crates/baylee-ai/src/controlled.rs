//! Deciding for a player whose turn this seat controls (CR 723).
//!
//! Mindslaver hands this seat every decision another player makes for one
//! turn (CR 723.1, 723.5): which of their spells they cast and at what,
//! which of their creatures attack and how their damage is divided, what
//! their spells and abilities choose as they resolve, what they discard.
//! This seat goes on making its own decisions as well (CR 723.8) — its
//! blocks, its own priority — and those reach the ordinary agent untouched:
//! the module answers only a question asked of somebody else that this
//! seat is awaited for ([`subject`]).
//!
//! **Every answer here is made against that player, inside the rules.** Only
//! their resources pay (CR 723.5a), which the payment readers already honour
//! through `decision::resource_player`, and the engine still refuses what is
//! not legal. The measure is the crate's own — [`crate::worth`], card value,
//! [`crate::tactics::material`] — read from *their* side of the table
//! ([`seen_by`]): no second evaluator for "bad for them", only the one there
//! is, pointed the other way, so a spell is cast for them where the worst it
//! can be aimed at costs them more than it would have done for them.
//!
//! What that measure cannot read is left undone: a spell nobody can price
//! stays in their hand, a "may" nobody can read is declined, and a question
//! with no adversarial reading here falls through to the ordinary answer,
//! which already reads their permanents as the hostile ones — removal they
//! resolve goes at their own creatures, a gift at this seat's.
//!
//! The stopgap this replaced passed every priority and attacked with
//! nothing that did not have to. That was safe and gave the turn away: a
//! good controller spends the other player's cards and creatures for them.

mod attack;

use baylee_cards_dsl::{AbilityDef, Effect, Filter, KeywordSet, Modifier, TargetReq};
use baylee_core::ids::{CardIndex, ObjectId, PlayerId};
use baylee_core::types::TypeSet;
use baylee_engine::choice::{LegalActions, NumberPrompt, Pending, PlayerAction};
use baylee_engine::engine::DecisionContext;
use baylee_view::{CardIdentity, Phase, PlayerView, Step};

use crate::HeuristicAgent;
use crate::worth::{Aim, Origin, THRESHOLD};

/// The player whose question this seat is answering, when it is somebody
/// else's: a player whose turn it controls (Mindslaver, CR 723.1) or whom it
/// controls for a moment (Word of Command, CR 723.2).
///
/// The engine asks the controlled player and awaits their controller, and
/// the view says both: it awaits this seat, and the question names the
/// other player. A question of this seat's own is `None`, whoever else it
/// controls (CR 723.8).
pub(crate) fn subject(view: &PlayerView, pending: &Pending) -> Option<PlayerId> {
    if view.awaiting != Some(view.seat) {
        return None;
    }
    pending.asked().filter(|player| *player != view.seat)
}

/// The table as `subject` sees it, made from this view alone: their hand
/// where this seat's was, and their seat as the one the view belongs to.
///
/// This seat sees what they see while it controls them (CR 723.4), so
/// nothing is added that the view did not already hold, and this seat's own
/// hand is left out, because it is not theirs to see. Every reader in the
/// crate then measures for them: "you" on their cards, their mana, their
/// life, their creatures as the friendly ones.
pub(crate) fn seen_by(view: &PlayerView, subject: PlayerId) -> PlayerView {
    let mut seen = view.clone();
    seen.hand = baylee_client_core::decision::hand(view).to_vec();
    seen.controlled_hands.clear();
    seen.shared_hands.clear();
    seen.seat = subject;
    seen.awaiting = Some(subject);
    seen.decision_player = Some(subject);
    seen
}

/// The abilities printed on `face` of `card` when that face is an Aura,
/// whose spell "attaches itself" to what it targets (CR 303.4a).
fn aura(card: CardIndex, face: u8) -> Option<&'static [AbilityDef]> {
    let def = baylee_cards::by_index(card)?;
    def.faces
        .get(usize::from(face))?
        .subtypes
        .contains(&baylee_core::generated::subtypes::enchantment::AURA)
        .then(|| def.abilities_for_face(usize::from(face)))
}

/// Whether a static ability's filter is "enchanted creature": the one its
/// Aura is attached to.
fn enchants(filter: &Filter) -> bool {
    match filter {
        Filter::AttachedToBySource => true,
        Filter::And(parts) => parts.iter().any(enchants),
        _ => false,
    }
}

impl HeuristicAgent {
    /// The answer to a question this seat decides for `subject`, made
    /// against them; `None` hands the question to the ordinary answer.
    pub(crate) fn decide_against(
        &self,
        view: &PlayerView,
        pending: &Pending,
        _context: &DecisionContext<'_>,
        subject: PlayerId,
    ) -> Option<PlayerAction> {
        match pending {
            Pending::Priority { legal, .. } => {
                Some(self.priority_against(view, &seen_by(view, subject), legal))
            }
            Pending::ChooseAttackers {
                attackers,
                defenders,
                required,
                alone,
                minimum_required,
                limits,
                ..
            } => Some(PlayerAction::DeclareAttackers {
                attackers: crate::combat::obey_attack_rules(
                    self.attack_against(view, subject, attackers, defenders, required, limits),
                    required,
                    alone,
                    *minimum_required,
                    limits,
                    defenders,
                ),
            }),
            // Their creatures do not block in their own turn, and the rules
            // say what any block they owe must be (CR 509.1c).
            Pending::ChooseBlockers { obeying, .. } => Some(PlayerAction::DeclareBlockers {
                blockers: obeying.clone(),
            }),
            Pending::ChooseNumber {
                min,
                max,
                reason:
                    NumberPrompt::CombatDamage {
                        source, recipient, ..
                    },
                ..
            } => self
                .share_against(view, *source, *recipient, *min, *max)
                .map(PlayerAction::ChooseNumber),
            _ => None,
        }
    }

    /// Their priority: what hurts them, and otherwise nothing.
    ///
    /// Nothing before their combat is over. What combat kills is gone by
    /// then, so a burn spell is not spent on a creature that was about to
    /// die in this seat's block; and a sorcery still has their second main
    /// phase. Their land is never played and nothing of theirs is cast that
    /// builds their board: a spell goes only where [`Self::casting_harm`]
    /// says it costs them, and an activation only where
    /// [`crate::activate::against`] does. Last, in their end step, every
    /// source of theirs that only makes mana is tapped ([`tap_out`]).
    fn priority_against(
        &self,
        view: &PlayerView,
        theirs: &PlayerView,
        legal: &LegalActions,
    ) -> PlayerAction {
        // A payment this seat took on for them is finished like any other
        // (Word of Command's card): the payment readers spend only their
        // mana (CR 723.5a).
        if let Some(action) = crate::policy::pay_owed(view, legal) {
            return action;
        }
        if view.owed.is_some() {
            return PlayerAction::PassPriority;
        }
        if theirs.active != theirs.seat
            || !matches!(theirs.phase, Phase::SecondMain | Phase::Ending)
        {
            return PlayerAction::PassPriority;
        }
        if let Some((source, ability_index)) = crate::activate::against(theirs, legal, self) {
            return PlayerAction::ActivateAbility {
                source,
                ability_index,
            };
        }
        if let Some(action) = self.spend_on(theirs, legal, 0, |theirs, id, card, available| {
            self.casting_harm(theirs, id, card, available)
                .map(|harm| -harm - THRESHOLD)
        }) {
            return action;
        }
        if theirs.step == Step::End
            && theirs.stack.is_empty()
            && let Some(action) = tap_out(theirs, legal)
        {
            return action;
        }
        PlayerAction::PassPriority
    }

    /// What casting `card` now does to the player `theirs` belongs to, aimed
    /// as badly for them as its targets allow, less what it would have done
    /// for them aimed as they would aim it: below nothing when casting it
    /// for them costs them, and `None` for a card this reading leaves in
    /// their hand.
    ///
    /// Measured against their own best use and not against nothing, because
    /// the card is theirs to keep: a Lightning Bolt at their own Grizzly
    /// Bears is the Bears *and* the Bolt that would have killed this seat's
    /// creature, and a Healing Salve on themselves costs them nothing they
    /// would not have done anyway, so it stays in hand. And never a spell
    /// that does anything for them at the worst it can be aimed at: what is
    /// cast for them never hands them a gain.
    ///
    /// Left in hand: every permanent spell but an Aura, because a creature,
    /// an artifact or a planeswalker cast for them is their board built; a
    /// spell with two target words, whose two aims are not one choice here;
    /// and any text [`Self::effects_worth`] cannot read. `available` is the
    /// mana that pays for it, out of which an X is announced.
    pub(crate) fn casting_harm(
        &self,
        theirs: &PlayerView,
        id: ObjectId,
        card: CardIdentity,
        available: u64,
    ) -> Option<i64> {
        let face = crate::policy::face(card)?;
        let printed = (card.index, card.face);
        if !face
            .types
            .intersects(TypeSet::INSTANT.union(TypeSet::SORCERY))
            && aura(card.index, card.face).is_none()
        {
            return None;
        }
        let x = if face.mana_cost.has_variable() {
            u32::try_from(available.saturating_sub(u64::from(face.mana_cost.cmc())))
                .unwrap_or(u32::MAX)
        } else {
            0
        };
        let origin = Origin::of(theirs, id);
        let mut read = false;
        let (mut worst, mut best) = (0_i64, 0_i64);
        for ability in
            baylee_cards::by_index(card.index)?.abilities_for_face(usize::from(card.face))
        {
            let (low, high) = match ability {
                AbilityDef::Spell {
                    effects,
                    targets,
                    second_targets: None,
                    ..
                } => self.ends(theirs, origin, Some(printed), effects, targets.as_ref(), x)?,
                AbilityDef::Spell { .. } => return None,
                // This seat chooses the mode as well (CR 700.2, 723.5), and
                // the player would have chosen their own.
                AbilityDef::ModalSpell { modes, .. } => modes
                    .iter()
                    .filter(|mode| mode.second_targets.is_none())
                    .filter_map(|mode| {
                        self.ends(theirs, origin, None, mode.effects, mode.targets.as_ref(), x)
                    })
                    .reduce(|(lo, hi), (low, high)| (lo.min(low), hi.max(high)))?,
                _ => continue,
            };
            read = true;
            worst = worst.saturating_add(low);
            best = best.saturating_add(high);
        }
        (read && worst <= 0).then(|| worst.saturating_sub(best.max(0)))
    }

    /// The worst and the best `effects` can be aimed at for the player
    /// `theirs` belongs to, by [`Self::aimed_against`]: what this seat would
    /// make of it, and what they would.
    fn ends(
        &self,
        theirs: &PlayerView,
        origin: Origin<'_>,
        printed: Option<(CardIndex, u8)>,
        effects: &[Effect],
        targets: Option<&TargetReq>,
        x: u32,
    ) -> Option<(i64, i64)> {
        let aims = match targets {
            None => vec![Aim::Source],
            Some(req) => {
                let mut aims = self.aims(theirs, origin, &req.spec)?;
                if req.min == 0 {
                    aims.push(Aim::Nothing);
                }
                aims
            }
        };
        aims.into_iter()
            .filter_map(|aim| self.aimed_against(theirs, origin, printed, effects, aim, x))
            .fold(None, |ends, worth| {
                Some(ends.map_or((worth, worth), |(lo, hi): (i64, i64)| {
                    (lo.min(worth), hi.max(worth))
                }))
            })
    }

    /// What `effects` from `origin` do to the player `theirs` belongs to
    /// when aimed at `aim`: the effects as [`Self::effects_worth`] reads
    /// them, and an Aura's attachment by what it does to the creature it
    /// enchants ([`Self::enchanting`]).
    ///
    /// One correction: outside combat, damage that does not destroy a
    /// creature does nothing, since it is gone at cleanup (CR 514.2), where
    /// the measure prices a ping as softening the creature up for a fight.
    /// Their spells are cast after their combat, and a Lightning Bolt left
    /// on their 4/4 was a Bolt not put into their face.
    pub(crate) fn aimed_against(
        &self,
        theirs: &PlayerView,
        origin: Origin<'_>,
        printed: Option<(CardIndex, u8)>,
        effects: &[Effect],
        aim: Aim,
        x: u32,
    ) -> Option<i64> {
        if let [Effect::AttachSelf { .. }] = effects
            && let Some(abilities) = printed.and_then(|(card, face)| aura(card, face))
        {
            return self.enchanting(theirs, abilities, aim);
        }
        if let ([Effect::DealDamage { amount, .. }], Aim::Object(id)) = (effects, aim)
            && theirs.phase != Phase::Combat
            && let Some(o) = theirs.object(id)
            && o.types.contains(TypeSet::CREATURE)
            && (i64::from(crate::tactics::amount(*amount, x)) < crate::tactics::damage_to_finish(o)
                || o.keywords & KeywordSet::INDESTRUCTIBLE.bits() != 0)
        {
            return Some(0);
        }
        self.effects_worth(theirs, origin, effects, aim, x)
    }

    /// What an Aura printing `abilities` does, enchanting `aim`, to the
    /// player `theirs` belongs to: its statics on the enchanted creature
    /// (CR 303.4) as a gain for that creature's controller, which is a loss
    /// to them when the creature is an opponent's. `None` for a static this
    /// reading has no row for, so Claustrophobia is not cast at all rather
    /// than cast on a guess.
    fn enchanting(&self, theirs: &PlayerView, abilities: &[AbilityDef], aim: Aim) -> Option<i64> {
        let Aim::Object(id) = aim else {
            return Some(0);
        };
        let creature = theirs
            .object(id)
            .filter(|o| o.types.contains(TypeSet::CREATURE))?;
        let left = i32::from(creature.toughness?) - i32::from(creature.damage);
        let body = crate::tactics::material(creature);
        let mut value = 0_i64;
        for ability in abilities {
            let AbilityDef::Static(s) = ability else {
                continue;
            };
            if !enchants(&s.filter) {
                continue;
            }
            value += match s.modifier {
                // A toughness it cannot keep is the creature (CR 704.5f).
                Modifier::ModifyPT(_, toughness) if left + i32::from(toughness) <= 0 => -body,
                Modifier::ModifyPT(power, toughness) => {
                    i64::from(power) * 110 + i64::from(toughness) * 30
                }
                Modifier::AddKeyword(keywords) if keywords.contains(KeywordSet::DEFENDER) => {
                    -body / 2
                }
                Modifier::AddKeyword(keywords) => 60 * i64::from(keywords.bits().count_ones()),
                Modifier::CantAttack | Modifier::CantBlock(_) => -body / 2,
                _ => return None,
            };
        }
        Some(if self.hostile(creature.controller, theirs.seat) {
            -value
        } else {
            value
        })
    }
}

/// In their end step, with nothing left to cast: the next of their sources
/// that only makes mana, tapped.
///
/// Their permanents untap only in their own untap step (CR 502.3), so a land
/// tapped now cannot pay for an answer in the turn that follows, and a mana
/// creature tapped now cannot block in it; the mana itself empties unspent
/// as the step ends (CR 500.5). A tap that sets off a trigger is left alone,
/// because what the trigger does is not read here.
fn tap_out(theirs: &PlayerView, legal: &LegalActions) -> Option<PlayerAction> {
    let quiet = |id: ObjectId| {
        theirs.object(id).is_some_and(|o| {
            o.controller == theirs.seat && !crate::policy::taps_set_off_a_trigger(theirs, o)
        })
    };
    if let Some(&source) = legal.mana_abilities.iter().find(|id| quiet(**id)) {
        return Some(PlayerAction::ActivateManaAbility { source });
    }
    legal
        .abilities
        .iter()
        .find(|&&(source, index)| quiet(source) && crate::only_makes_mana(theirs, source, index))
        .map(|&(source, ability_index)| PlayerAction::ActivateAbility {
            source,
            ability_index,
        })
}
