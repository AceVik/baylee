//! Turning a spell and copying one (#226 B): Misdirection, Deflecting Swat,
//! Hydroelectric Specimen, Dualcaster Mage and every other
//! `Effect::ChangeTarget`, `Effect::ChooseNewTargets` or
//! `Effect::CopyTargetSpell`.
//!
//! Each asks two target questions, and neither is answered by its own
//! effect, which means nothing to [`crate::tactics::meaning`]: read that way
//! the fallback's opponent-first rule decided both. Misdirection was aimed
//! at this seat's own Path to Exile and turned it from Serra Angel onto a
//! 1/1, and Dualcaster Mage's copy of a Path went where the Path already was.
//!
//! - **Which spell** — as the card is cast or its trigger is put on the
//!   stack. A redirect is worth what the spell it turns is aimed at of this
//!   seat's ([`HeuristicAgent::redirect_worth`]), and the counter's gate
//!   asks the same number before the card is cast. A copy is worth what the
//!   copied effect is worth on this board, whoever controls the original:
//!   the copy is this seat's.
//! - **What it becomes** — as the redirect or copy resolves, asked of the
//!   spell's targets. It is answered by what *that* spell means: a turned
//!   Path is removal and goes to the best creature across the table. A copy
//!   starts with the original's targets (CR 707.10) and may change them
//!   (CR 707.10c). What a harmful spell on the stack is aimed at scores
//!   nothing, whoever cast it, and the copy is one of them: a second Path at
//!   the creature one is already exiling misses.
//!
//! The two are told apart by the stack: while the redirect or copy
//! resolves, it is on the stack; while it is cast or put there, it is not
//! yet. The options cannot tell them apart — a Misdirection turning a
//! Counterspell is offered spells both times.
//!
//! Not yet: a spell's X is not in the view and reads as 0; a hostile gift
//! (their pump on their creature) is not turned onto this seat's; the
//! Specimen does not weigh that the new target is itself; nothing holds a
//! redirect or a Dualcaster Mage back for a better spell; and an opponent's
//! redirect aimed at this seat's own spell is not turned back, because a
//! redirect hurts nothing by itself.

use baylee_cards_dsl::Effect;
use baylee_core::ids::ObjectId;
use baylee_core::types::TypeSet;
use baylee_engine::choice::PlayerAction;
use baylee_engine::engine::DecisionContext;
use baylee_view::{PlayerView, PublicObject, StackItem, TargetRef};

use crate::HeuristicAgent;
use crate::tactics::{Meaning, Offer, material, stack_meaning};

/// Which of the two kinds of effect is asking.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// "Change the target" or "choose new targets".
    Redirect,
    /// "Copy target spell".
    Copy,
}

/// The kind of the first redirect or copy in `effects`, looked for behind a
/// "you may" too: Hydroelectric Specimen's is.
fn kind(effects: &[Effect]) -> Option<Kind> {
    effects.iter().find_map(|effect| match effect {
        Effect::ChangeTarget { .. } | Effect::ChooseNewTargets => Some(Kind::Redirect),
        Effect::CopyTargetSpell { .. } => Some(Kind::Copy),
        _ => {
            let (then, otherwise) = effect.branches();
            kind(then).or_else(|| kind(otherwise))
        }
    })
}

/// The spell a resolving redirect or copy is about: its first target, while
/// it is on the stack. `None` while it is being cast or put there.
fn resolving_about<'v>(
    view: &'v PlayerView,
    context: &DecisionContext<'_>,
) -> Option<&'v PublicObject> {
    let source = context.source?;
    let resolving = view.stack.iter().find(|o| {
        o.id == source
            || matches!(o.stack_item, Some(StackItem::Ability { source: s, .. }) if s == source)
    })?;
    let &TargetRef::Object(spell) = resolving.targets.first()? else {
        return None;
    };
    view.stack.iter().find(|o| o.id == spell)
}

/// Whether a spell hurts what it is aimed at.
fn harmful(m: Meaning) -> bool {
    m.benefit < 0 || m.damage > 0 || m.removal || m.counter
}

impl HeuristicAgent {
    /// What turning `o` is worth to this seat: what of this seat's a
    /// hostile spell that hurts is aimed at. Nothing for a hostile spell
    /// aimed elsewhere or a gift, and far below nothing for this seat's own
    /// spell, which the redirect would only take off its target.
    pub(crate) fn redirect_worth(&self, view: &PlayerView, o: &PublicObject) -> i64 {
        if !self.hostile(o.controller, view.seat) {
            return -10_000 - material(o);
        }
        if harmful(stack_meaning(o, 0)) {
            self.aimed_at_this_seat(view, o)
        } else {
            0
        }
    }

    /// What a copy of `o` is worth to this seat, which controls the copy
    /// whoever controls `o`: an instant's or sorcery's effect on this board,
    /// or a permanent spell's body.
    fn copy_worth(&self, view: &PlayerView, o: &PublicObject) -> i64 {
        if o.types.intersects(TypeSet::INSTANT.union(TypeSet::SORCERY)) {
            self.meaning_value(view, stack_meaning(o, 0))
        } else {
            material(o)
        }
    }

    /// Both questions of a redirect or copy, or `None` when the question is
    /// neither, or when the spell turned or copied means nothing this side
    /// can read and the general fallback answers.
    pub(crate) fn stack_targets(
        &self,
        view: &PlayerView,
        offer: Offer<'_>,
        context: &DecisionContext<'_>,
    ) -> Option<PlayerAction> {
        let kind = kind(context.effects)?;
        let Some(spell) = resolving_about(view, context) else {
            // Which spell: the best of the offered spells, one of them.
            let best = offer.objects.iter().copied().max_by_key(|&id| {
                let worth = view.object(id).map_or(i64::MIN, |o| match kind {
                    Kind::Redirect => self.redirect_worth(view, o),
                    Kind::Copy => self.copy_worth(view, o),
                });
                (worth, std::cmp::Reverse(id))
            })?;
            return Some(PlayerAction::ChooseTargets {
                objects: vec![best],
                players: vec![],
            });
        };
        // What it becomes, by what the spell means. What a harmful spell on
        // the stack is aimed at is already hit, whoever cast it, and a copy
        // is one of them: it starts where the original is (CR 707.10).
        let spent: Vec<ObjectId> = view
            .stack
            .iter()
            .filter(|o| harmful(stack_meaning(o, 0)))
            .flat_map(|o| &o.targets)
            .filter_map(|t| match t {
                TargetRef::Object(id) => Some(*id),
                TargetRef::Player(_) => None,
            })
            .collect();
        let m = stack_meaning(spell, 0);
        self.rank(view, offer, context, m, &spent)
    }
}
