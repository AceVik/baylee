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
use baylee_view::{Phase, PlayerView, PublicObject};

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
pub(crate) fn choose(
    view: &PlayerView,
    legal: &LegalActions,
    agent: &crate::HeuristicAgent,
) -> Option<(ObjectId, u32)> {
    fetch(view, legal)
        .or_else(|| {
            if agent.profile.lookahead > 0 {
                thoughtful_loyalty(view, legal, agent)
            } else {
                loyalty(view, legal)
            }
        })
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
/// loyalty delta the engine has already checked, and it is chosen by
/// [`loyalty`] rather than by the whitelist.
fn activated(def: &'static AbilityDef) -> Option<(&'static Cost, &'static [Effect], bool)> {
    match def {
        AbilityDef::Activated {
            cost,
            effects,
            targets,
            mana_ability,
            ..
        }
        | AbilityDef::ActivatedConditional {
            cost,
            effects,
            targets,
            mana_ability,
            ..
        } => (!*mana_ability).then_some((cost, *effects, targets.is_some())),
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

/// Whether the cost gives up a card other than the source: a permanent
/// sacrificed, a card discarded or exiled from hand.
///
/// Every gain [`gains`] recognises is small — a card, a scry, a few life, a
/// counter, a token — and what such a cost takes is at least as much, so it
/// is a trade this whitelist cannot weigh. Measured through the engine: a
/// Zuran Orb beside four Forests was fed all four on turn 1 for 8 life, and
/// a Viscera Seer sacrificed itself to scry 1. The source paying for itself
/// (a fetchland, cycling) is not this: it is what that ability is for.
///
/// Exhaustive with no wildcard, for the reason [`consumes`] gives.
fn gives_up_a_card(cost: &Cost) -> bool {
    cost.parts.iter().any(|part| match part {
        CostPart::Sacrifice(_) | CostPart::Discard(_) | CostPart::ExileFromHand(_) => true,
        CostPart::TapSelf
        | CostPart::UntapSelf
        | CostPart::SacrificeSelf
        | CostPart::DiscardSelf
        | CostPart::ExileSelf
        | CostPart::ReturnSelfToHand
        | CostPart::PayLife(_)
        | CostPart::PayLifeX
        | CostPart::RemoveCounterSelf { .. }
        | CostPart::RemoveCounterSelfX { .. }
        | CostPart::PutCounterSelf { .. }
        // A tapped creature stays where it is.
        | CostPart::TapOther(_)
        // A returned permanent comes back to hand, not to the graveyard.
        | CostPart::ReturnToHand(_)
        // A card already in the graveyard is the cheapest there is.
        | CostPart::ExileFromGraveyard(_) => false,
    })
}

/// Whether this effect is one the agent recognises as a gain.
///
/// An optional clause is a gain the seat can still decline, so what it *may*
/// do is what it is worth — and which effects have an inside at all is
/// [`Effect::branches`]' question rather than this one's. It used to name
/// `Sequence` and `MayDo` and stop there, so a gain printed inside a kicker
/// clause or behind "unless you pay" was an ability the agent never
/// activated.
fn gains(effect: &Effect) -> bool {
    let (then, otherwise) = effect.branches();
    if !then.is_empty() || !otherwise.is_empty() {
        return then.iter().chain(otherwise).any(gains);
    }
    matches!(
        effect,
        Effect::SearchLibrary { .. }
            | Effect::DrawCards { .. }
            | Effect::Scry { .. }
            | Effect::CreateToken { .. }
            | Effect::CreateTokenN { .. }
            | Effect::Amass { .. }
            | Effect::AddCounter { .. }
            | Effect::GainLife { .. }
    )
}

/// Whether this effect is one the agent knows will not hurt it.
///
/// Everything it counts as a gain, plus the bookkeeping that rides along
/// with one: Sensei's Divining Top draws a card *and* puts itself back on
/// the library, and refusing the second half would refuse the first.
///
/// A wrapper is as harmless as everything inside it, which is why this is
/// `all` where [`gains`] is `any`: one clause the agent cannot vouch for is
/// enough to leave the ability alone.
fn harmless(effect: &Effect) -> bool {
    let (then, otherwise) = effect.branches();
    if !then.is_empty() || !otherwise.is_empty() {
        return then.iter().chain(otherwise).all(harmless);
    }
    // A surveil is the second of these, and it is the split these two
    // functions exist for. This agent answers a surveil by keeping
    // everything — it cannot read a card well enough to decide one is
    // worth binning — so a surveil never costs it anything and never
    // wins it anything either. Calling it a gain would have it paying
    // `{2}{U}` for a no-op; leaving it out of *both* would have it
    // declining a draw that happened to surveil alongside.
    matches!(
        effect,
        Effect::PutSourceOnTopOfLibrary | Effect::Surveil { .. }
    ) || gains(effect)
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
///
/// The agent answers every optional clause with yes, so a draw inside one is
/// a draw it will take — counting it as zero would be the deck-out this
/// whole function exists to refuse. For the same reason both halves of a
/// two-branch effect are **summed** although only one of them runs: this
/// function is allowed to be wrong in one direction only, and over-counting
/// refuses a safe draw where under-counting loses the game (CR 704.5b).
fn draws(effects: &[Effect]) -> Option<u32> {
    let mut want = 0u32;
    for effect in effects {
        let (then, otherwise) = effect.branches();
        want = want.saturating_add(draws(then)?);
        want = want.saturating_add(draws(otherwise)?);
        if let Effect::DrawCards { amount } = effect {
            match amount {
                baylee_cards_dsl::Amount::Fixed(n) => want = want.saturating_add(*n),
                _ => return None,
            }
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

/// Price the actual effect and the loyalty spent. Removal can save a walker
/// or its controller; a large plus that does nothing cannot compete with it.
fn thoughtful_loyalty(
    view: &PlayerView,
    legal: &LegalActions,
    agent: &crate::HeuristicAgent,
) -> Option<(ObjectId, u32)> {
    legal
        .abilities
        .iter()
        .copied()
        .filter_map(|(source, index)| {
            let AbilityDef::Loyalty { cost, effects, .. } = printed(view, source, index)? else {
                return None;
            };
            let mut value = agent.effect_value(view, effects) + i64::from(*cost) * 45;
            let loyalty = view.object(source).map_or(0, |o| {
                o.counters
                    .iter()
                    .filter(|c| c.kind == baylee_view::CounterKind::Loyalty)
                    .map(|c| u32::from(c.count))
                    .sum::<u32>()
            });
            if *cost < 0 && loyalty == u32::from(cost.unsigned_abs()) {
                value -= 300;
            }
            (value > 0).then_some((value, (source, index)))
        })
        .max_by_key(|(value, handle)| (*value, std::cmp::Reverse(*handle)))
        .map(|(_, handle)| handle)
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
            && !gives_up_a_card(cost)
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

    /// Every printed ability [`useful`] would take but for
    /// [`gives_up_a_card`]: `(what prints it, the ability's index)`.
    ///
    /// The half of `useful` a card can answer on its own — untargeted, a
    /// cost that consumes, a whitelisted gain, nothing harmful. Life, a safe
    /// draw and a lost attack are the board's, so a card they would stop on
    /// some board is counted here all the same. Per face, over the cards and
    /// the tokens both, because `printed_list` reads both.
    fn refused_for_the_card_they_cost() -> Vec<(String, usize)> {
        let lists = baylee_cards::all()
            .flat_map(|def| {
                (0..def.faces.len()).map(move |face| {
                    (
                        def.faces[face].name.to_owned(),
                        def.abilities_for_face(face),
                    )
                })
            })
            .chain(
                baylee_cards::tokens::ALL
                    .iter()
                    .map(|token| (token.name.to_owned(), token.abilities)),
            );
        let mut refused = Vec::new();
        for (name, abilities) in lists {
            for (index, ability) in abilities.iter().enumerate() {
                let Some((cost, effects, targeted)) = activated(ability) else {
                    continue;
                };
                if !targeted
                    && consumes(cost)
                    && effects.iter().any(gains)
                    && effects.iter().all(harmless)
                    && gives_up_a_card(cost)
                {
                    refused.push((name.clone(), index));
                }
            }
        }
        refused
    }

    /// How many abilities the card-for-a-small-gain rule turns down: 38 on
    /// 24.09.2026, printed by 36 cards and the Blood token (Wand of the
    /// Elements prints two) — Zuran Orb and Viscera Seer among them,
    /// Survival of the Fittest and Carrion Feeder too.
    ///
    /// Bounds and not the number, so a card batch does not turn this red by
    /// itself. Each bound refuses a wrong [`gives_up_a_card`], measured by
    /// injection: one that forgets `Discard(_)` counts 28, under the floor;
    /// one that also counts the source paying for itself (a fetchland, a
    /// cycler) counts 223, over the ceiling.
    #[test]
    fn the_abilities_refused_for_the_card_they_cost_are_counted() {
        let refused = refused_for_the_card_they_cost();
        assert!(
            (30..=60).contains(&refused.len()),
            "{} abilities are refused for the card they cost, measured 38 on \
             24.09.2026: {refused:?}",
            refused.len()
        );
    }

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

    /// A surveil is harmless and is not a gain, and the pair is the point.
    ///
    /// This agent answers a surveil by keeping everything, so an ability
    /// whose whole text is a surveil buys it nothing — but one that draws a
    /// card *and* surveils is still worth the mana. Merging these two lists
    /// would cost one of the two.
    #[test]
    fn a_surveil_is_harmless_and_is_not_a_gain() {
        assert!(harmless(&Effect::surveil(1)));
        assert!(!gains(&Effect::surveil(1)));
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

    /// A clause behind a price is still a clause.
    ///
    /// `PlayerMayPayOr` carries a single `&'static Effect` rather than a
    /// list, and every hand-rolled walker in this workspace descended into
    /// the lists and stopped there — so "unless you pay {1}, draw a card"
    /// read as an effect with nothing in it. Both readers are asked, because
    /// they disagree about wrappers by design: `gains` is `any` and
    /// `harmless` is `all`, and a conversion that fixed one and not the
    /// other would leave the ability still unactivated.
    ///
    /// Synthetic and not a pool card on purpose: the census behind #109 says
    /// the pool hides `SacrificeSelf`, `DrawCards`, `CounterTargetSpell` and
    /// `CreateToken` behind that clause, and *none* of them sits in an
    /// activated ability — so no card here changes these two answers today
    /// and a test claiming otherwise would be about a card that does not
    /// exist. What is being pinned is the descent.
    #[test]
    fn a_gain_behind_a_price_is_still_a_gain() {
        static DRAW: Effect = Effect::DrawCards {
            amount: Amount::Fixed(1),
        };
        let priced = Effect::PlayerMayPayOr {
            player: baylee_cards_dsl::PlayerRel::Opponent,
            mana: Amount::Fixed(1),
            effect: &DRAW,
        };
        assert!(gains(&priced), "the draw behind the price was missed");
        assert!(
            harmless(&priced),
            "the draw behind the price read as a harm"
        );
        assert_eq!(
            draws(std::slice::from_ref(&priced)),
            Some(1),
            "a draw the agent will take has to count against the library"
        );
    }
}
