//! Which activated ability to use, and which cannot be used at all.
//!
//! The agent used to use none. The comment standing where this decision
//! belongs said that blind activation "loops on free no-op abilities", which
//! is true of one shape of ability and was being taken as a reason to skip
//! every shape — including the eight fetchlands and seven planeswalkers in
//! the acceptance decks. A fetchland nobody cracks is a land that makes no
//! mana at all, and a planeswalker nobody ticks is a card that does nothing
//! for the rest of the game.
//!
//! So the loop is ruled out by the **cost** instead of by abstention. Paying
//! a cost that taps, sacrifices, discards, exiles, bounces, spends life or
//! spends mana changes the state [`LegalActions`] is computed from, so the
//! same handle is either gone or unaffordable the next time the seat has
//! priority. Only a cost that is free *and* has no parts leaves the offer
//! exactly as it was, and that is the one shape refused outright.
//!
//! What is *worth* activating is [`crate::worth`]'s question: the ability's
//! effects at their best target, less what its cost gives up, on this
//! board. It replaced a whitelist of effects that are gains wherever they
//! land — a draw, a token, a search — which could not take anything whose
//! worth depends on its target or its cost: no Wasteland, Maze of Ith,
//! Recurring Nightmare or equip was ever activated. What the measure cannot
//! read it leaves alone, as the whitelist did, so a new mechanic is inert
//! rather than misplayed.

use baylee_cards_dsl::{AbilityDef, Cost, CostPart, Effect};
use baylee_core::ids::ObjectId;
use baylee_core::mana::ManaCost;
use baylee_core::types::TypeSet;
use baylee_engine::choice::LegalActions;
use baylee_view::{PlayerView, PublicObject};

use crate::worth::{Origin, THRESHOLD};

/// The ability to activate now, as one of the offered `(source, index)`
/// handles — or `None`, which is most of the time.
///
/// A fetchland first, because it is the one activation that changes what
/// the seat can *pay* with; the caller asks [`fetch`] on its own earlier for
/// that reason, and it stays here so that this function is the whole
/// policy. Then the shallow profiles' loyalty rule ([`loyalty`]), which is a
/// designed difference and not a gap. Then the offered activation worth the
/// most on this board, when it is worth [`THRESHOLD`] net of its cost — a
/// deep profile's loyalty ability among them, at no threshold, since
/// loyalty is a price this measure already charges.
#[must_use]
pub(crate) fn choose(
    view: &PlayerView,
    legal: &LegalActions,
    agent: &crate::HeuristicAgent,
) -> Option<(ObjectId, u32)> {
    fetch(view, legal)
        .or_else(|| {
            if agent.profile.lookahead > 0 {
                None
            } else {
                loyalty(view, legal)
            }
        })
        .or_else(|| best(view, legal, agent))
}

/// The offered activation worth the most, when it is worth taking.
fn best(
    view: &PlayerView,
    legal: &LegalActions,
    agent: &crate::HeuristicAgent,
) -> Option<(ObjectId, u32)> {
    legal
        .abilities
        .iter()
        .copied()
        .filter_map(|(source, index)| {
            let def = printed(view, source, index)?;
            let floor = match def {
                AbilityDef::Loyalty { .. } if agent.profile.lookahead == 0 => return None,
                AbilityDef::Loyalty { .. } => 0,
                _ => {
                    let (cost, _) = activated(def)?;
                    if !consumes(cost) {
                        return None;
                    }
                    THRESHOLD
                }
            };
            let worth = agent.activation_worth(view, Origin::of(view, source), def)?;
            (worth > floor).then_some((worth, (source, index)))
        })
        .max_by_key(|(worth, handle)| (*worth, std::cmp::Reverse(*handle)))
        .map(|(_, handle)| handle)
}

/// The [`AbilityDef`] an offered handle names, when a card prints one.
///
/// `None` for the synthetic indices — a prepared cast, a granted ability —
/// because those are positions in the *offer* and not on the card, so the
/// lookup misses and the agent leaves them be. `None` too for a card in
/// hand, since [`PlayerView::object`] does not reach the hand: cycling is
/// therefore not offered here, which is the right answer for now anyway
/// (`Engine::can_afford` probes the floating pool, so a `{3}` cycling cost
/// is never in `legal.abilities` in the first place).
pub(crate) fn printed(
    view: &PlayerView,
    object: ObjectId,
    index: u32,
) -> Option<&'static AbilityDef> {
    printed_list(view.object(object)?).get(usize::try_from(index).ok()?)
}

