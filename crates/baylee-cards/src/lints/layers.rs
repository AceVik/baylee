//! Layers, and the doors an ability is granted through.

use super::*;

/// Every `(layer, modifier)` pair a continuous effect inside this effect
/// states, the wrappers included.
///
/// The recursion is the one [`swept_filters`] does and for the same reason —
/// a continuous effect inside a conditional is still a continuous effect,
/// and the pool has exactly one of those (Jin-Gitaxias's kicked half). Both
/// ask [`Effect::branches`] which effects those are, rather than listing
/// them: the pair this lint is about appears in one variant, and every other
/// variant is either a carrier or has nothing to say, so the one arm covers
/// both cases.
pub(super) fn declared_layers(effect: &Effect) -> Vec<(Layer, Modifier)> {
    // The early return is exact rather than a shortcut: a continuous effect
    // carries no branches, so there is nothing under it to walk.
    if let Effect::CreateContinuousEffect {
        layer, modifier, ..
    } = effect
    {
        return vec![(*layer, *modifier)];
    }
    let (then, otherwise) = effect.branches();
    then.iter()
        .chain(otherwise)
        .flat_map(declared_layers)
        .collect()
}

/// A continuous effect declared on a layer its modifier does not belong to
/// (CR 613.1).
///
/// Returns `(declared, derived, modifier)`, which is what a failure has to
/// print: the modifier names the effect, and the two layers say which way
/// round the disagreement is.
///
/// This is the lint that lets [`crate::dsl::static_ability!`] take two
/// arguments. The layer is a function of the modifier — "all permanents are
/// artifacts" is layer 4 whatever a card says — so
/// [`Modifier::layer`](crate::dsl::Modifier::layer) is the one answer and a
/// hand-written literal is free to disagree with it. Measured over the whole
/// pool before the derivation existed: 108 pairings, 25 modifiers, no
/// modifier on two layers. That is what makes the table trustworthy and this
/// sweep is what keeps it so, including for the raw literals that stay raw.
pub(super) fn layer_fault(ability: &AbilityDef) -> Option<(Layer, Layer, Modifier)> {
    let from_static = match ability {
        AbilityDef::Static(sa) => vec![(sa.layer, sa.modifier)],
        _ => Vec::new(),
    };
    from_static
        .into_iter()
        .chain(
            branches(ability)
                .into_iter()
                .flat_map(|branch| branch.effects.iter().flat_map(declared_layers)),
        )
        .find(|(declared, modifier)| *declared != modifier.layer())
        .map(|(declared, modifier)| (declared, modifier.layer(), modifier))
}

/// Where an effect list an ability resolves through came from.
///
/// A reflexive trigger can arrive through any door an effect list can, and a
/// sweep that knows one of them reports a clean pool having read part of
/// it. That is how the CR 605.1 sweeps were found blind to grants. The
/// discriminant indexes the per-door counts a sweep holds its floor
/// against.
#[derive(Clone, Copy, Debug)]
pub(super) enum Door {
    /// Printed on the card or token.
    Printed = 0,
    /// Granted by an [`AbilityDef::Static`].
    Static = 1,
    /// Granted by an [`Effect::CreateContinuousEffect`] as a list resolves.
    Effect = 2,
    /// Granted by a [`CopyMod::Grant`](crate::dsl::ability::CopyMod::Grant)
    /// inside a copy clause.
    Copy = 3,
}

/// The `(effects, goes on the stack)` of an ability a modifier grants.
pub(super) fn granted_list(modifier: &Modifier) -> Option<(&'static [Effect], bool)> {
    match modifier {
        Modifier::GrantActivated {
            effects,
            mana_ability,
            ..
        } => Some((effects, !*mana_ability)),
        Modifier::GrantTriggered { effects, .. } => Some((effects, true)),
        Modifier::GrantStatic { modifier, .. } => granted_list(modifier),
        _ => None,
    }
}

