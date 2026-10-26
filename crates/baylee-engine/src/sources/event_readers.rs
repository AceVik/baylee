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
        Amount::Players(who) | Amount::LibrarySize(who) | Amount::LifeTotal(who) => {
            reads_player(who)
        }
        Amount::CountOf {
            zone: baylee_cards_dsl::ZoneSel::GraveyardOf(who),
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
        | Effect::EventObjectDealsDamageEqualToPower { .. }
        | Effect::EventObjectDealsDamage { .. }
        | Effect::CopyEventSpellOrAbility
        | Effect::TargetsCopyEventSpellUntilEndOfTurn
        | Effect::IfEventObjectMatches { .. }
        | Effect::IfEventPowerAtLeast { .. }
        | Effect::MoveCounterToEvent { .. } => true,
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
        | Effect::BottomCardFromHand { player, .. }
        | Effect::PlayerMayPayCostThen { player, .. }
        | Effect::PlayerMayPayCostOr { player, .. }
        | Effect::PlayerMayPayManaOr { player, .. }
        | Effect::PlayerMayPayManaThen { player, .. }
        | Effect::OptionalBasicLandSearchFor { player, .. } => reads_player(*player),
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
        | Effect::CreateTokenThenCounters { amount, .. }
        | Effect::CreateTokenN { amount, .. } => reads_amount(*amount),
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
        | Effect::Assimilate { target }
        | Effect::AttachSelf { target, .. }
        | Effect::ExileLinked { target, .. }
        | Effect::SacrificeObject { target, .. } => reads_target(*target),
        Effect::LookAtTopPick { count, .. } | Effect::SearchLibraryUpTo { count, .. } => {
            reads_amount(*count)
        }
        Effect::ChooseExiledToPlay { owner, .. } => reads_player(*owner),
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
        Effect::BecomeMonarch(who)
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
        Effect::DiscardRandom { who, count, .. } => reads_player(*who) || reads_amount(*count),
        Effect::Reflexive { target, .. }
        | Effect::CreateTokenCopyOf { target, .. }
        | Effect::PhaseOut { target, .. } => target.is_some_and(reads_target),
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
        Effect::PlayerMayPayLifeOr { player, life, .. } => {
            reads_player(*player) || reads_amount(*life)
        }
        Effect::ChangeController { new_controller, .. } => reads_player(*new_controller),
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
        _ => false,
    }
}
