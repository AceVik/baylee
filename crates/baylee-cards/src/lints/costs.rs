//! Costs: their order, what they spend, the numbers they announce, and the
//! waterbend wizard.

use super::*;

/// Does paying this part ask the engine for the source, where the source
/// still is?
///
/// **Exhaustive with no wildcard**, for the reason [`branches`] is: a new
/// [`CostPart`] has to be classified here before the crate compiles, and the
/// cost of getting that wrong is a lint that passes over the one shape it
/// was written to find.
pub(super) const fn needs_the_source(part: &CostPart) -> bool {
    match part {
        CostPart::TapSelf
        | CostPart::UntapSelf
        | CostPart::SacrificeSelf
        | CostPart::DiscardSelf
        | CostPart::ExileSelf
        | CostPart::ReturnSelfToHand
        | CostPart::RemoveCounterSelf { .. }
        | CostPart::RemoveCounterSelfX { .. }
        | CostPart::PutCounterSelf { .. } => true,
        // These ask a *player* something, or ask about another permanent.
        // None of them looks the source up, so none of them cares whether it
        // is still there.
        CostPart::Sacrifice(_)
        | CostPart::Discard(_)
        | CostPart::TapOther(_)
        | CostPart::Crew(_)
        | CostPart::ReturnToHand(_)
        | CostPart::ExileFromGraveyard(_)
        | CostPart::PayLife(_)
        | CostPart::PayLifeX
        | CostPart::ExileFromHand(_) => false,
    }
}

/// Does paying this part move the source out of the zone it is being paid
/// in?
pub(super) const fn moves_the_source(part: &CostPart) -> bool {
    match part {
        CostPart::SacrificeSelf
        | CostPart::DiscardSelf
        | CostPart::ExileSelf
        | CostPart::ReturnSelfToHand => true,
        CostPart::TapSelf
        | CostPart::UntapSelf
        | CostPart::RemoveCounterSelf { .. }
        | CostPart::RemoveCounterSelfX { .. }
        | CostPart::PutCounterSelf { .. }
        | CostPart::Sacrifice(_)
        | CostPart::Discard(_)
        | CostPart::TapOther(_)
        | CostPart::Crew(_)
        | CostPart::ReturnToHand(_)
        | CostPart::ExileFromGraveyard(_)
        | CostPart::PayLife(_)
        | CostPart::PayLifeX
        | CostPart::ExileFromHand(_) => false,
    }
}

/// The first part of a cost that is paid after the source has already left,
/// as `(what moved it, what then asked for it)`.
///
/// `Engine::pay_cost` walks a cost's parts in the order the card prints
/// them, and four of them move the source somewhere else. Everything after
/// one of those is asked of an object that is no longer where it was looked
/// for — and each one fails a different quiet way. `TapSelf` taps nothing
/// and journals that it did. `RemoveCounterSelf` finds no counters, refuses,
/// and leaves the cost **half paid**: the permanent is already in the
/// graveyard and the ability was never activated.
///
/// It is decidable here, without an engine and without a board, because
/// Magic prints these in one order and one only — "{T}, Sacrifice this
/// land:", "Remove two pressure counters and sacrifice this" — so a cost
/// written the other way round is a transcription, not a card. That is what
/// makes a lint the right answer rather than a runtime refusal: the engine
/// would be refusing something no printing asks for, at the one moment a
/// player cannot be told why.
pub(super) fn cost_order_fault(parts: &[CostPart]) -> Option<(CostPart, CostPart)> {
    let mut gone: Option<CostPart> = None;
    for part in parts {
        if let Some(mover) = gone
            && needs_the_source(part)
        {
            return Some((mover, *part));
        }
        if moves_the_source(part) {
            gone = Some(*part);
        }
    }
    None
}

/// Every cost list an ability carries, as the parts `pay_cost` would walk.
///
/// Two sources and they are not the same shape: an activated ability's own
/// cost, and the cost of an activated ability a continuous effect *grants*,
/// which is printed on no card and is reached exactly the way
/// [`layer_fault`] reaches a modifier — from the static ability, and from
/// every effect that creates one.
pub(super) fn cost_lists(ability: &AbilityDef) -> Vec<&'static [CostPart]> {
    costs(ability).into_iter().map(|cost| cost.parts).collect()
}

/// The same walk as [`cost_lists`], before it throws the mana away.
///
/// Split out rather than copied, because the two would answer "which costs
/// does an ability have" differently the first time a door was added to one
/// of them — which is the whole finding [`cost_lists`]'s own sweep is built
/// to catch, and it would be blind to a second reader drifting from it.
pub(super) fn costs(ability: &AbilityDef) -> Vec<Cost> {
    let own = match ability {
        AbilityDef::Activated { cost, .. } | AbilityDef::ActivatedConditional { cost, .. } => {
            vec![*cost]
        }
        _ => Vec::new(),
    };
    let from_static = match ability {
        AbilityDef::Static(sa) => vec![sa.modifier],
        _ => Vec::new(),
    };
    own.into_iter()
        .chain(
            from_static
                .into_iter()
                .chain(
                    branches(ability)
                        .into_iter()
                        .flat_map(|branch| branch.effects.iter().flat_map(declared_layers))
                        .map(|(_, modifier)| modifier),
                )
                .filter_map(|modifier| match modifier {
                    Modifier::GrantActivated { cost, .. } => Some(cost),
                    _ => None,
                }),
        )
        .collect()
}

