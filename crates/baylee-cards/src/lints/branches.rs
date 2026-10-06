//! Walking an ability's branches, and what its targets and sweeps reach.

use super::*;

/// One `(what it targets, what it does)` pair out of an ability.
///
/// An ability is not always one of these: a modal spell is one per mode,
/// because each mode targets for itself.
#[derive(Clone, Copy, Debug)]
pub(super) struct Branch {
    /// What the branch targets, if anything.
    pub(super) target: Option<TargetSpec>,
    /// What it does when it resolves.
    pub(super) effects: &'static [Effect],
}

/// Every `(target, effects)` pair an ability resolves through.
///
/// A trailing [`Effect::Reflexive`] is cut off the list it ends and becomes
/// a branch of its own, with its own target. That is what it is at
/// resolution: a triggered ability of its own (CR 603.12), whose target is
/// chosen as it goes on the stack, and which the outer ability's target
/// never reaches. Read as one branch, Eden's "another target permanent
/// card" sat in a list that targets nothing, and every lint here that asks
/// what a list is pointed at could not see it.
/// `every_reflexive_sits_where_it_can_trigger` makes the last op the only
/// place a reflexive can stand, so only that place is read here.
pub(super) fn branches(ability: &AbilityDef) -> Vec<Branch> {
    printed_branches(ability)
        .into_iter()
        .flat_map(split_reflexive)
        .collect()
}

/// A branch, and its trailing reflexive trigger's body as a branch of its
/// own, recursively.
pub(super) fn split_reflexive(branch: Branch) -> Vec<Branch> {
    let Some((
        Effect::Reflexive {
            when: _,
            effects,
            target,
        },
        outer,
    )) = branch.effects.split_last()
    else {
        return vec![branch];
    };
    let mut out = vec![Branch {
        target: branch.target,
        effects: outer,
    }];
    out.extend(split_reflexive(Branch {
        target: *target,
        effects,
    }));
    out
}

/// Every `(target, effects)` pair an ability prints, before a reflexive
/// trigger is cut out of one. See [`branches`].
///
/// The `match` is deliberately **exhaustive with no wildcard arm**: a new
/// [`AbilityDef`] variant must be classified here before the crate compiles,
/// because a variant silently falling into a `_ => vec![]` would leave every
/// lint in this module quietly passing over it.
pub(super) fn printed_branches(ability: &AbilityDef) -> Vec<Branch> {
    let modal = |modes: &'static [SpellMode]| {
        modes
            .iter()
            .map(|m| Branch {
                target: m.targets.map(|t| t.spec),
                effects: m.effects,
            })
            .collect()
    };
    match ability {
        AbilityDef::Spell {
            effects, targets, ..
        } => vec![Branch {
            target: targets.map(|t| t.spec),
            effects,
        }],
        AbilityDef::Triggered {
            effects, targets, ..
        } => vec![Branch {
            target: targets.map(|t| t.spec),
            effects,
        }],
        AbilityDef::Activated {
            effects, targets, ..
        }
        | AbilityDef::ActivatedConditional {
            effects, targets, ..
        } => vec![Branch {
            target: targets.map(|t| t.spec),
            effects,
        }],
        AbilityDef::SagaChapter {
            effects, targets, ..
        } => vec![Branch {
            target: targets.map(|req| req.spec),
            effects,
        }],
        // A loyalty ability states a count as well as a filter, and only the
        // filter is a branch's business: these lints ask what an effect list
        // is *pointed* at, not how many of them it may point at.
        AbilityDef::Loyalty {
            effects, targets, ..
        } => vec![Branch {
            target: targets.map(|req| req.spec),
            effects,
        }],
        AbilityDef::ModalSpell { modes, .. } | AbilityDef::ModalTriggered { modes, .. } => {
            modal(modes)
        }
        // Nothing that resolves through an effect list of its own: a
        // keyword the engine synthesises, a cost, a continuous rule, or a
        // choice made as the permanent enters.
        AbilityDef::Unimplemented
        | AbilityDef::Ward { .. }
        | AbilityDef::Toxic { .. }
        | AbilityDef::Prepared { .. }
        | AbilityDef::Echo { .. }
        | AbilityDef::Static(_)
        | AbilityDef::Replacement(_)
        | AbilityDef::Suspend { .. }
        | AbilityDef::CopyOnEnterUntilEot { .. }
        | AbilityDef::CopyOnEnter { .. } => Vec::new(),
    }
}

