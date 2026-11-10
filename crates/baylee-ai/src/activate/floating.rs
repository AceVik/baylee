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
//! a fixed mana cost and nothing else but tapping itself, at a moment its
//! timing allows. Mana kept up for a counterspell or a trick is not spent
//! here ([`HeuristicAgent::reserve`]).

use baylee_cards_dsl::{AbilityDef, ActivationLimit, ActivationTiming, ActivationZone, CostPart};
use baylee_client_core::decision::resource_player;
use baylee_client_core::manaplan::Tap;
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
    let mut best: Option<(i64, baylee_core::ids::ObjectId, &'static AbilityDef)> = None;
    for object in view.battlefield_of(view.seat) {
        if object.status.contains(ObjectStatus::PHASED_OUT) {
            continue;
        }
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
            let price = cost
                .mana
                .cmc()
                .saturating_add(legal.activation_increase(object.id));
            if cost.mana.cmc() == 0 || cost.mana.has_variable() || u64::from(price) > room {
                continue;
            }
            if cost.parts.iter().any(|part| *part != CostPart::TapSelf) {
                continue;
            }
            if cost.parts.contains(&CostPart::TapSelf)
                && (object.status.contains(ObjectStatus::TAPPED)
                    || object.types.contains(TypeSet::CREATURE) && object.summoning_sick)
            {
                continue;
            }
            if *timing == ActivationTiming::SorcerySpeed && !main {
                continue;
            }
            let Ok(index) = u32::try_from(index) else {
                continue;
            };
            if legal.abilities.contains(&(object.id, index)) || on_the_stack(view, object.id, index)
            {
                continue;
            }
            let Some(worth) = agent.activation_worth(view, Origin::of(view, object.id), def) else {
                continue;
            };
            if worth > THRESHOLD
                && best.is_none_or(|(w, id, _)| {
                    (worth, std::cmp::Reverse(object.id)) > (w, std::cmp::Reverse(id))
                })
            {
                best = Some((worth, object.id, def));
            }
        }
    }
    let (_, source, def) = best?;
    let AbilityDef::Activated { cost, .. } = def else {
        return None;
    };
    let mana = cost
        .mana
        .with_more_generic(legal.activation_increase(source));
    let plan = crate::policy::plan_for_activation(
        &mana,
        &seat.mana_pool,
        view,
        legal,
        source,
        cost.parts.contains(&CostPart::TapSelf),
    )?;
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
