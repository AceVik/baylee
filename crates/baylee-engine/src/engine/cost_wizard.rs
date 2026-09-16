//! Paying an activation cost that has to ask the player a question.
//!
//! "{T}, Sacrifice a creature: Add {B}{B}" names no creature, and until this
//! module existed the engine had nowhere to ask which one. `can_afford`
//! refused `CostPart::Sacrifice` and `CostPart::Discard` outright — not for
//! anything it could see on the board, but because `pay_cost` would answer
//! them with "choice costs are not supported yet (M2)" *after* emptying the
//! mana pool, and without rewinding. Five cards in the pool wrote the cost
//! and carried `Coverage::Partial` for exactly that reason, and fifty more
//! print such a line in their `//! Oracle:` header.
//!
//! The question is asked on the seam that already exists. `start_activation`
//! suspends for targets by writing a [`super::PlanKind`] and a `Pending`, and
//! `apply` re-enters it once the answer is in; the cost stage is the same
//! move one step later, in the order CR 601.2 gives and CR 602.2b applies to
//! an activation — targets at 601.2c, costs at 601.2h.
//!
//! **Choosing what to sacrifice is not targeting** (CR 115.1: only an effect
//! that uses the word "target" targets). A creature with hexproof may be
//! sacrificed to its controller's own Viscera Seer, and sacrificing does not
//! make anything "become the target of" an ability. The answer therefore
//! arrives as [`crate::choice::Pending::ChooseCards`] with its own
//! [`ChoicePrompt`], which is where a cost belongs: `ChoicePrompt::Delve`
//! was already there for the same reason, and the convoke question next door
//! is labelled `TargetPrompt::Convoke` on the same argument.
//!
//! One function answers what may be chosen, and both readers use it. A board
//! question in `can_afford` that disagreed with the list put in front of the
//! player is the offer/apply contradiction this engine treats as its worst
//! kind: an ability offered and then refused, or worse, a list holding
//! something the payment will not accept.
//!
//! One thing the pair is honestly not exact about, and no card in the pool
//! reaches it: `can_afford` reads each asking part on its own, so a cost
//! printing **two** of them over one narrow board — two sacrifices where the
//! seat controls a single creature — is offered, asks its first question,
//! and is then refused at the second for want of anything left to name.
//! `start_activation` takes the answers so far off each later menu, so the
//! one permanent is never eaten twice, and nothing has been paid at the
//! moment of the refusal. Making the offer exact would mean counting the
//! parts against a board where two filters may overlap, which is a real
//! question and not one a card is asking yet.

use super::{
    Cause, Cost, CostPart, EngineError, GameEvent, ObjectId, PlayerId, Status, ZoneLocation,
    ZonePosition,
};
use crate::choice::ChoicePrompt;
use crate::eval;
use crate::state::GameState;

/// Whether this part cannot be paid until somebody names an object.
///
/// The successor to `choice_cost_unpayable`, and the opposite of it: that
/// predicate said "refuse this", this one says "ask about this". Every
/// reader that used to relax when the old one answered `false` now asks
/// [`options`] instead, which is a question about the board rather than
/// about the engine's own limits.
pub(crate) const fn needs_an_answer(part: &CostPart) -> bool {
    matches!(
        part,
        CostPart::Sacrifice(_)
            | CostPart::Discard(_)
            | CostPart::TapOther(_)
            | CostPart::ReturnToHand(_)
    )
}

/// How many answers a cost needs before it can be paid.
pub(crate) fn answers_wanted(cost: &Cost) -> usize {
    cost.parts.iter().filter(|p| needs_an_answer(p)).count()
}

/// The parts that need an answer, in the order the cost prints them.
///
/// The order is the contract between the question and the payment: the nth
/// answer pays the nth asking part, so a cost with a sacrifice and a discard
/// cannot pay one with the other's card.
pub(crate) fn asking_parts(cost: &Cost) -> impl Iterator<Item = &CostPart> {
    cost.parts.iter().filter(|p| needs_an_answer(p))
}

