//! Combat-only redirection to an exact permanent incarnation.
use super::{DamageEffectKind, DamageTarget, GameState, Part};
use crate::prevention::{RedirectDestination, Shield, ShieldKind, damageable_recipient};

pub(super) fn shield_kind(
    state: &GameState,
    part: &Part,
    shield: Shield,
) -> Option<DamageEffectKind> {
    let ShieldKind::RedirectCombat {
        sources,
        to,
        context,
    } = shield.kind
    else {
        return None;
    };
    if !part.view.is_combat {
        return None;
    }
    let actual = state.damage_source(part.view.source, part.source_version)?;
    let to = match to {
        RedirectDestination::Object(reference) => {
            let destination = state.object(reference.object)?;
            if destination.version != reference.version
                || destination
                    .status
                    .contains(crate::object::Status::PHASED_OUT)
            {
                return None;
            }
            DamageTarget::Object(reference.object)
        }
        RedirectDestination::SourceController => DamageTarget::Player(actual.controller),
    };
    if !damageable_recipient(state, to) || part.view.recipient == to {
        return None;
    }
    if !crate::eval::matches_projected_with_context(
        sources,
        state,
        actual,
        actual.characteristics(),
        shield.controller,
        context,
    ) {
        return None;
    }
    Some(DamageEffectKind::Redirect { to })
}
