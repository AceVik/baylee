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
            Effect::AttachSlot { object, to, .. } => {
                matches!(
                    (object, to),
                    (baylee_cards_dsl::TargetSlot::This, _)
                        | (_, baylee_cards_dsl::TargetSlot::This)
                )
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
            | Effect::MoveAttachment {
                attachment: target, ..
            }
            | Effect::AttachChosen { to: target, .. }
            | Effect::AttachChosenThen { to: target, .. }
            | Effect::BecomeAura { target, .. }
            | Effect::SetDynamicPT {
                subject: target, ..
            }
            | Effect::Unattach { target }
            | Effect::AttachObject { object: target, .. }
            | Effect::MoveAllAttachments { from: target, .. }
            | Effect::AttachLtbAttachments { of: target, .. }
            | Effect::RelocateAttachment {
                attachment: baylee_cards_dsl::AttachmentSource::Spec(target),
                ..
            }
            | Effect::PutOntoBattlefieldAttacking {
                subject: baylee_cards_dsl::DefinedObjects::Spec(target),
                ..
            }
            | Effect::DiscardDefined {
                cards: baylee_cards_dsl::DefinedObjects::Spec(target),
            }
            | Effect::CreateTokenAttached {
                host: baylee_cards_dsl::AttachHost::Spec(target),
                ..
            }
            | Effect::ExileLinked { target, .. }
            | Effect::SacrificeObject { target, .. }
            | Effect::RedirectNextDamage { target, .. }
            | Effect::PreventNextDamage { target, .. }
            | Effect::DealDamage { target, .. }
            | Effect::DamagePermanentAndController { target, .. }
            | Effect::DealDamageWithoutPreventionOrRedirection { target, .. }
            | Effect::DealDamageWithoutPrevention { target, .. }
            | Effect::DealDamageEvenly { target, .. }
            | Effect::Detain { target }
            | Effect::Goad { target }
            | Effect::Suspect { target }
            | Effect::Unsuspect { target }
            | Effect::AddTempModifier { target, .. }
            | Effect::GainControlEx { target, .. }
            | Effect::PutEachCounterKindFromSource { target }
            | Effect::Play {
                source: baylee_cards_dsl::PlaySource::Subject(target),
                ..
            } => matches!(target, TargetSpec::ThisObject),
            Effect::PutAllCountersFrom { donor, target } => {
                matches!(donor, baylee_cards_dsl::CounterDonor::Source)
                    || matches!(target, TargetSpec::ThisObject)
            }
            Effect::PutAllCountersFromIntoFound { donor } => {
                matches!(donor, baylee_cards_dsl::CounterDonor::Source)
            }
            Effect::PreventNextDamageWithThen {
                sources,
                recipients,
                ..
            } => {
                matches!(sources, TargetSpec::ThisObject)
                    || matches!(recipients, TargetSpec::ThisObject)
            }
            Effect::MoveAndModify {
                subject: baylee_cards_dsl::MoveSubject::Target(target),
                attach_source,
                when_source_leaves,
                ..
            } => {
                matches!(target, TargetSpec::ThisObject)
                    || *attach_source
                    || when_source_leaves.is_some()
            }
            Effect::MoveAndModify {
                subject: baylee_cards_dsl::MoveSubject::AllInZone { zone, .. },
                attach_source,
                when_source_leaves,
                ..
            } => {
                matches!(zone, baylee_cards_dsl::ZoneSel::LinkedExile)
                    || *attach_source
                    || when_source_leaves.is_some()
            }
            Effect::PutOntoBattlefieldAttached { subject, host, .. } => {
                matches!(
                    subject,
                    baylee_cards_dsl::DefinedObjects::Spec(TargetSpec::ThisObject)
                        | baylee_cards_dsl::DefinedObjects::AllInZone {
                            zone: baylee_cards_dsl::ZoneSel::LinkedExile,
                            ..
                        }
                ) || matches!(
                    host,
                    baylee_cards_dsl::AttachHost::Spec(TargetSpec::ThisObject)
                )
            }
            Effect::CreateContinuousEffect { filter, .. }
            | Effect::AddGlobalTempModifier { filter, .. }
            | Effect::GainControlAllEx { filter, .. }
            | Effect::BecomeChosenCreatureType { filter, .. }
            | Effect::GrantProtectionFromChosenColorUntilEot { filter }
            | Effect::DetainFilter { filter }
            | Effect::PumpFilter { filter, .. }
            | Effect::PumpFilterPerKeyword { filter, .. }
            | Effect::SetPTFilter { filter, .. }
            | Effect::SetPTByCountFilter { filter, .. } => {
                implicit_source && matches!(filter, Filter::This)
            }
            Effect::DoublePT { subject, .. } => match subject {
                baylee_cards_dsl::DoublePtSubject::Source => true,
                baylee_cards_dsl::DoublePtSubject::Filter(filter) => {
                    implicit_source && matches!(filter, Filter::This)
                }
                _ => false,
            },
            Effect::RedirectCombatDamageToYouThisTurn { to, .. } => matches!(
                to,
                baylee_cards_dsl::DamageRedirectTo::ThisObject
                    | baylee_cards_dsl::DamageRedirectTo::Target(TargetSpec::ThisObject)
            ),
            Effect::PutOntoBattlefieldAttacking {
                subject:
                    baylee_cards_dsl::DefinedObjects::AllInZone {
                        zone: baylee_cards_dsl::ZoneSel::LinkedExile,
                        ..
                    },
                ..
            }
            | Effect::DiscardDefined {
                cards:
                    baylee_cards_dsl::DefinedObjects::AllInZone {
                        zone: baylee_cards_dsl::ZoneSel::LinkedExile,
                        ..
                    },
            }
            | Effect::ChooseCardConstrained {
                zone: baylee_cards_dsl::ChooseZone::LinkedExile,
                ..
            }
            | Effect::ChooseCardConstrained {
                movement: baylee_cards_dsl::ChooseMove::ExileLinked,
                ..
            }
            | Effect::PreventNextDamageFromSource { .. }
            | Effect::Play {
                source: baylee_cards_dsl::PlaySource::LinkedExile,
                ..
            }
            | Effect::RevealUntilThen {
                found: baylee_cards_dsl::RevealUntilFound::ExileLinked,
                ..
            }
            | Effect::RepeatEachObject {
                zone: baylee_cards_dsl::ZoneSel::LinkedExile,
                ..
            }
            | Effect::SeparateIntoTwoPiles {
                zone: baylee_cards_dsl::ZoneSel::LinkedExile,
                ..
            }
            | Effect::ChooseCard {
                zone: baylee_cards_dsl::ChooseZone::LinkedExile,
                ..
            }
            | Effect::ChooseCard {
                movement: baylee_cards_dsl::ChooseMove::ExileLinked,
                ..
            }
            | Effect::ChooseCardEx {
                zone: baylee_cards_dsl::ChooseZone::LinkedExile,
                ..
            }
            | Effect::ChooseCardEx {
                movement: baylee_cards_dsl::ChooseMove::ExileLinked,
                ..
            }
            | Effect::ChooseCardEx {
                not_chosen_movement: baylee_cards_dsl::ChooseMove::ExileLinked,
                ..
            }
            | Effect::PreventNextDamageToAttached { .. }
            | Effect::ReflectNextChosenSourceDamage
            | Effect::MustBlockSourceIfAbleThisCombat
            | Effect::ExileSource
            | Effect::ExileSourceWithCounters { .. }
            | Effect::ExileSourceThenCastTransformed { .. }
            | Effect::TakklemaggotDeath
            | Effect::MeldWithNamed { .. }
            | Effect::MeldPair { .. }
            | Effect::SetState {
                subject:
                    baylee_cards_dsl::StateSubject::Source | baylee_cards_dsl::StateSubject::LinkedExile,
                ..
            }
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
            | Effect::PutCountersDivided { .. }
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
            | Effect::PreventFromChosenSourceWithLifeCondition { target, .. }
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
        | C::ChosenMode(_)
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
                | Effect::ExileSource
                | Effect::ExileSourceWithCounters { .. }
                | Effect::MeldWithNamed { .. }
                | Effect::MeldPair { .. }
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
                | Effect::MoveAndModify {
                    subject: baylee_cards_dsl::MoveSubject::Target(TargetSpec::ThisObject),
                    ..
                }
                | Effect::PutOntoBattlefieldAttached {
                    subject: baylee_cards_dsl::DefinedObjects::Spec(TargetSpec::ThisObject),
                    ..
                }
                | Effect::PutOntoBattlefieldAttacking {
                    subject: baylee_cards_dsl::DefinedObjects::Spec(TargetSpec::ThisObject),
                    ..
                }
                | Effect::DiscardDefined {
                    cards: baylee_cards_dsl::DefinedObjects::Spec(TargetSpec::ThisObject),
                }
        );
    });
    moves
}
