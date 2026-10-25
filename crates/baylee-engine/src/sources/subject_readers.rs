//! References to a source found through its own public-zone cost (CR 400.7j).
use baylee_cards_dsl::{Effect, Filter, TargetSpec};
pub(super) fn refers_to_subject(effects: &'static [Effect], implicit_source: bool) -> bool {
    let mut refers = false;
    Effect::walk(effects, &mut 0, &mut |effect| {
        refers |= match effect {
            Effect::DestroyTargetSlots { targets, .. } => {
                targets.contains(&baylee_cards_dsl::TargetSlot::This)
            }
            Effect::RedirectTargetSourceDamageThisTurn { source, to, .. } => {
                matches!(source, TargetSpec::ThisObject)
                    || matches!(to, baylee_cards_dsl::DamageRedirectTo::ThisObject)
                    || matches!(
                        to,
                        baylee_cards_dsl::DamageRedirectTo::Target(TargetSpec::ThisObject)
                    )
            }
            Effect::ChooseLandwalk { target, .. }
            | Effect::RedirectNextChosenSourceDamageToTarget { target }
            | Effect::PreventDamageFromTargetingCausesThisTurn { target }
            | Effect::GrantSpecialActionUntilEndOfTurn {
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
            | Effect::GraveyardToBattlefieldLinked { target }
            | Effect::GraveyardToBattlefield { target, .. }
            | Effect::AttachSelf { target, .. }
            | Effect::ExileLinked { target, .. }
            | Effect::SacrificeObject { target, .. }
            | Effect::RedirectNextDamage { target, .. }
            | Effect::PreventNextDamage { target, .. }
            | Effect::DealDamage { target, .. }
            | Effect::DealDamageWithoutPreventionOrRedirection { target, .. }
            | Effect::DealDamageEvenly { target, .. } => matches!(target, TargetSpec::ThisObject),
            Effect::CreateContinuousEffect { filter, .. }
            | Effect::PumpFilter { filter, .. }
            | Effect::SetPTFilter { filter, .. } => {
                implicit_source && matches!(filter, Filter::This)
            }
            Effect::RedirectCombatDamageToYouThisTurn { to, .. } => matches!(
                to,
                baylee_cards_dsl::DamageRedirectTo::ThisObject
                    | baylee_cards_dsl::DamageRedirectTo::Target(TargetSpec::ThisObject)
            ),
            Effect::PreventNextDamageFromSource { .. }
            | Effect::PreventNextDamageToAttached { .. }
            | Effect::ReflectNextChosenSourceDamage
            | Effect::ExileSourceThenCastTransformed { .. }
            | Effect::TakklemaggotDeath
            | Effect::MeldWithNamed { .. }
            | Effect::MoveSourceToOwnerGraveyard
            | Effect::PutSourceFromHandOntoBattlefield
            | Effect::SacrificeSelf
            | Effect::UntapSelf
            | Effect::TapSelf
            | Effect::ReturnSelfDuringNextUntap
            | Effect::RemoveCounterSelf { .. }
            | Effect::MoveCounterToEvent { .. }
            | Effect::DetachSource
            | Effect::IfNoCountersOnSelf { .. } => true,
            Effect::AddCounter { .. }
            | Effect::AddCountersUpTo { .. }
            | Effect::AtNextEndStep { .. }
            | Effect::AtNextCleanup { .. } => implicit_source,
            Effect::AtEndOfCombat { about, .. } | Effect::AtEndOfCombatThisTurn { about, .. } => {
                matches!(about, TargetSpec::ThisObject)
            }
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
            Effect::TakklemaggotDeath
                | Effect::MeldWithNamed { .. }
                | Effect::ReturnToHand {
                    target: TargetSpec::ThisObject
                }
                | Effect::GraveyardToBattlefieldLinked {
                    target: TargetSpec::ThisObject,
                }
                | Effect::GraveyardToBattlefield {
                    target: TargetSpec::ThisObject,
                    ..
                }
                | Effect::GraveyardToHand {
                    target: TargetSpec::ThisObject
                }
                | Effect::GraveyardToTop {
                    target: TargetSpec::ThisObject
                }
                | Effect::ReturnToBattlefieldTapped {
                    target: TargetSpec::ThisObject
                }
                | Effect::PutOnBottomOfLibraryFromGraveyard {
                    target: TargetSpec::ThisObject
                }
        );
    });
    moves
}