/// The list an object's abilities are printed on, which is what an offered
/// index counts into.
///
/// Read from `rules` and not from `card`, because the two part for a copy:
/// a Glasspool Mimic that entered as a Werefox Bodyguard is still a Mimic
/// underneath and has the Fox's abilities (CR 707.2), and the engine offers
/// them as indices into the Fox's list (#214). A token copy has no card at
/// all and a `rules` all the same, so its abilities are read here too. A
/// registry token has no `rules`: its text is its definition's (CR 111.3),
/// which `token` names — a Treasure read as printing nothing until #223.
/// Face down it has no text at all (CR 708.2).
pub(crate) fn printed_list(object: &PublicObject) -> &'static [AbilityDef] {
    if let Some(rules) = object.rules {
        return baylee_cards::by_index(rules.card)
            .map_or(&[], |def| def.abilities_for_face(usize::from(rules.face)));
    }
    object
        .token
        .filter(|_| !object.status.is_face_down())
        .and_then(baylee_cards::tokens::by_token_id)
        .map_or(&[], |token| token.abilities)
}

/// The activation cost and effects of an ability, for the two shapes that
/// have both. Loyalty is deliberately not one of them — its cost is a
/// loyalty delta the engine has already checked.
fn activated(def: &'static AbilityDef) -> Option<(&'static Cost, &'static [Effect])> {
    match def {
        AbilityDef::Activated {
            cost,
            effects,
            mana_ability,
            ..
        }
        | AbilityDef::ActivatedConditional {
            cost,
            effects,
            mana_ability,
            ..
        } => (!*mana_ability).then_some((cost, *effects)),
        _ => None,
    }
}

/// Whether paying this cost changes something the next [`LegalActions`] is
/// built from.
///
/// This is the whole anti-loop argument, and it is about the cost rather
/// than the effect because an effect can be legitimately invisible — a scry
/// changes nothing a view shows — while a cost never is.
///
/// [`CostPart::PayLifeX`] is left out on purpose: X may be zero, and a cost
/// that may be nothing is not a cost that limits anything.
///
/// The `match` is **exhaustive with no wildcard**, and it is written that way
/// because the list it replaces was a `matches!` that had gone one part
/// short. `TapOther` was added for the convoke lands and nothing here
/// classified it, so Earthcraft — whose whole cost is "Tap an untapped
/// creature you control", with no mana and no `{T}` of its own — came out as
/// a cost that consumes nothing, and the house AI declined to untap a basic
/// land with it for as long as the card existed. A positive list answers a
/// new variant with silence; a `match` answers it with a compile error.
fn consumes(cost: &Cost) -> bool {
    cost.mana != ManaCost::ZERO
        || cost.parts.iter().any(|part| match part {
            CostPart::TapSelf
            | CostPart::UntapSelf
            | CostPart::SacrificeSelf
            | CostPart::Sacrifice(_)
            | CostPart::Discard(_)
            | CostPart::DiscardSelf
            | CostPart::ExileSelf
            | CostPart::ExileFromHand(_)
            | CostPart::ReturnSelfToHand
            | CostPart::PayLife(_)
            | CostPart::RemoveCounterSelf { .. }
            // A number the seat announces, and the agent announces the
            // smallest one it is offered — so this alone would take nothing
            // off and repeat for ever. It is `true` because every cost in
            // the pool that carries it also carries a `{T}` or a mana part,
            // and because the conservative answer to "can this repeat?" is
            // the one that does not hang the game.
            | CostPart::RemoveCounterSelfX { .. }
            // A counter put **on** the source changes the source, so the
            // next offer is built from a different board. The bound is the
            // counter rather than the cost: Devoted Druid's -1/-1s reach its
            // toughness and a state-based action takes it away, and Wall of
            // Roots carries an activation limit beside it.
            | CostPart::PutCounterSelf { .. }
            // A creature that paid is a creature that cannot pay again, and
            // it is the *board* that shrinks rather than the source — which
            // is the same limit, read one permanent over.
            | CostPart::TapOther(_)
            | CostPart::Crew(_)
            // A permanent returned to a hand leaves the battlefield, so the
            // board shrinks the same way — more so than a tap, which leaves
            // the permanent where it was.
            | CostPart::ReturnToHand(_)
            // A card exiled out of the graveyard is gone for good, so the
            // pile that pays for the next activation is one card shorter.
            | CostPart::ExileFromGraveyard(_) => true,
            CostPart::PayLifeX => false,
        })
}

