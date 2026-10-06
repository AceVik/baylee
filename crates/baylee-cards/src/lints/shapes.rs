//! The shapes an effect must have: divisions, modes, choices, reflexives
//! and mana abilities.

use super::*;

/// How many damage divisions (`Effect::DealDamageDivided`) a list holds,
/// nested ones included.
pub(super) fn divisions_in(effects: &'static [Effect]) -> usize {
    let (mut found, mut seen) = (0_usize, 0_usize);
    Effect::walk(effects, &mut seen, &mut |effect| {
        if matches!(effect, Effect::DealDamageDivided { .. }) {
            found += 1;
        }
    });
    found
}

/// Whether an ability that divides damage sits where the engine asks the
/// division, and how not.
///
/// `Engine::ask_trigger_division` asks it only as a triggered ability is put
/// on the stack, reads it off the ability's own top-level list, and asks one
/// share per target with each at least 1 (CR 601.2d). So the effect is a
/// top-level op of a `Triggered` ability that targets, once, and the ability
/// asks for no more targets than there is damage to give each of them one.
pub(super) fn division_fault(ability: &AbilityDef) -> Option<&'static str> {
    let found: usize = resolving_lists(ability)
        .iter()
        .map(|(effects, _, _)| divisions_in(effects))
        .sum();
    if found == 0 {
        return None;
    }
    let AbilityDef::Triggered {
        effects, targets, ..
    } = ability
    else {
        return Some("divides damage outside a triggered ability, where nothing asks the division");
    };
    let amounts: Vec<u32> = effects
        .iter()
        .filter_map(|effect| match effect {
            Effect::DealDamageDivided { amount } => Some(*amount),
            _ => None,
        })
        .collect();
    if amounts.len() != found {
        return Some("divides damage inside a carrier, where the division is not read");
    }
    if amounts.len() > 1 {
        return Some("divides damage twice in one ability");
    }
    let Some(req) = targets else {
        return Some("divides damage among no targets");
    };
    if req.max == 0 || u32::from(req.max) > amounts[0] {
        return Some("asks for more targets than there is damage to give each one");
    }
    None
}

/// Whether a second instance of the word "target" may ask for this: objects
/// only, because the players a spell targets ride with its first instance
/// alone (`Engine::instance_legality`). Fail-closed: a spec is here once
/// someone has read that it names no player.
pub(super) const fn objects_only(spec: TargetSpec) -> bool {
    matches!(
        spec,
        TargetSpec::Object(_)
            | TargetSpec::Spell(_)
            | TargetSpec::StackOrBattlefield(_)
            | TargetSpec::SpellOrAbility(_)
            | TargetSpec::AbilityOnStack(_)
            | TargetSpec::CardInGraveyard(..)
    )
}

/// Why a modal spell could not be cast the way its card prints it, or
/// [`None`] when it can, or when the ability is no modal spell.
///
/// Choosing one mode is the shape every modal spell had before Farewell: a
/// mode may replace the card's cost (overload), and has no cost of its own
/// to add, because the one-mode path never adds one. Choosing several
/// (CR 700.2d) is `CastModeKind::Modes`, a set of at most eight bits. There
/// a mode's own cost is added to the card's (CR 700.2h) and never put in
/// its place. The engine holds two instances of the word "target" for a
/// spell, and each chosen mode that targets takes the next one
/// (`cast_wizard::targeted_mode`), so at most two modes may target, and the
/// second of them objects only ([`objects_only`]). A mode that says
/// "target" twice itself (`SpellMode::second_targets`, Archdruid's Charm)
/// takes both instances, so only a choose-one spell may print one.
pub(super) fn modes_fault(ability: &AbilityDef) -> Option<&'static str> {
    let AbilityDef::ModalSpell { modes, choose } = ability else {
        return None;
    };
    if choose.min == 0 || choose.min > choose.max {
        return Some("chooses a count of modes that no cast can meet");
    }
    if usize::from(choose.min) > modes.len() {
        return Some("chooses more modes than it prints");
    }
    if choose.is_one() {
        if modes.iter().any(|m| m.additional_cost.is_some()) {
            return Some(
                "gives a mode of a choose-one spell a cost of its own, which nothing adds",
            );
        }
        return None;
    }
    if modes.len() > 8 {
        return Some("prints more modes than a set of them can hold");
    }
    if modes.iter().any(|m| m.cost_override.is_some()) {
        return Some("replaces the card's cost in one of several modes");
    }
    if modes.iter().any(|m| m.second_targets.is_some()) {
        return Some(
            "says \"target\" twice in one of several modes, and each chosen mode takes one instance of the word",
        );
    }
    let targeting: Vec<TargetSpec> = modes
        .iter()
        .filter_map(|m| m.targets.map(|req| req.spec))
        .collect();
    if targeting.len() > 2 {
        return Some(
            "targets in more than two of its modes, and a spell holds two instances of the word",
        );
    }
    if targeting.get(1).is_some_and(|spec| !objects_only(*spec)) {
        return Some("may name a player in its second targeting mode, which holds objects only");
    }
    None
}