/// What `part` may be paid with, on this board.
///
/// The single reader of that question. `can_afford` asks whether this is
/// empty and the prompt hands the same list to the player, so the two cannot
/// drift — which is the whole reason it is a function and not two loops.
///
/// Ownership is enforced here rather than left to the filter, because it is
/// the rule and not the card: CR 701.21a only lets a player sacrifice a
/// permanent **they control**, and CR 701.9a discards from their own hand.
/// Survival of the Fittest prints `Discard(&Filter::CREATURE)` with no
/// "you control" in it at all, and reading that filter alone over every hand
/// at the table would have offered an opponent's card.
///
/// [`CostPart::TapOther`] is the one where no rule says whose — the card
/// prints "a creature **you control**" and all thirteen in this pool do — so
/// the same line is drawn here anyway, deliberately narrower than the rules
/// require. A cost paid by tapping something across the table is not a thing
/// Magic prints, and being wrong in this direction offers a player less than
/// the card allows rather than handing them an opponent's permanent.
///
/// [`CostPart::ReturnToHand`] is the second of that kind and the measurement
/// behind it is stronger: 52 `Cost$` lines in the card-script reference print
/// a return cost naming something other than the source, and **all 52** print
/// "you control". The filter says it too — the transcoder writes
/// `Filter::ControlledByYou` into every one it emits — and this line is the
/// second half of the same answer rather than a substitute for it, which is
/// Earthcraft's arrangement one variant up.
///
/// What it does *not* borrow from `TapOther` is "untapped". A Forest tapped
/// for `{G}` is the cost Quirion Ranger was printed to pay, so the word is
/// absent here and lives in the filter on the six costs that print it.
///
/// What the rule does supply there is "untapped": CR 118.3 says a permanent
/// that is already tapped cannot be tapped to pay a cost, whether or not the
/// card thought to say so. Summoning sickness is deliberately *not* read —
/// CR 302.6 restricts a creature's own `{T}` ability and says nothing about
/// a creature being tapped to pay for somebody else's, which is why a
/// freshly cast Bird can pay Earthcraft the turn it arrives.
pub(crate) fn options(
    state: &GameState,
    player: PlayerId,
    source: ObjectId,
    part: &CostPart,
) -> Vec<ObjectId> {
    let (zone, controlled, untapped) = match part {
        CostPart::Sacrifice(_) | CostPart::ReturnToHand(_) => {
            (ZoneLocation::Battlefield, true, false)
        }
        CostPart::TapOther(_) => (ZoneLocation::Battlefield, true, true),
        CostPart::Discard(_) => (ZoneLocation::Hand(player), false, false),
        _ => return Vec::new(),
    };
    let (CostPart::Sacrifice(filter)
    | CostPart::Discard(filter)
    | CostPart::TapOther(filter)
    | CostPart::ReturnToHand(filter)) = part
    else {
        return Vec::new();
    };
    state
        .zones
        .list(zone)
        .iter()
        .filter(|id| {
            state.object(**id).is_some_and(|o| {
                (!controlled || o.controller == player)
                    && (!untapped || !o.status.contains(Status::TAPPED))
                    && eval::matches(filter, state, o, player, source)
            })
        })
        .copied()
        .collect()
}

/// What the player is being asked for.
pub(crate) const fn prompt(part: &CostPart) -> ChoicePrompt {
    match part {
        CostPart::Discard(_) => ChoicePrompt::CostDiscard,
        CostPart::TapOther(_) => ChoicePrompt::CostTap,
        CostPart::ReturnToHand(_) => ChoicePrompt::CostReturn,
        _ => ChoicePrompt::CostSacrifice,
    }
}

/// Pays one asking part with the object the player named.
///
/// A sacrifice and a discard put a card in its owner's graveyard through
/// [`GameState::move_object`] under [`Cause::Cost`], which is what the
/// `SacrificeSelf` and `DiscardSelf` arms of `pay_cost` already do — the same
/// door, so a sacrifice chosen by a player and a sacrifice printed on the
/// card cannot come out as two different events. A tap goes through the
/// `TapSelf` arm's door for the same reason, down to the
/// [`GameEvent::ObjectTapped`] it records: an ability that triggers on a
/// creature becoming tapped may not see one of the two and miss the other.
///
/// A return goes through the same door to a different zone, which is the
/// `ReturnSelfToHand` arm of `pay_cost` one file over: a permanent bounced
/// to pay a cost and one that bounced itself are the same event.
///
/// The part is passed in rather than inferred from the object, because the
/// object cannot say it. A creature on the battlefield is a legal answer to
/// a sacrifice, a tap and a return alike, and the three are different
/// outcomes.
///
/// The legality of the answer is checked by `apply` against the very list
/// [`options`] produced, before this is ever reached. This re-reads the owner
/// and nothing else.
pub(crate) fn pay(
    state: &mut GameState,
    player: PlayerId,
    part: &CostPart,
    chosen: ObjectId,
) -> Result<(), EngineError> {
    if matches!(part, CostPart::TapOther(_)) {
        if let Some(obj) = state.object_mut(chosen) {
            obj.status.insert(Status::TAPPED);
        }
        state.journal.record(GameEvent::ObjectTapped {
            object: chosen,
            cause: Cause::Cost,
        });
        return Ok(());
    }
    let owner = state.object(chosen).map_or(player, |o| o.owner);
    // Owner's hand, never the payer's: CR 400.3 puts a returned card in the
    // zone of the player who owns it, and a Forest borrowed off somebody
    // else's battlefield goes home rather than joining the borrower's hand.
    let to = if matches!(part, CostPart::ReturnToHand(_)) {
        ZoneLocation::Hand(owner)
    } else {
        ZoneLocation::Graveyard(owner)
    };
    state.move_object(chosen, to, ZonePosition::Top, Cause::Cost)?;
    Ok(())
}