/// The filters an effect applies to **every match of**, rather than to a
/// target.
///
/// This is the list that makes [`target_reuse`] mean something, so it is
/// worth being exact about what belongs on it. A filter can appear in an
/// effect for three different jobs, and only the first is a sweep:
///
/// * *which objects this happens to* — `DestroyAll`, `PumpFilter`. On the
///   list.
/// * *what may be found* — `SearchLibrary`, `WishToHand`. A filter over
///   cards nobody has chosen yet, and reusing the target's filter there is
///   ordinary ("search for a creature card" beside "target creature").
/// * *a condition* — `IfControlGreatestCmc`. Reads the board, changes
///   nothing.
///
/// `Filter::This` is the DSL's way of writing "the target" inside a
/// continuous effect, so it is never a sweep however it is spelled.
///
/// Which effects run another effect is [`Effect::branches`]' question, not
/// this lint's. This file used to answer it twice, in two different matches,
/// and both were short: a sweep behind "unless you pay" was read by neither,
/// and a sweep inside a `Sequence` by only one of them.
pub(super) fn swept_filters(effect: &Effect) -> Vec<&'static Filter> {
    match effect {
        Effect::DestroyAll { filter, .. }
        | Effect::ReturnAllToHand { filter, .. }
        | Effect::SetPTFilter { filter, .. }
        | Effect::AddCounterFilter { filter, .. }
        | Effect::DoubleCountersFilter { filter, .. }
        | Effect::PumpFilter { filter, .. }
        | Effect::SacrificeFilter { filter, .. }
        | Effect::EqualizePermanents { filter }
        | Effect::DealDamageEach { filter, .. }
        | Effect::CreateContinuousEffect { filter, .. } => {
            if matches!(filter, Filter::This) {
                Vec::new()
            } else {
                vec![filter]
            }
        }
        // Anything that runs another effect runs the sweep inside it, and
        // which effects those are is `Effect::branches`' to say rather than
        // this lint's. It used to be listed here and the list was short by
        // three — `Sequence` and both halves of "unless you pay" — so a
        // sweep behind a Karoo's price was a sweep this lint never read.
        _ => {
            let (then, otherwise) = effect.branches();
            then.iter()
                .chain(otherwise)
                .flat_map(swept_filters)
                .collect()
        }
    }
}

/// The filter a target spec picks its target with, when it picks one by
/// characteristics at all.
pub(super) fn target_filter(spec: TargetSpec) -> Option<&'static Filter> {
    match spec {
        TargetSpec::Object(f)
        | TargetSpec::ObjectOfEachOpponent(f)
        | TargetSpec::OpponentOrObject(f)
        | TargetSpec::ObjectOfFirstTargetsPlayer(f)
        | TargetSpec::ObjectControlledBy(f, _)
        | TargetSpec::ObjectOfEventPlayer(f)
        | TargetSpec::Spell(f)
        | TargetSpec::StackOrBattlefield(f)
        | TargetSpec::CardInGraveyard(f, _)
        | TargetSpec::CardInGraveyardBelowEvent(f, _)
        | TargetSpec::CardInGraveyardBelowValue(f, _, _)
        | TargetSpec::AbilityOnStack(f)
        | TargetSpec::SpellOrAbility(f) => Some(f),
        // A player, the source, or the event's own object: no filter, and
        // nothing an effect could reuse by accident.
        TargetSpec::ThisObject
        | TargetSpec::EventObject
        | TargetSpec::Player(_)
        | TargetSpec::AnyPlayer
        | TargetSpec::AnyOpponent
        | TargetSpec::AnyTarget => None,
    }
}