/// A cost that would have the player announce **two different numbers for
/// one `X`**, as the counter part that asks for the first.
///
/// CR 601.2b, reached for an activation through CR 602.2b, announces the
/// number once, and CR 107.3i makes every instance of X on the object that
/// one value. This engine holds that one answer in a single field
/// (`Engine::activation_x`) and asks for it at whichever of the two shapes
/// it meets first — the counter part, which is bounded by what is on the
/// permanent, or the mana `{X}`, which is bounded by the pool. A cost
/// carrying both would be asked about under the counter bound alone and
/// could announce a number the mana cannot pay, which `pay_cost` refuses
/// *after* the ability has been announced.
///
/// There is a **third** kind of X and it is guarded elsewhere:
/// `CostPart::PayLifeX` is paid by the casting wizard and skipped by
/// `pay_cost`, which is right for a spell and would hand an activation half
/// its cost for free, so the engine's own
/// `offer_tests::nothing_in_the_pool_carries_an_activated_cost_the_engine_would_skip`
/// keeps it off an activated ability altogether. It is not a finding here,
/// because on a *spell* the wizard announces both and they agree.
///
/// So the engine's single field rests on a property of the pool, and this
/// is that property written where a build fails on it — the same bargain as
/// [`announced_number_beside_a_target`], one rule further along.
pub(super) fn two_xs_in_one_cost(cost: &Cost) -> Option<CostPart> {
    if !cost.mana.has_variable() {
        return None;
    }
    cost.parts
        .iter()
        .find(|part| matches!(part, CostPart::RemoveCounterSelfX { .. }))
        .copied()
}

/// An activated ability that **announces a number as it is activated and
/// also names a target**, as `(the counter, what it targets)`.
///
/// [`CostPart::RemoveCounterSelfX`] is the storage lands' cost — "remove any
/// number of storage counters from this land" — and the number is asked for
/// at CR 601.2b, with the activation, *before* targets are chosen
/// (CR 601.2c) — both reached for an activated ability through CR 602.2b. The printed spelling on eleven of those cards is not an `X`
/// at all but "any number of", which is a choice made as the cost is **paid**
/// (CR 601.2h), after targets. This engine asks at the earlier moment for
/// both, and that is legal for one spelling and merely unobservable for the
/// other — unobservable for exactly as long as no card chooses a target in
/// between.
///
/// So the decision in [`CostPart::RemoveCounterSelfX`]'s own documentation
/// rests on a property of the pool, and this is that property written down
/// where a build can fail on it. The day a card prints both, the order stops
/// being a simplification and becomes a wrong answer: the player is asked how
/// many counters to spend before being shown what the ability can point at.
///
/// It reads the ability's **own** cost. A [`Modifier::GrantActivated`]
/// carries a `Cost` too and cannot be a finding, because it has no target
/// field at all — a granted activated ability targets nothing, so the pair
/// this is about cannot exist there. The sweep below says that with a count
/// rather than leaving it unsaid.
pub(super) fn announced_number_beside_a_target(
    ability: &AbilityDef,
) -> Option<(crate::dsl::CounterKind, TargetSpec)> {
    let (cost, target) = match ability {
        AbilityDef::Activated { cost, targets, .. }
        | AbilityDef::ActivatedConditional { cost, targets, .. } => (cost, (*targets)?.spec),
        _ => return None,
    };
    cost.parts.iter().find_map(|part| match part {
        CostPart::RemoveCounterSelfX { kind } => Some((*kind, target)),
        _ => None,
    })
}

/// Every cost list a card's **faces** carry, which is the door
/// [`cost_lists`] is not.
///
/// An alternative cost (CR 601.2b) is printed on a face rather than on an
/// ability — Force of Will's "exile a blue card from your hand and pay 1
/// life" — so it is reached from the `CardDef` and from nowhere else. It
/// holds `CostPart`s like any other cost and `Engine::pay_cost` walks it the
/// same way, which is what makes it a door rather than a curiosity: a sweep
/// that opens only the ability costs reports a clean pool having read part of
/// it.
///
/// It was found by probing the guard in
/// [`no_cost_announces_a_number_on_an_ability_that_also_targets`] rather than
/// by reading, which is the argument for that guard.
pub(super) fn face_cost_lists(def: &CardDef) -> Vec<&'static [CostPart]> {
    face_costs(def).into_iter().map(|cost| cost.parts).collect()
}

/// The face-level door with its mana still on it, for the same reason
/// [`costs`] exists beside [`cost_lists`].
pub(super) fn face_costs(def: &CardDef) -> Vec<Cost> {
    def.faces
        .iter()
        .flat_map(|face| face.alternative_costs.iter().map(|alt| alt.cost))
        .collect()
}

/// What keeps a face's waterbend from being the one the cast wizard asks.
///
/// `FaceDef.waterbend` says the face's optional additional cost is a waterbend
/// cost, and the wizard reads it that way (#229): it asks for taps once
/// `additional_costs` is paid and bounds them by that cost's generic mana
/// (CR 701.67b), on the stage convoke asks on. So a waterbend face needs an
/// additional cost of generic mana alone, and no convoke beside it.
pub(super) fn waterbend_fault(face: &FaceDef) -> Option<&'static str> {
    if !face.waterbend {
        return None;
    }
    if face.convoke {
        Some("prints convoke too, and one tap question cannot ask for both")
    } else if face.additional_costs.is_empty() {
        Some("has no additional cost to waterbend")
    } else if face
        .additional_costs
        .iter()
        .any(|c| !c.parts.is_empty() || c.mana.generic_total() != c.mana.cmc() || c.mana.cmc() == 0)
    {
        Some("waterbends a cost that is not generic mana alone")
    } else {
        None
    }
}
