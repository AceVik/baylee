//! References to a source found through its own public-zone cost (CR 400.7j).
use baylee_cards_dsl::{Effect, Filter, TargetSpec};
#[allow(clippy::too_many_lines)] // Flat vocabulary table matching each subject-bearing variant.
pub(super) fn refers_to_subject(effects: &'static [Effect], implicit_source: bool) -> bool {
    let mut refers = false;
    Effect::walk(effects, &mut 0, &mut |effect| {
        refers |= match effect {
            Effect::DestroyTargetSlots { targets, .. } => {
                targets.contains(&baylee_cards_dsl::TargetSlot::This)
            }
            Effect::TapTargetSlot(slot) | Effect::UntapTargetSlot(slot) => {
                matches!(slot, baylee_cards_dsl::TargetSlot::This)
            }
            Effect::BecomeCopy {
                subject,
                subject_filter,
                original,
                ..
            } => {
                matches!(subject, baylee_cards_dsl::TargetSlot::This)
                    || matches!(original, TargetSpec::ThisObject)
                    || subject_filter.is_some_and(|f| implicit_source && matches!(f, Filter::This))
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
            | Effect::CreateTokenCopy { target, .. }
            | Effect::CopySpellOrAbility { target, .. }
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
            | Effect::GraveyardToBattlefieldModified { target, .. }
            | Effect::AttachSelf { target, .. }
            | Effect::ExileLinked { target, .. }
            | Effect::SacrificeObject { target, .. }
            | Effect::RedirectNextDamage { target, .. }
            | Effect::PreventNextDamage { target, .. }
            | Effect::DealDamage { target, .. }
            | Effect::DamagePermanentAndController { target, .. }
            | Effect::DealDamageWithoutPreventionOrRedirection { target, .. }
            | Effect::DealDamageEvenly { target, .. }
            | Effect::Detain { target }
            | Effect::Goad { target }
            | Effect::Suspect { target }
            | Effect::Unsuspect { target } => matches!(target, TargetSpec::ThisObject),
            Effect::CreateContinuousEffect { filter, .. }
            | Effect::BecomeChosenCreatureType { filter, .. }
            | Effect::GrantProtectionFromChosenColorUntilEot { filter }
            | Effect::DetainFilter { filter }
            | Effect::PumpFilter { filter, .. }
            | Effect::PumpFilterPerKeyword { filter, .. }
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
            | Effect::MustBlockSourceIfAbleThisCombat
            | Effect::ExileSourceThenCastTransformed { .. }
            | Effect::TakklemaggotDeath
            | Effect::MeldWithNamed { .. }
            | Effect::MoveSourceToOwnerGraveyard
            | Effect::PutSourceFromHandOntoBattlefield
            | Effect::PutNinjutsuOntoBattlefield
            | Effect::Myriad
            | Effect::ExertSource
            | Effect::Exploit
            | Effect::Endure(_)
            | Effect::ReturnCraftedTransformed
            | Effect::ReturnSourceFromGraveyard { .. }
            | Effect::SacrificeSelf
            | Effect::UntapSelf
            | Effect::TapSelf
            | Effect::ReturnSelfDuringNextUntap
            | Effect::RemoveCounterSelf { .. }
            | Effect::MoveCounterToEvent { .. }
            | Effect::DetachSource
            | Effect::SoulbondPair { .. }
            | Effect::Monstrosity(_)
            | Effect::SetClassLevel(_)
            | Effect::IfNoCountersOnSelf { .. } => true,
            Effect::AddCounter { .. }
            | Effect::AddCountersUpTo { .. }
            | Effect::AtNextEndStep { .. }
            | Effect::AtNextCleanup { .. } => implicit_source,
            Effect::AtEndOfCombat { about, .. } | Effect::AtEndOfCombatThisTurn { about, .. } => {
                matches!(about, TargetSpec::ThisObject)
            }
            Effect::MustBlockIfAble {
                blocker, attacker, ..
            } => {
                matches!(blocker, TargetSpec::ThisObject)
                    || matches!(attacker, TargetSpec::ThisObject)
            }
            Effect::PreventFromChosenSource { target, .. }
            | Effect::PreventNextFromMatchingSources { target, .. }
            | Effect::PreventAllDamageFromMatchingThisTurn { target, .. } => {
                matches!(target, Some(TargetSpec::ThisObject))
            }
            Effect::IfCondition { condition, .. } | Effect::RepeatWhile { condition, .. } => {
                condition_refers_to_subject(condition)
            }
            Effect::RepeatProcess {
                pre_condition,
                post_condition,
                ..
            } => {
                pre_condition
                    .as_ref()
                    .is_some_and(condition_refers_to_subject)
                    || post_condition
                        .as_ref()
                        .is_some_and(condition_refers_to_subject)
            }
            _ => false,
        };
    });
    refers
}

fn condition_refers_to_subject(condition: &baylee_cards_dsl::Condition) -> bool {
    use baylee_cards_dsl::Condition as C;
    match condition {
        C::ObjectMatches { what, .. } => matches!(*what, TargetSpec::ThisObject),
        C::SourceMatches(_)
        | C::IsPaired
        | C::IsMonstrous
        | C::ClassLevelExactly(_)
        | C::ClassLevelAtLeast(_)
        | C::IsSuspected
        | C::SoulbondSelfCanPair
        | C::SoulbondEventCanPair => true,
        C::All(parts) | C::Any(parts) => parts.iter().any(condition_refers_to_subject),
        C::Not(inner) => condition_refers_to_subject(inner),
        _ => false,
    }
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
                | Effect::ReturnSourceFromGraveyard { .. }
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
                | Effect::GraveyardToBattlefieldModified {
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
