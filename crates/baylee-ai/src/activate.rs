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
//! What is *worth* activating is the smaller question, and the one a real
//! evaluator will answer. Until then the whitelist here is short and
//! explicit: an effect this module does not recognise leaves the ability
//! alone, so a new mechanic is inert rather than misplayed.

use baylee_cards_dsl::{AbilityDef, Cost, CostPart, Effect};
use baylee_core::ids::ObjectId;
use baylee_core::mana::ManaCost;
use baylee_core::types::TypeSet;
use baylee_engine::choice::LegalActions;
use baylee_view::{Phase, PlayerView};

/// The ability to activate now, as one of the offered `(source, index)`
/// handles — or `None`, which is most of the time.
///
/// The order is the only judgement here that is not local: a fetchland is
/// cracked before anything else because it is the one activation that
/// changes what the seat can *pay* with, and the caller taps for mana after
/// this returns. The caller asks [`fetch`] on its own earlier for exactly
/// that reason, so by the time this runs there is normally none left — it
/// stays in the order anyway, because this function is the whole policy and
/// a test of it should not depend on a caller's step numbering.
#[must_use]
pub(crate) fn choose(view: &PlayerView, legal: &LegalActions) -> Option<(ObjectId, u32)> {
    fetch(view, legal)
        .or_else(|| loyalty(view, legal))
        .or_else(|| useful(view, legal))
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
fn printed(view: &PlayerView, object: ObjectId, index: u32) -> Option<&'static AbilityDef> {
    let card = view.object(object)?.card?;
    let def = baylee_cards::by_index(card.index)?;
    def.abilities_for_face(card.face as usize)
        .get(usize::try_from(index).ok()?)
}