/// Whether the life this cost asks for is life the seat can spare.
///
/// The `+ 5` is the same margin the pay-life-or-enter-tapped answer uses;
/// a second threshold for the same question would be two numbers to keep
/// in step.
pub(crate) fn life_ok(view: &PlayerView, cost: &Cost) -> bool {
    let need = cost.parts.iter().fold(0u16, |sum, part| match part {
        CostPart::PayLife(n) => sum.saturating_add(*n),
        _ => sum,
    });
    need == 0
        || view
            .seat(view.seat)
            .is_some_and(|s| s.life > i32::from(need) + 5)
}

/// The fetchland to crack, if one is offered.
///
/// It gets its own pass rather than a place in `best` because it is
/// not merely good: a land that sacrifices itself to search is a land that
/// makes no mana until it does, and cracking it is what turns a dead card
/// into a source. Nothing else the agent can activate changes what it can
/// pay with.
pub(crate) fn fetch(view: &PlayerView, legal: &LegalActions) -> Option<(ObjectId, u32)> {
    legal.abilities.iter().copied().find(|&(source, index)| {
        let Some(object) = view.object(source) else {
            return false;
        };
        if !object.types.contains(TypeSet::LAND) {
            return false;
        }
        let Some(def) = printed(view, source, index) else {
            return false;
        };
        let Some((cost, effects)) = activated(def) else {
            return false;
        };
        cost.parts.contains(&CostPart::SacrificeSelf)
            && effects
                .iter()
                .any(|e| matches!(e, Effect::SearchLibrary { .. }))
            && life_ok(view, cost)
    })
}

/// The loyalty ability to use, out of what is offered.
///
/// The ultimate first: the engine offers a negative ability only when the
/// walker can pay for it, so an offered ultimate is one that resolves.
/// Otherwise the largest plus. Never the small minus that happens to be
/// affordable — taking a −1 every turn because it is the only thing on
/// offer walks the planeswalker into the graveyard for effects the agent
/// cannot yet weigh.
///
/// The ultimate is read off the **card**, not off the offer, which is the
/// difference between "the ability with the biggest cost this walker
/// prints" and "the most negative thing available right now". The second
/// reading is how a walker at three loyalty spends itself down to nothing.
fn loyalty(view: &PlayerView, legal: &LegalActions) -> Option<(ObjectId, u32)> {
    let mut best: Option<((ObjectId, u32), i8)> = None;
    for &(source, index) in &legal.abilities {
        let Some(AbilityDef::Loyalty { cost, .. }) = printed(view, source, index) else {
            continue;
        };
        // Is this the walker's own ultimate — the most expensive thing it
        // prints, whether or not anything else is on offer today?
        let ultimate = view.object(source).is_some_and(|o| {
            printed_list(o)
                .iter()
                .filter_map(|a| match a {
                    AbilityDef::Loyalty { cost, .. } => Some(*cost),
                    _ => None,
                })
                .min()
                == Some(*cost)
        });
        if ultimate {
            return Some((source, index));
        }
        if *cost >= 0 && best.is_none_or(|(_, seen)| *cost > seen) {
            best = Some(((source, index), *cost));
        }
    }
    best.map(|(handle, _)| handle)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The anti-loop rule, both ways round: a free cost with no parts is the
    /// one shape that leaves the offer exactly as it found it.
    #[test]
    fn only_a_free_cost_can_repeat() {
        assert!(!consumes(&Cost::FREE), "a free ability was taken");
        assert!(consumes(&Cost::TAP), "a tap ability was refused");
        assert!(
            consumes(&Cost {
                mana: ManaCost::ZERO,
                parts: &[CostPart::SacrificeSelf],
            }),
            "sacrificing the source was not counted as consuming it"
        );
        assert!(
            consumes(&Cost {
                mana: ManaCost::ZERO,
                parts: &[CostPart::TapOther(&baylee_cards_dsl::Filter::YOUR_CREATURE)],
            }),
            "Earthcraft: the whole cost is somebody else's tap, and a creature \
             that paid cannot pay again"
        );
    }

    /// X may be zero, so paying it is not a limit on anything.
    #[test]
    fn paying_x_life_is_not_a_limit() {
        assert!(!consumes(&Cost {
            mana: ManaCost::ZERO,
            parts: &[CostPart::PayLifeX],
        }));
    }
}
