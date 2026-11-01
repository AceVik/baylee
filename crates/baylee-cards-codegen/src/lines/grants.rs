//! Activated costs granted by otherwise unshaped abilities.

use baylee_cards_dsl::AbilityDef;

/// The activated cost granted by a static, copy clause or spell body.
/// These otherwise unshaped abilities use [`super::LineShape::Grant`].
///
/// Abilities with an existing shape (activated, triggered, saga) retain their
/// own sentence even when their resolving effects also grant an ability.
pub(super) fn granted_cost(ability: &AbilityDef) -> Option<baylee_cards_dsl::Cost> {
    use baylee_cards_dsl::{CopyMod, Modifier};
    let cost = |modifier: &Modifier| match modifier {
        Modifier::GrantActivated { cost, .. } => Some(*cost),
        _ => None,
    };
    match ability {
        AbilityDef::Static(rule) => cost(&rule.modifier),
        AbilityDef::CopyOnEnter { mods, .. } | AbilityDef::CopyOnEnterUntilEot { mods, .. } => {
            mods.iter().find_map(|m| match m {
                CopyMod::Grant(modifier) => cost(modifier),
                CopyMod::GrantAbility(
                    AbilityDef::Activated { cost, .. }
                    | AbilityDef::ActivatedConditional { cost, .. }
                    | AbilityDef::ModalActivated { cost, .. },
                ) => Some(*cost),
                _ => None,
            })
        }
        AbilityDef::Spell { effects, .. } => {
            let mut found = None;
            baylee_cards_dsl::effect::Effect::walk(effects, &mut 0, &mut |effect| {
                if found.is_none()
                    && let baylee_cards_dsl::effect::Effect::CreateContinuousEffect {
                        modifier, ..
                    } = effect
                {
                    found = cost(modifier);
                }
            });
            found
        }
        _ => None,
    }
}
