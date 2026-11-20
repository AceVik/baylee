//! Mana made for an ability worth its price ([`AIProfile::ABILITY_MANA`]).
//!
//! The engine offers an ability with a mana cost only once that mana floats
//! (`Engine::can_afford`), and nothing in the house made mana for one the
//! way `policy::aim` makes it for a spell. So every level left an equip
//! cost, a pump it pays for, a regeneration shield and a draw engine alone
//! unless mana happened to float: `docs/house-ai.md` carried it as open
//! ("an ability with a mana cost is taken only when the mana already
//! floats"). A level that manages its open mana with care
//! ([`HoldUp::ThreatAware`]: `sharp` and `expert`)
//! now taps toward the ability worth the most on this board, by the same
//! measure that takes an offered one ([`super::best`]), one source at a
//! time; once the mana floats the engine offers the ability and
//! [`super::choose`] takes it.
//!
//! Only shapes the offer will follow: an ability printed on a permanent of
//! this seat's, used from the battlefield, with no limit and no condition,
//! a fixed mana cost and an optional tap, at a moment its timing allows.
//! From generation 20, a fixed life payment may accompany that cost when
//! the engine lists only mana as unpaid. Mana reserved for interaction is
//! kept up ([`HeuristicAgent::reserve`]).

use baylee_cards_dsl::{
    AbilityDef, ActivationLimit, ActivationTiming, ActivationZone, Cost, CostPart,
};
use baylee_client_core::decision::resource_player;
use baylee_client_core::manaplan::Tap;
use baylee_core::ids::ObjectId;
use baylee_core::mana::ManaCost;
use baylee_core::preset::{AIProfile, HoldUp};
use baylee_core::types::TypeSet;
use baylee_engine::choice::{LegalActions, PlayerAction};
use baylee_view::{ObjectStatus, Phase, PlayerView};

use super::{on_the_stack, printed_list};
use crate::HeuristicAgent;
use crate::worth::{Origin, THRESHOLD};

impl HeuristicAgent {
    /// Whether this level makes mana for an ability: one that manages its
    /// open mana with care, from [`AIProfile::ABILITY_MANA`].
    pub(crate) fn taps_for_abilities(&self) -> bool {
        self.profile.hold_up == HoldUp::ThreatAware && self.profile.plays(AIProfile::ABILITY_MANA)
    }
}

/// The next tap toward the ability worth the most that the engine does not
/// offer yet for want of mana, when one is worth more than [`THRESHOLD`] net
/// of its cost; `None` otherwise, which is most of the time.
pub(crate) fn float_for(
    view: &PlayerView,
    legal: &LegalActions,
    agent: &HeuristicAgent,
) -> Option<PlayerAction> {
    let seat = view.seat(resource_player(view))?;
    let main = view.active == view.seat
        && view.stack.is_empty()
        && matches!(view.phase, Phase::FirstMain | Phase::SecondMain);
    let sources = crate::policy::sources(view, legal);
    let available =
        seat.mana_pool.total() + sources.iter().map(|s| u64::from(s.amount)).sum::<u64>();
    let room = available.saturating_sub(u64::from(agent.reserve(view, main)));
    let mut best: Option<(i64, ObjectId, ManaCost, bool)> = None;
    for object in view.battlefield_of(view.seat) {
        if object.status.contains(ObjectStatus::PHASED_OUT) {
            continue;
        }
        consider_turn_face_up(view, legal, agent, object, room, &mut best);
        for (index, def) in printed_list(object).iter().enumerate() {
            let AbilityDef::Activated {
                cost,
                timing,
                mana_ability: false,
                zone: ActivationZone::Battlefield,
                limit: ActivationLimit::Unlimited,
                cost_reduction: None,
                ..
            } = def
            else {
                continue;
            };
            if cost.mana.cmc() == 0 || cost.mana.has_variable() {
                continue;
            }
            let Ok(index) = u32::try_from(index) else {
                continue;
            };
            let Some(price) = funding_cost(legal, agent.profile(), object.id, index, cost) else {
                continue;
            };
            if u64::from(price.cmc()) > room {
                continue;
            }
            let taps_self = cost.parts.contains(&CostPart::TapSelf);
            if taps_self
                && (object.status.contains(ObjectStatus::TAPPED)
                    || object.types.contains(TypeSet::CREATURE) && object.summoning_sick)
            {
                continue;
            }
            if *timing == ActivationTiming::SorcerySpeed && !main {
                continue;
            }
            if legal.abilities.contains(&(object.id, index)) || on_the_stack(view, object.id, index)
            {
                continue;
            }
            let Some(worth) = agent.activation_worth(view, Origin::of(view, object.id), def) else {
                continue;
            };
            if worth > THRESHOLD
                && best.is_none_or(|(w, id, _, _)| {
                    (worth, std::cmp::Reverse(object.id)) > (w, std::cmp::Reverse(id))
                })
            {
                best = Some((worth, object.id, price, taps_self));
            }
        }
    }
    if main {
        float_graveyard_activations(view, legal, agent, room, &mut best);
        float_hand_plot_activations(view, legal, room, &mut best);
    }
    let (_, source, mana, taps_self) = best?;
    // From generation 10 through the shared planner, which keeps a second
    // Port untapped for its own ability and reads every restricted tap the
    // engine names for this one.
    let plan = if agent.profile().plays(AIProfile::FULL_CASTS) {
        crate::plan_for_activation(&mana, &seat.mana_pool, view, legal, source, taps_self)
    } else {
        crate::policy::plan_for_activation(&mana, &seat.mana_pool, view, legal, source, taps_self)
    }?;
    // A plan with nothing to tap means the pool pays already, and the
    // engine would be offering the ability: something else stops it.
    let step = plan.steps.first()?;
    Some(match step.tap {
        Tap::Intrinsic => PlayerAction::ActivateManaAbility {
            source: step.source,
        },
        Tap::Ability(ability_index) => PlayerAction::ActivateAbility {
            source: step.source,
            ability_index,
        },
    })
}