/// The activation cost and effects of an ability, for the two shapes that
/// have both. Loyalty is deliberately not one of them — its cost is a
/// loyalty delta the engine has already checked, and it is chosen by
/// [`loyalty`] rather than by the whitelist.
fn activated(def: &'static AbilityDef) -> Option<(&'static Cost, &'static [Effect], bool)> {
    match def {
        AbilityDef::Activated {
            cost,
            effects,
            target,
            mana_ability,
            ..
        }
        | AbilityDef::ActivatedConditional {
            cost,
            effects,
            target,
            mana_ability,
            ..
        } => (!*mana_ability).then_some((cost, *effects, target.is_some())),
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
fn consumes(cost: &Cost) -> bool {
    cost.mana != ManaCost::ZERO
        || cost.parts.iter().any(|part| {
            matches!(
                part,
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
            )
        })
}

/// Whether this effect is one the agent recognises as a gain.
fn gains(effect: &Effect) -> bool {
    match effect {
        Effect::Sequence(inner) => inner.iter().any(gains),
        e => matches!(
            e,
            Effect::SearchLibrary { .. }
                | Effect::DrawCards { .. }
                | Effect::Scry { .. }
                | Effect::CreateToken { .. }
                | Effect::CreateTokenN { .. }
                | Effect::Amass { .. }
                | Effect::AddCounter { .. }
                | Effect::GainLife { .. }
        ),
    }
}

/// Whether this effect is one the agent knows will not hurt it.
///
/// Everything it counts as a gain, plus the bookkeeping that rides along
/// with one: Sensei's Divining Top draws a card *and* puts itself back on
/// the library, and refusing the second half would refuse the first.
fn harmless(effect: &Effect) -> bool {
    match effect {
        Effect::Sequence(inner) => inner.iter().all(harmless),
        Effect::PutSourceOnTopOfLibrary => true,
        e => gains(e),
    }
}

/// Whether the life this cost asks for is life the seat can spare.
///
/// The `+ 5` is the same margin the pay-life-or-enter-tapped answer uses;
/// a second threshold for the same question would be two numbers to keep
/// in step.
fn life_ok(view: &PlayerView, cost: &Cost) -> bool {
    let need = cost.parts.iter().fold(0u16, |sum, part| match part {
        CostPart::PayLife(n) => sum.saturating_add(*n),
        _ => sum,
    });
    need == 0
        || view
            .seat(view.seat)
            .is_some_and(|s| s.life > i32::from(need) + 5)
}

/// Whether drawing from these effects would draw from an empty library.
///
/// The one recognised gain that can lose the game on its own (CR 704.5b),
/// so it is bounded by the count the view already carries. A draw whose
/// amount is not a plain number is refused rather than guessed at.
fn draw_is_safe(view: &PlayerView, effects: &[Effect]) -> bool {
    let Some(want) = draws(effects) else {
        return false;
    };
    want == 0 || view.seat(view.seat).is_some_and(|s| s.library_count > want)
}

/// How many cards these effects draw, or `None` when one of them draws an
/// amount that is not a plain number.
fn draws(effects: &[Effect]) -> Option<u32> {
    let mut want = 0u32;
    for effect in effects {
        match effect {
            Effect::Sequence(inner) => want = want.saturating_add(draws(inner)?),
            Effect::DrawCards { amount } => match amount {
                baylee_cards_dsl::Amount::Fixed(n) => want = want.saturating_add(*n),
                _ => return None,
            },
            _ => {}
        }
    }
    Some(want)
}

/// The fetchland to crack, if one is offered.
///
/// It gets its own pass rather than a place in the whitelist because it is
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
        let Some((cost, effects, _)) = activated(def) else {
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
        let ultimate = view
            .object(source)
            .and_then(|o| o.card)
            .and_then(|c| baylee_cards::by_index(c.index))
            .is_some_and(|def| {
                def.abilities_for_face(
                    view.object(source)
                        .and_then(|o| o.card)
                        .map_or(0, |c| c.face) as usize,
                )
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

/// Anything else the whitelist recognises.
///
/// Untargeted only. The agent answers `Pending::ChooseTargets` by taking an
/// opponent's permanents first, which is right for a removal spell and
/// exactly wrong for an ability that helps whatever it points at — and
/// nothing here knows which of the two it is holding.
fn useful(view: &PlayerView, legal: &LegalActions) -> Option<(ObjectId, u32)> {
    legal.abilities.iter().copied().find(|&(source, index)| {
        let Some(def) = printed(view, source, index) else {
            return false;
        };
        let Some((cost, effects, targeted)) = activated(def) else {
            return false;
        };
        !targeted
            && consumes(cost)
            && life_ok(view, cost)
            && draw_is_safe(view, effects)
            && effects.iter().any(gains)
            && effects.iter().all(harmless)
            && !tap_costs_an_attack(view, source, cost)
    })
}

/// Whether tapping this source now is an attack given up.
///
/// A creature tapped in the precombat main is a creature that cannot swing,
/// and the agent has no way to weigh a card against a hit. So it waits:
/// lands, artifacts and planeswalkers are untouched by this, and a
/// creature's tap ability happens after combat instead.
fn tap_costs_an_attack(view: &PlayerView, source: ObjectId, cost: &Cost) -> bool {
    cost.parts.contains(&CostPart::TapSelf)
        && view.active == view.seat
        && view.phase == Phase::FirstMain
        && view
            .object(source)
            .is_some_and(|o| o.types.contains(TypeSet::CREATURE))
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_cards_dsl::Amount;

    static TOP: [Effect; 2] = [
        Effect::DrawCards {
            amount: Amount::Fixed(1),
        },
        Effect::PutSourceOnTopOfLibrary,
    ];

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
    }

    /// X may be zero, so paying it is not a limit on anything.
    #[test]
    fn paying_x_life_is_not_a_limit() {
        assert!(!consumes(&Cost {
            mana: ManaCost::ZERO,
            parts: &[CostPart::PayLifeX],
        }));
    }

    /// An effect the whitelist has never heard of leaves the ability alone,
    /// so a mechanic added tomorrow is inert rather than misplayed.
    #[test]
    fn an_unrecognised_effect_is_not_a_gain() {
        assert!(gains(&Effect::SearchLibrary {
            filter: &baylee_cards_dsl::Filter::CREATURE,
            finds: &[],
            optional: false,
        }));
        assert!(!gains(&Effect::LoseLife {
            amount: Amount::Fixed(1),
            target: baylee_cards_dsl::PlayerRel::You,
        }));
        assert!(!harmless(&Effect::LoseLife {
            amount: Amount::Fixed(1),
            target: baylee_cards_dsl::PlayerRel::You,
        }));
    }

    /// Sensei's Divining Top draws *and* puts itself back. Reading only the
    /// first effect would take abilities whose second half is a cost; reading
    /// only "all harmless" would take an ability that does nothing at all.
    #[test]
    fn a_sequence_is_read_all_the_way_through() {
        let sequence = Effect::Sequence(&TOP);
        assert!(gains(&sequence), "the draw inside the sequence was missed");
        assert!(
            harmless(&sequence),
            "putting the source back read as a harm"
        );
        assert!(
            TOP.iter().any(gains) && TOP.iter().all(harmless),
            "the same two effects fail as a flat list"
        );
    }
}