/// An ability that targets and then does something to **everything its
/// target filter matches**, rather than to the target.
///
/// Returns the reused filter, which is what a failure has to print: the
/// mistake is invisible in the card's text and obvious in the filter's name.
pub(super) fn target_reuse(ability: &AbilityDef) -> Option<&'static Filter> {
    for branch in branches(ability) {
        let Some(wanted) = branch.target.and_then(target_filter) else {
            continue;
        };
        for effect in branch.effects {
            for swept in swept_filters(effect) {
                if swept == wanted {
                    return Some(swept);
                }
            }
        }
    }
    None
}

/// Whether a target spec can name an **object** at all, rather than only a
/// player.
///
/// Exhaustive with no wildcard arm for the reason [`branches`] is: a new
/// [`TargetSpec`] has to be put on one side of this before the crate
/// compiles.
pub(super) fn can_target_an_object(spec: TargetSpec) -> bool {
    match spec {
        TargetSpec::Object(_)
        | TargetSpec::ObjectOfEachOpponent(_)
        | TargetSpec::OpponentOrObject(_)
        | TargetSpec::ObjectOfFirstTargetsPlayer(_)
        | TargetSpec::ObjectControlledBy(..)
        | TargetSpec::ObjectOfEventPlayer(_)
        | TargetSpec::Spell(_)
        | TargetSpec::StackOrBattlefield(_)
        | TargetSpec::CardInGraveyard(..)
        | TargetSpec::CardInGraveyardBelowEvent(..)
        | TargetSpec::CardInGraveyardBelowValue(..)
        | TargetSpec::AbilityOnStack(_)
        | TargetSpec::SpellOrAbility(_)
        | TargetSpec::ThisObject
        | TargetSpec::EventObject
        | TargetSpec::AnyTarget => true,
        TargetSpec::Player(_) | TargetSpec::AnyPlayer | TargetSpec::AnyOpponent => false,
    }
}

/// Whether an effect list reads `PlayerRel::ControllerOfTarget` anywhere.
///
/// Asked of the `Debug` spelling because the relation sits in differently
/// named fields — `target` on a mill, `player` on a graveyard exile, `who`
/// on a life gain — and every one of them resolves through the same
/// `resolve::players_of`. The pattern is the variant's bare name and ends
/// open, so a field added beside it cannot blind the check.
pub(super) fn reads_controller_of_target(effects: &[Effect]) -> bool {
    effects
        .iter()
        .any(|e| format!("{e:?}").contains("ControllerOfTarget"))
}

/// An ability that can only target a **player** and then reads
/// `PlayerRel::ControllerOfTarget`.
///
/// The engine answers that relation from the resolution's first *object*
/// target (`resolve::players_of`), and only an object has a controller
/// (CR 109.4: "Only objects on the stack or on the battlefield have a
/// controller") — `Resolution::targets` holds `ObjectId`s, so a targeted
/// player is carried beside it and never in it. On such a branch the
/// relation names nobody, the effect happens to no one, and nothing says
/// so. "Target player mills four cards" is `PlayerRel::Chosen`: Ashiok,
/// Dream Render's −1 was written with `ControllerOfTarget`, claimed
/// `Coverage::Implemented`, and milled nobody.
///
/// What it does not see: `TargetSpec::AnyTarget` can name a player too, and
/// on that choice the relation names nobody just the same — but it can also
/// name a creature, whose controller is a perfectly good answer, so the
/// shape is not wrong on its face and is left alone. No card in the pool
/// pairs the two.
///
/// Returns the target spec, which is what a failure has to print.
pub(super) fn controller_of_a_player_target(ability: &AbilityDef) -> Option<TargetSpec> {
    for branch in branches(ability) {
        let Some(spec) = branch.target else {
            continue;
        };
        if !can_target_an_object(spec) && reads_controller_of_target(branch.effects) {
            return Some(spec);
        }
    }
    None
}