/// Whether an effect can happen to a chosen permanent without stopping to
/// ask anything. Fail-closed, like [`cannot_move_the_source`]: an effect
/// is here once someone has read that it resolves without a question.
pub(super) const fn asks_nothing(effect: &Effect) -> bool {
    matches!(effect, Effect::CreateContinuousEffect { .. })
}

/// Why an effect list's "choose a permanent you control, then …" could lose
/// its choice, or [`None`] when it cannot.
///
/// `Effect::ChooseYoursThen` runs `then` as a nested list with the chosen
/// permanent as its object. A nested list that stops for a question hands
/// the rest of itself back to the list around it, which never knew what
/// was chosen, so `then` holds only effects that ask nothing
/// ([`asks_nothing`]) — and at least one, or nothing happens to what was
/// chosen.
pub(super) fn chosen_then_fault(effects: &'static [Effect]) -> Option<&'static str> {
    let (mut fault, mut seen) = (None, 0_usize);
    Effect::walk(effects, &mut seen, &mut |effect| {
        if let Effect::ChooseYoursThen { then, .. } = effect {
            if then.is_empty() {
                fault = Some("chooses a permanent and does nothing to it");
            } else if !then.iter().all(asks_nothing) {
                fault = Some("does something to the chosen permanent that could stop to ask");
            }
        }
    });
    fault
}

/// How many reflexive triggers an effect list writes, at any depth.
pub(super) fn reflexives_in(effects: &'static [Effect]) -> usize {
    let (mut found, mut seen) = (0_usize, 0_usize);
    Effect::walk(effects, &mut seen, &mut |effect| {
        if matches!(effect, Effect::Reflexive { .. }) {
            found += 1;
        }
    });
    found
}

/// Whether an op before the action could move the reflexive ability's
/// source.
///
/// The engine counts **any** departure of the source by effect after the
/// resolution's marker as the action (`resolve::reflexive`). That count is
/// exact only while nothing else in the list can move the source, so this
/// list is fail-closed: an op is allowed only after someone has read it and
/// named it. A card that needs another op here extends the list on purpose.
pub(super) const fn cannot_move_the_source(effect: &Effect) -> bool {
    matches!(effect, Effect::Mill { .. })
}

/// Why an effect list's reflexive trigger could not fire the way its card
/// prints it, or [`None`] when the list writes none or writes it correctly.
///
/// "When you do" names the action printed directly before it, and the
/// engine reads that action back out of what the resolution has done. The
/// shape is therefore one shape: an allowed prefix, the action, and the
/// reflexive trigger as the last op of a list that resolves off the stack.
/// Every other placement is an ability the engine would read wrongly.
/// - Anything after the reflexive would run while the reflexive is still
///   held in the state, which is supposed to be empty whenever a question
///   is out.
/// - An op between the action and the reflexive could move the source a
///   second way.
/// - A reflexive nested in a carrier or in another reflexive is
///   one the placement rule no longer describes.
/// - A mana ability has no resolution to read back through (CR 605.3b).
pub(super) fn reflexive_fault_in(
    effects: &'static [Effect],
    on_the_stack: bool,
) -> Option<&'static str> {
    let found = reflexives_in(effects);
    if found == 0 {
        return None;
    }
    if !on_the_stack {
        return Some(
            "writes a reflexive trigger in a mana ability, which never resolves off the stack",
        );
    }
    let Some((Effect::Reflexive { when, .. }, before)) = effects.split_last() else {
        return Some("writes a reflexive trigger that is not the last op of its ability");
    };
    if found > 1 {
        return Some("writes a reflexive trigger inside another one or inside a carrier");
    }
    let Some((action, prefix)) = before.split_last() else {
        return Some("writes a reflexive trigger with no action before it");
    };
    match when {
        ReflexiveEvent::SacrificedThis => {
            if !matches!(
                action,
                Effect::SacrificeSelf
                    | Effect::MayDo {
                        effects: [Effect::SacrificeSelf]
                    }
            ) {
                return Some("waits for a sacrifice that is not the op directly before it");
            }
        }
        ReflexiveEvent::ExiledThis => {
            if !matches!(
                action,
                Effect::ExileSource
                    | Effect::MayDo {
                        effects: [Effect::ExileSource]
                    }
            ) {
                return Some("waits for an exile that is not the op directly before it");
            }
        }
    }
    if !prefix.iter().all(cannot_move_the_source) {
        return Some("lets an op that could move the source stand before the action");
    }
    None
}