/// Every effect list an ability can resolve through, whether it resolves
/// off the stack, and the door it came through.
///
/// A list is on the stack unless it is a mana ability, printed or granted
/// (CR 605.3b). That is the one question [`reflexive_fault_in`] needs
/// answered about where a list came from. The `match` is exhaustive with no
/// wildcard for the reason [`branches`] gives.
pub(super) fn resolving_lists(ability: &AbilityDef) -> Vec<(&'static [Effect], bool, Door)> {
    use crate::dsl::ability::CopyMod;
    let mut lists: Vec<(&'static [Effect], bool, Door)> = Vec::new();
    match ability {
        AbilityDef::Spell { effects, .. }
        | AbilityDef::Triggered { effects, .. }
        | AbilityDef::Loyalty { effects, .. }
        | AbilityDef::SagaChapter { effects, .. } => lists.push((effects, true, Door::Printed)),
        AbilityDef::Activated {
            effects,
            mana_ability,
            ..
        }
        | AbilityDef::ActivatedConditional {
            effects,
            mana_ability,
            ..
        } => lists.push((effects, !*mana_ability, Door::Printed)),
        AbilityDef::ModalSpell { modes, .. } | AbilityDef::ModalTriggered { modes, .. } => {
            lists.extend(modes.iter().map(|m| (m.effects, true, Door::Printed)));
        }
        AbilityDef::Static(rule) => {
            if let Some((effects, stack)) = granted_list(&rule.modifier) {
                lists.push((effects, stack, Door::Static));
            }
        }
        AbilityDef::CopyOnEnter { mods, .. } | AbilityDef::CopyOnEnterUntilEot { mods, .. } => {
            for m in *mods {
                match m {
                    CopyMod::Grant(modifier) => {
                        if let Some((effects, stack)) = granted_list(modifier) {
                            lists.push((effects, stack, Door::Copy));
                        }
                    }
                    CopyMod::GrantAbility(granted) => {
                        lists.extend(
                            resolving_lists(granted)
                                .into_iter()
                                .map(|(effects, stack, _)| (effects, stack, Door::Copy)),
                        );
                    }
                    _ => {}
                }
            }
        }
        AbilityDef::Unimplemented
        | AbilityDef::Ward { .. }
        | AbilityDef::Toxic { .. }
        | AbilityDef::Prepared { .. }
        | AbilityDef::Echo { .. }
        | AbilityDef::Replacement(_)
        | AbilityDef::Suspend { .. } => {}
    }
    // The effect door, read off every list already found, including the
    // ones this loop adds: a granted ability may itself grant one.
    let mut i = 0;
    while i < lists.len() {
        let (effects, ..) = lists[i];
        let mut seen = 0_usize;
        Effect::walk(effects, &mut seen, &mut |effect| {
            if let Effect::CreateContinuousEffect { modifier, .. } = effect
                && let Some((granted, stack)) = granted_list(modifier)
            {
                lists.push((granted, stack, Door::Effect));
            }
        });
        i += 1;
    }
    lists
}

/// Copiable exception abilities, including those within resolving copy effects.
pub(super) fn copiable_grants(ability: &AbilityDef) -> Vec<&'static AbilityDef> {
    use crate::dsl::ability::CopyMod;
    let mut found = Vec::new();
    let mut read = |mods: &'static [CopyMod]| {
        found.extend(mods.iter().filter_map(|m| match m {
            CopyMod::GrantAbility(granted) => Some(*granted),
            _ => None,
        }));
    };
    if let AbilityDef::CopyOnEnter { mods, .. } | AbilityDef::CopyOnEnterUntilEot { mods, .. } =
        ability
    {
        read(mods);
    }
    for (effects, ..) in resolving_lists(ability) {
        let mut seen = 0;
        Effect::walk(effects, &mut seen, &mut |effect| match effect {
            Effect::CreateTokenCopyOfTarget { mods, .. }
            | Effect::CreateTokenCopyOfSource { mods }
            | Effect::CreateTokenCopyOfEquipped { mods, .. }
            | Effect::CopyTargetSpell { mods }
            | Effect::BecomeCopyOfTarget { mods } => read(mods),
            _ => {}
        });
    }
    found
}
