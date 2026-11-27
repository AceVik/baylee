//! Semantic event-object readers for damage-source eligibility.
//! Merely remembering a triggering event does not make its context a source
//! referred to by the instructions on the stack (CR 609.7a).
use baylee_cards_dsl::{Amount, Effect, PlayerRel, TargetSpec};

pub(super) fn refers_to_event(effects: &'static [Effect]) -> bool {
    let mut refers = false;
    Effect::walk(effects, &mut 0, &mut |effect| refers |= reads_event(effect));
    refers
}
fn reads_player(value: PlayerRel) -> bool {
    matches!(value, PlayerRel::ControllerOfEvent)
}
fn reads_filter(filter: &baylee_cards_dsl::Filter) -> bool {
    use baylee_cards_dsl::Filter as F;
    match filter {
        F::ControlledBy(who) => reads_player(*who),
        F::And(parts) | F::Or(parts) => parts.iter().any(reads_filter),
        F::Not(inner)
        | F::AttachedTo(inner)
        | F::BlockedThisTurnBy(inner)
        | F::LowestManaValueOnBattlefield(inner)
        | F::SharesAColorWith(inner)
        | F::SharesColorWithMostCommon(inner)
        | F::SharesCreatureTypeWith(inner)
        | F::ControllerControls(inner)
        | F::TargetsYouControl(inner) => reads_filter(inner),
        _ => false,
    }
}
fn reads_choose_zone(zone: baylee_cards_dsl::ChooseZone) -> bool {
    match zone {
        baylee_cards_dsl::ChooseZone::Graveyard(p)
        | baylee_cards_dsl::ChooseZone::Hand(p)
        | baylee_cards_dsl::ChooseZone::Exile(p) => reads_player(p),
        baylee_cards_dsl::ChooseZone::Battlefield
        | baylee_cards_dsl::ChooseZone::LinkedExile
        | baylee_cards_dsl::ChooseZone::Found
        | baylee_cards_dsl::ChooseZone::Remembered => false,
    }
}
fn reads_target(value: TargetSpec) -> bool {
    match value {
        TargetSpec::EventObject | TargetSpec::CardInGraveyardBelowEvent(..) => true,
        TargetSpec::Player(who)
        | TargetSpec::CardInGraveyard(_, who)
        | TargetSpec::CardInGraveyardBelowValue(_, who, _) => reads_player(who),
        _ => false,
    }
}
fn reads_defined_objects(value: baylee_cards_dsl::DefinedObjects) -> bool {
    match value {
        baylee_cards_dsl::DefinedObjects::Spec(spec) => reads_target(spec),
        baylee_cards_dsl::DefinedObjects::Found => false,
        baylee_cards_dsl::DefinedObjects::AllInZone { zone, filter } => {
            reads_filter(filter)
                || matches!(
                    zone,
                    baylee_cards_dsl::ZoneSel::GraveyardOf(who)
                        | baylee_cards_dsl::ZoneSel::HandOf(who)
                        | baylee_cards_dsl::ZoneSel::LibraryOf(who)
                        if reads_player(who)
                )
        }
    }
}
fn reads_attach_host(value: baylee_cards_dsl::AttachHost) -> bool {
    match value {
        baylee_cards_dsl::AttachHost::Spec(spec) => reads_target(spec),
        baylee_cards_dsl::AttachHost::ChosenPermanent(filter) => reads_filter(filter),
        baylee_cards_dsl::AttachHost::ChosenPlayer(who) => reads_player(who),
    }
}
fn reads_amount(value: Amount) -> bool {
    match value {
        Amount::EventLastToughness | Amount::EventPower | Amount::EventManaSpentToCast => true,
        Amount::Sum { left, right } | Amount::Difference { left, right } => {
            reads_amount(*left) || reads_amount(*right)
        }
        Amount::Players(who)
        | Amount::LibrarySize(who)
        | Amount::LifeTotal(who)
        | Amount::StartingLifeTotal(who)
        | Amount::LifeGainedThisTurn(who)
        | Amount::LifeLostThisTurn(who)
        | Amount::HighestLifeLostThisTurn(who)
        | Amount::CardsDiscardedThisTurn(who)
        | Amount::DistinctCardTypesCastThisTurn(who)
        | Amount::DistinctSacrificedCardTypesThisTurn(who)
        | Amount::EnergyRemovedThisTurn(who)
        | Amount::CountOf {
            zone:
                baylee_cards_dsl::ZoneSel::GraveyardOf(who)
                | baylee_cards_dsl::ZoneSel::HandOf(who)
                | baylee_cards_dsl::ZoneSel::LibraryOf(who),
            ..
        }
        | Amount::CommanderCasts(who)
        | Amount::HighestCardsDrawnThisTurn(who) => reads_player(who),
        Amount::EnteredThisTurn {
            filter,
            owner_or_ctrl: who,
            ..
        }
        | Amount::TokensCreatedThisTurn {
            filter,
            creator: who,
        }
        | Amount::LeftZoneThisTurn {
            filter,
            owner_or_ctrl: who,
            ..
        }
        | Amount::PermanentsSacrificedThisTurn { filter, who }
        | Amount::SpellsCastMatchingThisTurn {
            filter,
            caster: who,
        }
        | Amount::CreaturesAttackedThisTurn {
            filter,
            controller: who,
        }
        | Amount::HighestControlledCount { filter, among: who } => {
            reads_player(who) || reads_filter(filter)
        }
        Amount::DistinctPowersAmong(filter) => reads_filter(filter),
        Amount::DistinctManaValuesInZone { zone, filter }
        | Amount::DistinctNames { zone, filter }
        | Amount::TotalManaValueInZone { zone, filter }
        | Amount::DistinctExactManaCostsInZone { zone, filter }
        | Amount::DistinctCardTypesInZone { zone, filter } => {
            reads_filter(filter)
                || matches!(
                    zone,
                    baylee_cards_dsl::ZoneSel::GraveyardOf(who)
                        | baylee_cards_dsl::ZoneSel::HandOf(who)
                        | baylee_cards_dsl::ZoneSel::LibraryOf(who)
                        if reads_player(who)
                )
        }
        Amount::Half { base, .. }
        | Amount::Divided { base, .. }
        | Amount::Times { base, .. }
        | Amount::Plus { base, .. }
        | Amount::Negated(base)
        | Amount::SaturatingSub { base, .. } => reads_amount(*base),
        Amount::IfCondition {
            condition,
            then,
            otherwise,
        } => reads_condition(condition) || reads_amount(*then) || reads_amount(*otherwise),
        _ => false,
    }
}
// Effect fields form a flat vocabulary; each event-relative argument must be read.
#[allow(clippy::too_many_lines)]
fn reads_event(effect: &Effect) -> bool {
    match effect {
        Effect::DealDamageWithLimitedLifeGain { amount, life_limit } => {
            reads_amount(*amount) || reads_amount(*life_limit)
        }
        Effect::TapEventObject
        | Effect::PumpEventObject { .. }
        | Effect::EventObjectDealsDamageEqualToPower { .. }
        | Effect::EventObjectDealsDamage { .. }
        | Effect::EventObjectDealsDamageEach { .. }
        | Effect::SacrificeEventDealDamageToEachBlocker { .. }
        | Effect::CopyEventSpellOrAbility
        | Effect::TargetsCopyEventSpellUntilEndOfTurn
        | Effect::IfEventObjectMatches { .. }
        | Effect::IfEventPowerAtLeast { .. }
        | Effect::MoveCounterToEvent { .. }
        | Effect::ExileEventFromGraveyardWithCounters { .. } => true,
        Effect::RedirectTargetSourceDamageThisTurn { source, to, .. } => {
            reads_target(*source)
                || matches!(to, baylee_cards_dsl::DamageRedirectTo::Target(target) if reads_target(*target))
        }
        Effect::PreventNextDamageFromSource { to, .. } => reads_player(*to),
        Effect::RedirectCombatDamageToYouThisTurn { to, .. } => {
            matches!(to, baylee_cards_dsl::DamageRedirectTo::Target(target) if reads_target(*target))
        }
        Effect::CopyResolvingSpell { controller } => reads_player(*controller),
        Effect::DealDamageEvenly { amount, target, .. }
        | Effect::DealDamage { amount, target, .. }
        | Effect::DealDamageWithoutPreventionOrRedirection { amount, target, .. }
        | Effect::DealDamageWithoutPrevention { amount, target, .. }
        | Effect::PutCounterChoice { amount, target, .. } => {
            reads_amount(*amount) || reads_target(*target)
        }
        Effect::DamagePermanentAndController {
            target,
            permanent_amount,
            controller_amount,
        } => {
            reads_target(*target)
                || reads_amount(*permanent_amount)
                || reads_amount(*controller_amount)
        }
        Effect::PayManaToPreventDamage { player, amount, .. }
        | Effect::ScryFor { player, amount, .. }
        | Effect::CreateTokensFor { player, amount, .. }
        | Effect::AddManaChoiceFor {
            who: player,
            amount,
            ..
        } => reads_player(*player) || reads_amount(*amount),
        Effect::ChoosePlayerEx {
            chooser,
            filter,
            if_none,
            then,
            ..
        } => {
            reads_player(*chooser)
                || match filter {
                    baylee_cards_dsl::PlayerFilter::Among(rel)
                    | baylee_cards_dsl::PlayerFilter::NotYetRemembered(rel) => reads_player(*rel),
                    baylee_cards_dsl::PlayerFilter::OpponentControlsMoreThanYou(_)
                    | baylee_cards_dsl::PlayerFilter::OpponentWithGreatestLife
                    | baylee_cards_dsl::PlayerFilter::OpponentNotAttackedBySourceLastCombat => {
                        false
                    }
                }
                || refers_to_event(if_none)
                || refers_to_event(then)
        }
        Effect::ChooseNumberAtRandom { min, max, then } => {
            reads_amount(*min) || reads_amount(*max) || refers_to_event(then)
        }
        Effect::SecretChooseNumbers(spec) => {
            reads_player(spec.players)
                || reads_amount(spec.min)
                || reads_amount(spec.max)
                || refers_to_event(spec.on_each)
                || refers_to_event(spec.on_highest)
                || refers_to_event(spec.on_lowest)
                || refers_to_event(spec.on_not_lowest)
                || refers_to_event(spec.on_matched)
                || refers_to_event(spec.on_unmatched)
        }
        Effect::SecretNumberGuess {
            guesser,
            on_correct,
            on_wrong,
            ..
        } => reads_player(*guesser) || refers_to_event(on_correct) || refers_to_event(on_wrong),
        Effect::RepeatEachPlayerPayUntilNone {
            max_per_player,
            then_per_player,
            ..
        } => reads_amount(*max_per_player) || refers_to_event(then_per_player),
        Effect::LoseLifeAtNextDrawUnlessPaidBefore { player, .. }
        | Effect::LoseHalfLife { player }
        | Effect::SacrificeChosenByOpponent { player, .. }
        | Effect::ExileLibraryAndShuffleHand { player, .. }
        | Effect::ExileGraveyard { player, .. }
        | Effect::DestroyAllOf { who: player, .. }
        | Effect::ShuffleGraveyardMatchingIntoLibrary { who: player, .. }
        | Effect::BottomCardFromHand { player, .. }
        | Effect::PlayerMayPayCostThen { player, .. }
        | Effect::PlayerMayPayCostOr { player, .. }
        | Effect::PlayerMayPayManaOr { player, .. }
        | Effect::PlayerMayPayManaThen { player, .. }
        | Effect::ExileAttackingForThenMaySearchBasicLands { player }
        | Effect::OptionalBasicLandSearchFor { player, .. }
        | Effect::LookHandDiscard { player, .. } => reads_player(*player),
        Effect::PreventNextDamageToAttached { amount }
        | Effect::DealDamageToBothTargetInstances { amount }
        | Effect::DealDamageWithCappedLifeGain { amount, .. }
        | Effect::GainLife { amount, .. }
        | Effect::DrawCards { amount, .. }
        | Effect::DealDamageToTargetController { amount, .. }
        | Effect::DealDamageToAttached { amount, .. }
        | Effect::DealDamageEach { amount, .. }
        | Effect::DealDamageToMatchingTargets { amount, .. }
        | Effect::Scry { amount, .. }
        | Effect::Surveil { amount, .. }
        | Effect::AddMana { amount, .. }
        | Effect::AddCounter { amount, .. }
        | Effect::PutCountersDivided { amount, .. }
        | Effect::AddCountersUpTo { amount, .. }
        | Effect::AddCounterFilter { amount, .. }
        | Effect::AddCounterCreatedTokens { amount, .. }
        | Effect::CreateTokenThenCounters { amount, .. }
        | Effect::CreateTokenN { amount, .. }
        | Effect::Monstrosity(amount)
        | Effect::ChooseAndCreateTokenCopy { amount, .. }
        | Effect::Empower { amount, .. } => reads_amount(*amount),
        Effect::CreateTokenSized {
            amount,
            power,
            toughness,
            who,
            ..
        } => {
            reads_amount(*amount)
                || reads_amount(*power)
                || reads_amount(*toughness)
                || reads_player(*who)
        }
        Effect::Poison { amount, who }
        | Effect::DiscardAmountForPlayers { amount, who }
        | Effect::SacrificeAmountForPlayers { amount, who, .. }
        | Effect::GainLifeFor { amount, who, .. }
        | Effect::DrawCardsFor { amount, who, .. } => reads_amount(*amount) || reads_player(*who),
        Effect::ChooseLandwalk { target, .. }
        | Effect::RedirectNextChosenSourceDamageToTarget { target }
        | Effect::PreventDamageFromTargetingCausesThisTurn { target }
        | Effect::CantAttackDuringNextControllerTurn { target }
        | Effect::SuppressUntap { target, .. }
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
        | Effect::Assimilate { target }
        | Effect::AttachSelf { target, .. }
        | Effect::ExileLinked { target, .. }
        | Effect::BecomeCopy {
            original: target, ..
        }
        | Effect::ExileWithPermission { target, .. }
        | Effect::TurnFaceUp { target }
        | Effect::Plot { target }
        | Effect::Suspect { target }
        | Effect::Unsuspect { target }
        | Effect::AddTempModifier { target, .. }
        | Effect::BecomeChosenBasicLandType { target, .. }
        | Effect::PutEachCounterKindFromSource { target }
        | Effect::BecomesBlocked(target)
        | Effect::SacrificeObject { target, .. }
        | Effect::Unattach { target } => reads_target(*target),
        Effect::SetState {
            subject,
            transition,
            if_cant,
        } => {
            let subj_reads = match subject {
                baylee_cards_dsl::StateSubject::EventObject => true,
                baylee_cards_dsl::StateSubject::BattlefieldFilter(filter) => reads_filter(filter),
                _ => false,
            };
            let trans_reads = match transition {
                baylee_cards_dsl::StateTransition::RevealAndTurnFaceUpIf { filter, .. } => {
                    reads_filter(filter)
                }
                _ => false,
            };
            subj_reads || trans_reads || if_cant.iter().any(reads_event)
        }
        Effect::PutAllCountersFrom { donor, target } => {
            matches!(donor, baylee_cards_dsl::CounterDonor::Event) || reads_target(*target)
        }
        Effect::PutAllCountersFromIntoFound { donor } => {
            matches!(donor, baylee_cards_dsl::CounterDonor::Event)
        }
        Effect::ChooseDistinctPowersOnBattlefield {
            chooser,
            filter,
            max,
            ..
        } => reads_player(*chooser) || reads_filter(filter) || reads_amount(*max),
        Effect::AddGlobalTempModifier { filter, .. } => reads_filter(filter),
        Effect::PreventNextDamageWithThen {
            sources,
            recipients,
            ..
        } => reads_target(*sources) || reads_target(*recipients),
        Effect::GainControlEx { target, to, .. } => reads_target(*target) || reads_player(*to),
        Effect::GainControlAllEx { filter, to, .. } => reads_filter(filter) || reads_player(*to),
        Effect::LookAtTopPick { count, .. }
        | Effect::SearchLibraryUpTo { count, .. }
        | Effect::FlipCoins { count, .. }
        | Effect::RollDice { count, .. }
        | Effect::RepeatEach {
            domain: baylee_cards_dsl::RepeatDomain::Times(count),
            ..
        } => reads_amount(*count),
        Effect::ChooseCard {
            chooser,
            zone,
            filter,
            max,
            ..
        }
        | Effect::ChooseCardEx {
            chooser,
            zone,
            filter,
            max,
            ..
        }
        | Effect::ChooseCardConstrained {
            chooser,
            zone,
            filter,
            max,
            ..
        } => {
            reads_player(*chooser)
                || reads_amount(*max)
                || reads_choose_zone(*zone)
                || reads_filter(filter)
        }
        Effect::ChooseNumberEx {
            chooser, min, max, ..
        } => reads_player(*chooser) || reads_amount(*min) || reads_amount(*max),
        Effect::MoveAttachment {
            attachment,
            recipient_filter,
            ..
        } => reads_target(*attachment) || reads_filter(recipient_filter),
        Effect::PutOntoBattlefieldAttached { subject, host, .. } => {
            reads_defined_objects(*subject) || reads_attach_host(*host)
        }
        Effect::CreateTokenAttached { amount, host, .. } => {
            reads_amount(*amount) || reads_attach_host(*host)
        }
        Effect::PutOntoBattlefieldAttacking {
            subject: defined, ..
        }
        | Effect::DiscardDefined { cards: defined } => reads_defined_objects(*defined),
        Effect::SearchLibraryDistinctNames { filter, count, .. } => {
            reads_filter(filter) || reads_amount(*count)
        }
        Effect::AttachChosen {
            filter, to, max, ..
        }
        | Effect::AttachChosenThen {
            filter, to, max, ..
        } => reads_filter(filter) || reads_target(*to) || reads_amount(*max),
        Effect::BecomeAura {
            target, enchant, ..
        } => reads_target(*target) || reads_target(*enchant),
        Effect::SetDynamicPT {
            subject,
            power,
            toughness,
            ..
        } => {
            reads_target(*subject)
                || power.is_some_and(reads_amount)
                || toughness.is_some_and(reads_amount)
        }
        Effect::AttachObject { object, to, .. } => reads_target(*object) || reads_target(*to),
        Effect::MoveAllAttachments {
            attachments,
            from,
            recipient_filter,
            ..
        } => reads_filter(attachments) || reads_target(*from) || reads_filter(recipient_filter),
        Effect::AttachLtbAttachments { of, filter, to, .. } => {
            reads_target(*of) || reads_filter(filter) || reads_target(*to)
        }
        Effect::RelocateAttachment {
            attachment,
            chooser,
            recipient_filter,
            ..
        } => {
            matches!(attachment, baylee_cards_dsl::AttachmentSource::Spec(t) if reads_target(*t))
                || reads_player(*chooser)
                || reads_filter(recipient_filter)
        }
        Effect::SeparateIntoTwoPiles {
            separator,
            pile_chooser,
            zone,
            filter,
            ..
        } => {
            reads_player(*separator)
                || reads_player(*pile_chooser)
                || reads_filter(filter)
                || matches!(
                    zone,
                    baylee_cards_dsl::ZoneSel::GraveyardOf(who)
                        | baylee_cards_dsl::ZoneSel::HandOf(who)
                        | baylee_cards_dsl::ZoneSel::LibraryOf(who)
                        if reads_player(*who)
                )
        }
        Effect::PeekAndReveal {
            library: player,
            count,
            ..
        }
        | Effect::RevealFromHand {
            who: player,
            max: count,
            ..
        }
        | Effect::RevealTop { player, count, .. } => reads_player(*player) || reads_amount(*count),
        Effect::RevealUntilThen {
            library,
            count,
            max_revealed,
            ..
        } => {
            reads_player(*library) || reads_amount(*count) || max_revealed.is_some_and(reads_amount)
        }
        Effect::ChooseExiledToPlay { owner, .. }
        | Effect::RemoveCountersFromChosenExile { owner, .. }
        | Effect::CreateTokenCopiesOfGraveyard { player: owner, .. } => reads_player(*owner),
        Effect::LoseLife { amount, target, .. } | Effect::Mill { amount, target, .. } => {
            reads_amount(*amount) || reads_player(*target)
        }
        Effect::RedirectNextDamage { target, amount, to } => {
            reads_target(*target) || reads_amount(*amount) || reads_player(*to)
        }
        Effect::GrantSpecialActionUntilEndOfTurn {
            effect: baylee_cards_dsl::SpecialActionEffect::PreventNextDamage { target, amount },
            ..
        }
        | Effect::PreventNextDamage { target, amount, .. } => {
            reads_target(*target) || reads_amount(*amount)
        }
        Effect::AtEndOfCombat { about, .. } | Effect::AtEndOfCombatThisTurn { about, .. } => {
            reads_target(*about)
        }
        Effect::RepeatEachObject {
            zone:
                baylee_cards_dsl::ZoneSel::GraveyardOf(who)
                | baylee_cards_dsl::ZoneSel::HandOf(who)
                | baylee_cards_dsl::ZoneSel::LibraryOf(who),
            ..
        }
        | Effect::BecomeMonarch(who)
        | Effect::TakeInitiative(who)
        | Effect::DestroyChosenForPlayers { who, .. }
        | Effect::DiscardForPlayers { who, .. }
        | Effect::DiscardHand { who, .. }
        | Effect::SacrificeFilter { who, .. }
        | Effect::SacrificeAnyNumber { who, .. }
        | Effect::ReturnChosenToHand { who, .. }
        | Effect::ShuffleIntoLibrary { who, .. }
        | Effect::ShuffleLibrary { who, .. }
        | Effect::AddManaFor { who, .. }
        | Effect::ReorderTopLibraryOf { who, .. }
        | Effect::TapAllOf { who, .. }
        | Effect::LoseUnspentMana { who, .. }
        | Effect::ExileTopToPlay { who, .. }
        | Effect::ExileTopMayCast { who, .. } => reads_player(*who),
        Effect::ExileTopWithPermission { who, count, .. }
        | Effect::ManifestTop { who, count }
        | Effect::PlotTop { who, count }
        | Effect::DiscardRandom { who, count, .. } => reads_player(*who) || reads_amount(*count),
        Effect::Reflexive { target, .. }
        | Effect::CreateTokenCopyOf { target, .. }
        | Effect::PhaseOut { target, .. }
        | Effect::PreventFromChosenSource { target, .. }
        | Effect::PreventFromChosenSourceWithLifeCondition { target, .. }
        | Effect::PreventNextFromMatchingSources { target, .. }
        | Effect::PreventAllDamageFromMatchingThisTurn { target, .. } => {
            target.is_some_and(reads_target)
        }
        Effect::MustBlockIfAble {
            blocker, attacker, ..
        } => reads_target(*blocker) || reads_target(*attacker),
        Effect::ReflexiveTrigger {
            targets, remember, ..
        }
        | Effect::RegisterDelayedTrigger {
            targets, remember, ..
        } => {
            matches!(
                remember,
                baylee_cards_dsl::TriggerRemember::EventObject
                    | baylee_cards_dsl::TriggerRemember::EventSubject
            ) || targets.is_some_and(|req| reads_target(req.spec))
        }
        Effect::SetPTFilter {
            power, toughness, ..
        }
        | Effect::PumpTarget {
            power, toughness, ..
        }
        | Effect::PumpTargetSlot {
            power, toughness, ..
        } => reads_amount(*power) || reads_amount(*toughness),
        Effect::PlayerMayPayOr { player, mana, .. }
        | Effect::PlayerMayPayThen { player, mana, .. }
        | Effect::PlayerMayPayThenElse { player, mana, .. } => {
            reads_player(*player) || reads_amount(*mana)
        }
        Effect::PlayerMayPayLifeOr { player, life, .. }
        | Effect::PlayerMayPayLifeThen { player, life, .. } => {
            reads_player(*player) || reads_amount(*life)
        }
        Effect::ChangeController { new_controller, .. } => reads_player(*new_controller),
        Effect::CreateTokenCopy {
            target,
            controller,
            amount,
            ..
        }
        | Effect::CopySpellOrAbility {
            target,
            controller,
            amount,
            ..
        } => reads_target(*target) || reads_player(*controller) || reads_amount(*amount),
        Effect::SearchLibraryOf { library, .. } => reads_player(*library),
        Effect::PumpFilter {
            controlled_by,
            power,
            toughness,
            ..
        } => {
            controlled_by.is_some_and(reads_player)
                || reads_amount(*power)
                || reads_amount(*toughness)
        }
        Effect::IfCondition { condition, .. } | Effect::RepeatWhile { condition, .. } => {
            reads_condition(condition)
        }
        Effect::RepeatProcess {
            max_times,
            pre_condition,
            post_condition,
            ..
        } => {
            max_times.is_some_and(reads_amount)
                || pre_condition.as_ref().is_some_and(reads_condition)
                || post_condition.as_ref().is_some_and(reads_condition)
        }
        Effect::SoulbondPair { with_event_object } => *with_event_object,
        Effect::ReduceNextSpellCost { who, generic, .. } => {
            reads_player(*who) || reads_amount(*generic)
        }
        Effect::MultiplyCounters { subject, .. } => match subject {
            baylee_cards_dsl::CounterSubject::EventObject => true,
            baylee_cards_dsl::CounterSubject::Player(who) => reads_player(*who),
            _ => false,
        },
        Effect::MoveCounters { from, to, .. } => {
            matches!(from, baylee_cards_dsl::MoveCounterSource::Filter(f) if reads_filter(f))
                || matches!(to, baylee_cards_dsl::MoveCounterDest::EventObject)
                || matches!(to, baylee_cards_dsl::MoveCounterDest::DistributeAmong(f) if reads_filter(f))
        }
        Effect::RemoveCounters { subject, count, .. } => {
            let s_reads = match subject {
                baylee_cards_dsl::RemoveCounterSubject::Player(who) => reads_player(*who),
                baylee_cards_dsl::RemoveCounterSubject::Choice(f)
                | baylee_cards_dsl::RemoveCounterSubject::Filter(f) => reads_filter(f),
                _ => false,
            };
            let c_reads = match count {
                baylee_cards_dsl::RemoveCounterCount::Exact(a)
                | baylee_cards_dsl::RemoveCounterCount::UpTo(a) => reads_amount(*a),
                _ => false,
            };
            s_reads || c_reads
        }
        Effect::Play {
            source, controller, ..
        } => {
            reads_player(*controller)
                || match source {
                    baylee_cards_dsl::PlaySource::Subject(spec) => reads_target(*spec),
                    baylee_cards_dsl::PlaySource::Zone { owner, .. } => reads_player(*owner),
                    baylee_cards_dsl::PlaySource::LinkedExile => false,
                }
        }
        _ => false,
    }
}

fn reads_condition(condition: &baylee_cards_dsl::Condition) -> bool {
    use baylee_cards_dsl::Condition as C;
    match condition {
        C::ObjectMatches { what, .. } => reads_target(**what),
        C::AmountAtLeast(amount, _) | C::AmountIsEven(amount) => reads_amount(*amount),
        C::AmountCompare { left, right, .. } => reads_amount(**left) || reads_amount(**right),
        C::SoulbondEventCanPair => true,
        C::All(parts) | C::Any(parts) => parts.iter().any(reads_condition),
        C::Not(inner) | C::TriggerOnly(inner) => reads_condition(inner),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static EFFECTS: &[Effect] = &[Effect::PlayerMayPayManaThen {
        player: PlayerRel::You,
        cost: baylee_core::mana!("{B}{B}"),
        effects: &[Effect::LookHandDiscard {
            player: PlayerRel::ControllerOfEvent,
            count: Amount::Fixed(1),
            filter: &baylee_cards_dsl::Filter::Any,
        }],
    }];

    #[test]
    fn nested_private_hand_discard_retains_the_event_player_reference() {
        assert!(refers_to_event(EFFECTS));
    }
}
