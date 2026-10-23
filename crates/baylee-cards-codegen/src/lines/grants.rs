//! Activated costs granted by otherwise unshaped abilities.

use baylee_cards_dsl::AbilityDef;

/// The cost of the activated ability a static or a copy clause grants, when
/// it grants one: the two kinds of ability [`LineShape::Grant`] is for.
///
/// `None` for every other ability, including one that grants through an
/// effect it resolves (a saga chapter, Spawning Pool's `{1}{B}:`), since
/// that one already has a sentence of its own shape.
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
        _ => None,
    }
}
