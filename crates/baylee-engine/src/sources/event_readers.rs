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
fn reads_target(value: TargetSpec) -> bool {
    match value {
        TargetSpec::EventObject | TargetSpec::CardInGraveyardBelowEvent(..) => true,
        TargetSpec::Player(who)
        | TargetSpec::CardInGraveyard(_, who)
        | TargetSpec::CardInGraveyardBelowValue(_, who, _) => reads_player(who),
        _ => false,
    }
}
fn reads_amount(value: Amount) -> bool {
    match value {
        Amount::EventLastToughness | Amount::EventPower => true,
        Amount::Sum { left, right } | Amount::Difference { left, right } => {
            reads_amount(*left) || reads_amount(*right)
        }
        Amount::Players(who)
        | Amount::LibrarySize(who)
        | Amount::LifeTotal(who)
        | Amount::StartingLifeTotal(who)
        | Amount::LifeGainedThisTurn(who)
        | Amount::CountOf {
            zone:
                baylee_cards_dsl::ZoneSel::GraveyardOf(who)
                | baylee_cards_dsl::ZoneSel::HandOf(who)
                | baylee_cards_dsl::ZoneSel::LibraryOf(who),
            ..
        } => reads_player(who),
        Amount::Half { base, .. }
        | Amount::Divided { base, .. }
        | Amount::Times { base, .. }
        | Amount::Plus { base, .. }
        | Amount::Negated(base)
        | Amount::SaturatingSub { base, .. } => reads_amount(*base),
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
        | Effect::DealDamageWithoutPreventionOrRedirection { amount, target, .. } => {
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
        | Effect::CreateTokensFor { player, amount, .. } => {
            reads_player(*player) || reads_amount(*amount)
        }
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
        | Effect::AddCountersUpTo { amount, .. }
        | Effect::AddCounterFilter { amount, .. }
        | Effect::AddCounterCreatedTokens { amount, .. }
        | Effect::CreateTokenThenCounters { amount, .. }
        | Effect::CreateTokenN { amount, .. }
        | Effect::Monstrosity(amount)
        | Effect::ChooseAndCreateTokenCopy { amount, .. } => reads_amount(*amount),
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
        | Effect::SacrificeObject { target, .. } => reads_target(*target),
        Effect::LookAtTopPick { count, .. }
        | Effect::SearchLibraryUpTo { count, .. }
        | Effect::FlipCoins { count, .. }
        | Effect::RollDice { count, .. }
        | Effect::RepeatEach {
            domain: baylee_cards_dsl::RepeatDomain::Times(count),
            ..
        } => reads_amount(*count),
        Effect::ChooseCard {
            chooser, zone, max, ..
        } => {
            reads_player(*chooser)
                || reads_amount(*max)
                || matches!(zone, baylee_cards_dsl::ChooseZone::Graveyard(p) if reads_player(*p))
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
        C::Not(inner) => reads_condition(inner),
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
