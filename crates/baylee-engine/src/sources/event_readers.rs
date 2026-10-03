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
        Amount::EventLastToughness => true,
        Amount::Plus { base, .. } | Amount::Negated(base) | Amount::SaturatingSub { base, .. } => {
            reads_amount(*base)
        }
        _ => false,
    }
}
// Effect fields form a flat vocabulary; each event-relative argument must be read.
#[allow(clippy::too_many_lines)]
fn reads_event(effect: &Effect) -> bool {
    match effect {
        Effect::EventObjectDealsDamageEqualToPower { .. }
        | Effect::IfEventObjectMatches { .. }
        | Effect::IfEventPowerAtLeast { .. } => true,
        Effect::DealDamageEvenly { amount, target, .. }
        | Effect::DealDamage { amount, target, .. } => {
            reads_amount(*amount) || reads_target(*target)
        }
        Effect::PayManaToPreventDamage { player, amount, .. }
        | Effect::ScryFor { player, amount, .. } => reads_player(*player) || reads_amount(*amount),
        Effect::LoseHalfLife { player }
        | Effect::SacrificeChosenByOpponent { player, .. }
        | Effect::ExileLibraryAndShuffleHand { player, .. }
        | Effect::ExileGraveyard { player, .. }
        | Effect::BottomCardFromHand { player, .. }
        | Effect::PlayerMayPayCostOr { player, .. }
        | Effect::PlayerMayPayManaOr { player, .. }
        | Effect::PlayerMayPayManaThen { player, .. }
        | Effect::OptionalBasicLandSearchFor { player, .. } => reads_player(*player),
        Effect::DealDamageWithCappedLifeGain { amount, .. }
        | Effect::GainLife { amount, .. }
        | Effect::DrawCards { amount, .. }
        | Effect::DealDamageToTargetController { amount, .. }
        | Effect::DealDamageToAttached { amount, .. }
        | Effect::DealDamageEach { amount, .. }
        | Effect::Scry { amount, .. }
        | Effect::Surveil { amount, .. }
        | Effect::AddMana { amount, .. }
        | Effect::AddCounter { amount, .. }
        | Effect::AddCountersUpTo { amount, .. }
        | Effect::AddCounterFilter { amount, .. }
        | Effect::CreateTokenN { amount, .. } => reads_amount(*amount),
        Effect::GainLifeFor { amount, who, .. } | Effect::DrawCardsFor { amount, who, .. } => {
            reads_amount(*amount) || reads_player(*who)
        }
        Effect::Exile { target, .. }
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
        Effect::AtEndOfCombat { about, .. } => reads_target(*about),
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
        } => reads_amount(*power) || reads_amount(*toughness),
        Effect::PlayerMayPayOr { player, mana, .. }
        | Effect::PlayerMayPayThen { player, mana, .. } => {
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
