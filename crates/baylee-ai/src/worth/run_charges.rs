//! Renewable run equipment has no persistent charge opportunity cost. Ordinary
//! cards and finite potion supplies retain their existing valuation.
use super::{AbilityDef, Amount, CostPart, CounterKind, Effect, Origin, PlayerView, printed_list};

pub(super) fn renewable(view: &PlayerView, origin: Origin<'_>, part: &CostPart) -> bool {
    let CostPart::RemoveCounterSelf {
        kind: CounterKind::Charge,
        n: 1..,
    } = part
    else {
        return false;
    };
    let Some(object) = origin.object else {
        return false;
    };
    object.types.is_empty()
        && object.token.is_some()
        && view.command.iter().flatten().any(|entry| entry.id == object.id)
        && printed_list(object).iter().any(|ability| {
            matches!(ability, AbilityDef::Triggered { effects, .. } if effects.iter().any(|effect| {
                matches!(effect, Effect::AddCounter { kind: CounterKind::Charge, amount: Amount::Fixed(1..) })
            }))
        })
}