fn funding_cost(
    legal: &LegalActions,
    profile: AIProfile,
    source: ObjectId,
    index: u32,
    cost: &Cost,
) -> Option<ManaCost> {
    if cost.parts.iter().all(|part| *part == CostPart::TapSelf) {
        return Some(
            cost.mana
                .with_more_generic(legal.activation_increase(source)),
        );
    }
    if !profile.plays(AIProfile::PAY_LIFE_ACTIVATION_MANA)
        || cost
            .parts
            .iter()
            .any(|part| !matches!(part, CostPart::TapSelf | CostPart::PayLife(_)))
    {
        return None;
    }
    // The hint has already checked life payment, timing, targets and locks.
    // Worth keeps a safe life margin, and each subsequent tap rechecks it.
    legal
        .unpaid_abilities
        .iter()
        .find_map(|&(id, ability, mana)| (id == source && ability == index).then_some(mana))
}

fn consider_turn_face_up(
    view: &PlayerView,
    legal: &LegalActions,
    agent: &HeuristicAgent,
    object: &baylee_view::PublicObject,
    room: u64,
    best: &mut Option<(i64, ObjectId, ManaCost, bool)>,
) {
    if !object.status.is_face_down()
        || legal
            .abilities
            .contains(&(object.id, baylee_engine::choice::TURN_FACE_UP))
    {
        return;
    }
    let Some(price) = legal.unpaid_abilities.iter().find_map(|&(id, ab, mana)| {
        (id == object.id && ab == baylee_engine::choice::TURN_FACE_UP).then_some(mana)
    }) else {
        return;
    };
    if u64::from(price.cmc()) > room {
        return;
    }
    let Some(worth) = agent.turn_face_up_worth(view, object.id) else {
        return;
    };
    if worth > THRESHOLD
        && best.is_none_or(|(w, id, _, _)| {
            (worth, std::cmp::Reverse(object.id)) > (w, std::cmp::Reverse(id))
        })
    {
        *best = Some((worth, object.id, price, false));
    }
}

fn float_graveyard_activations(
    view: &PlayerView,
    legal: &LegalActions,
    agent: &HeuristicAgent,
    room: u64,
    best: &mut Option<(i64, ObjectId, ManaCost, bool)>,
) {
    let Some(grave) = view.graveyards.get(usize::from(view.seat.get())) else {
        return;
    };
    for object in grave {
        for (index, def) in printed_list(object).iter().enumerate() {
            let (AbilityDef::Unearth { cost } | AbilityDef::Encore { cost }) = def else {
                continue;
            };
            if cost.mana.cmc() == 0 || cost.mana.has_variable() {
                continue;
            }
            let Ok(index) = u32::try_from(index) else {
                continue;
            };
            if legal.abilities.contains(&(object.id, index)) || on_the_stack(view, object.id, index)
            {
                continue;
            }
            let Some(price) = legal
                .unpaid_abilities
                .iter()
                .find_map(|&(id, ab, mana)| (id == object.id && ab == index).then_some(mana))
            else {
                continue;
            };
            if u64::from(price.cmc()) > room {
                continue;
            }
            let Some(worth) = agent.activation_worth(view, Origin::of(view, object.id), def) else {
                continue;
            };
            if worth > THRESHOLD
                && best.is_none_or(|(w, id, _, _)| {
                    (worth, std::cmp::Reverse(object.id)) > (w, std::cmp::Reverse(id))
                })
            {
                *best = Some((worth, object.id, price, false));
            }
        }
    }
}

fn float_hand_plot_activations(
    view: &PlayerView,
    legal: &LegalActions,
    room: u64,
    best: &mut Option<(i64, ObjectId, ManaCost, bool)>,
) {
    if view.phase != Phase::SecondMain {
        return;
    }
    for card in &view.hand {
        if legal.castable_once_paid(card.id)
            || legal
                .abilities
                .contains(&(card.id, baylee_engine::choice::PLOT_CARD))
        {
            continue;
        }
        let is_instant = baylee_cards::by_index(card.card.index)
            .and_then(|def| def.faces.get(usize::from(card.card.face)))
            .is_some_and(|face| face.types.contains(TypeSet::INSTANT));
        if is_instant {
            continue;
        }
        let Some(price) = legal.unpaid_abilities.iter().find_map(|&(id, ab, mana)| {
            (id == card.id && ab == baylee_engine::choice::PLOT_CARD).then_some(mana)
        }) else {
            continue;
        };
        if u64::from(price.cmc()) > room {
            continue;
        }
        let worth = 120_i64;
        if worth > THRESHOLD
            && best.is_none_or(|(w, id, _, _)| {
                (worth, std::cmp::Reverse(card.id)) > (w, std::cmp::Reverse(id))
            })
        {
            *best = Some((worth, card.id, price, false));
        }
    }
}
