//! References to a source found through its own public-zone cost (CR 400.7j).
use baylee_cards_dsl::{Effect, Filter, TargetSpec};
pub(super) fn refers_to_subject(effects: &'static [Effect], implicit_source: bool) -> bool {
    let mut refers = false;
    Effect::walk(effects, &mut 0, &mut |effect| {
        refers |= match effect {
            Effect::GrantSpecialActionUntilEndOfTurn {
                effect: baylee_cards_dsl::SpecialActionEffect::PreventNextDamage { target, .. },
                ..
            }
            | Effect::Exile { target, .. }
            | Effect::Blink { target, .. }
            | Effect::RedirectNextFromChosenSource { target, .. }
            | Effect::Destroy { target, .. }
            | Effect::PutOnBottomOfLibraryFromGraveyard { target, .. }
            | Effect::OwnerPutsOnTopOrBottom { target, .. }
            | Effect::ExileIfDiesThisTurn { target, .. }
            | Effect::CantBeRegeneratedThisTurn { target, .. }
            | Effect::ReturnToBattlefieldTapped { target, .. }
            | Effect::ReturnToHand { target, .. }
            | Effect::DestroyOthersNamedLike { target, .. }
            | Effect::Regenerate { target, .. }
            | Effect::GraveyardToTop { target, .. }
            | Effect::GraveyardToHand { target, .. }
            | Effect::GraveyardToBattlefield { target, .. }
            | Effect::AttachSelf { target, .. }
            | Effect::ExileLinked { target, .. }
            | Effect::SacrificeObject { target, .. }
            | Effect::RedirectNextDamage { target, .. }
            | Effect::PreventNextDamage { target, .. }
            | Effect::DealDamage { target, .. }
            | Effect::DealDamageEvenly { target, .. } => matches!(target, TargetSpec::ThisObject),
            Effect::CreateContinuousEffect { filter, .. }
            | Effect::PumpFilter { filter, .. }
            | Effect::SetPTFilter { filter, .. } => {
                implicit_source && matches!(filter, Filter::This)
            }
            Effect::SacrificeSelf
            | Effect::UntapSelf
            | Effect::TapSelf
            | Effect::RemoveCounterSelf { .. }
            | Effect::IfNoCountersOnSelf { .. } => true,
            Effect::AddCounter { .. }
            | Effect::AddCountersUpTo { .. }
            | Effect::AtNextEndStep { .. } => implicit_source,
            Effect::AtEndOfCombat { about, .. } => matches!(about, TargetSpec::ThisObject),
            _ => false,
        };
    });
    refers
}

/// A zone-move trigger may find its own public destination for an instruction
/// that moves that object again (CR 400.7e). This does not change damage LKI.
pub(super) fn moves_subject(effects: &'static [Effect]) -> bool {
    let mut moves = false;
    Effect::walk(effects, &mut 0, &mut |effect| {
        moves |= matches!(
            effect,
            Effect::ReturnToHand {
                target: TargetSpec::ThisObject
            } | Effect::GraveyardToBattlefield {
                target: TargetSpec::ThisObject,
                ..
            } | Effect::GraveyardToHand {
                target: TargetSpec::ThisObject
            } | Effect::GraveyardToTop {
                target: TargetSpec::ThisObject
            } | Effect::ReturnToBattlefieldTapped {
                target: TargetSpec::ThisObject
            } | Effect::PutOnBottomOfLibraryFromGraveyard {
                target: TargetSpec::ThisObject
            }
        );
    });
    moves
}