/// The `(flag, cost, effects)` of an activated ability a modifier **grants**.
///
/// [`None`] for every other modifier, so a caller may hand it whatever a
/// walk turned up without asking twice.
pub(super) fn as_granted_activated(
    modifier: &Modifier,
) -> Option<(bool, crate::dsl::Cost, &'static [Effect])> {
    match modifier {
        Modifier::GrantActivated {
            cost,
            effects,
            mana_ability,
            ..
        } => Some((*mana_ability, *cost, effects)),
        _ => None,
    }
}

/// Whether an activated ability's `mana_ability` flag disagrees with what
/// the ability actually does (CR 605.1), and how.
///
/// `None` for an ability that is honest, and for every ability the rule has
/// nothing to say about — a spell, a trigger, a loyalty ability, which
/// CR 605.1a excludes by name and which never reaches the match below.
///
/// The rule is read the way it is written: an activated ability is a mana
/// ability if it **could** add mana, does not require a target, and is not
/// a loyalty ability, and its own cost/effect cannot move library cards.
/// Damage/life riders are permitted; Chromatic Sphere's draw is not.
pub(super) fn mana_ability_fault(ability: &AbilityDef) -> Option<&'static str> {
    match ability {
        AbilityDef::Activated {
            cost,
            mana_ability,
            effects,
            targets,
            second_targets,
            ..
        }
        | AbilityDef::ActivatedConditional {
            cost,
            mana_ability,
            effects,
            targets,
            second_targets,
            ..
        } => mana_ability_fault_of(
            *mana_ability,
            cost,
            effects,
            targets
                .map(|req| req.spec)
                .or(second_targets.map(|req| req.spec)),
        ),
        _ => None,
    }
}

/// Printed and granted abilities obey the same current CR 605.1a criteria.
pub(super) fn mana_ability_fault_of(
    claimed: bool,
    cost: &crate::dsl::Cost,
    effects: &'static [Effect],
    target: Option<TargetSpec>,
) -> Option<&'static str> {
    let makes_any = crate::dsl::mana_rule::could_add_mana(effects);
    if claimed && !makes_any {
        return Some("claims a mana ability that makes no mana");
    }
    if claimed && target.is_some() {
        return Some("claims a mana ability that targets (CR 605.1a)");
    }
    let actual = crate::dsl::mana_rule::activated_mana_ability(cost, effects, target.is_some());
    if claimed && !actual {
        return Some("claims a mana ability whose cost or effect moves a library card (CR 605.1a)");
    }
    if !claimed && actual {
        return Some("meets CR 605.1a but is not marked a mana ability");
    }
    None
}
