/// Semantic effect channels (`24` dimensions) shared by per-ability rows and
/// card-level effect aggregation.
#[derive(Default, Clone, Copy)]
struct EffectChannels {
    draw: f32,
    scry_surveil: f32,
    tutor_search: f32,
    mill: f32,
    discard: f32,
    mana_add: f32,
    damage_any: f32,
    damage_board: f32,
    destroy_single: f32,
    destroy_all: f32,
    exile_single: f32,
    exile_all_or_gy: f32,
    bounce: f32,
    counter_spell: f32,
    tap_untap: f32,
    pump_pt: f32,
    counters_add: f32,
    tokens_create: f32,
    reanimate: f32,
    life_gain: f32,
    life_lose: f32,
    sacrifice_force: f32,
    blink_copy_control: f32,
    extra_turn_or_win: f32,
}

impl EffectChannels {
    fn to_array(self) -> [f32; 24] {
        [
            unit(self.draw, 4.0),
            unit(self.scry_surveil, 4.0),
            unit(self.tutor_search, 2.0),
            unit(self.mill, 6.0),
            unit(self.discard, 4.0),
            unit(self.mana_add, 4.0),
            unit(self.damage_any, 6.0),
            unit(self.damage_board, 6.0),
            unit(self.destroy_single, 2.0),
            unit(self.destroy_all, 2.0),
            unit(self.exile_single, 2.0),
            unit(self.exile_all_or_gy, 2.0),
            unit(self.bounce, 2.0),
            unit(self.counter_spell, 2.0),
            unit(self.tap_untap, 3.0),
            signed_unit(self.pump_pt, 6.0),
            unit(self.counters_add, 4.0),
            unit(self.tokens_create, 4.0),
            unit(self.reanimate, 2.0),
            unit(self.life_gain, 6.0),
            unit(self.life_lose, 6.0),
            unit(self.sacrifice_force, 2.0),
            unit(self.blink_copy_control, 2.0),
            unit(self.extra_turn_or_win, 1.0),
        ]
    }
}

fn amount_scalar(amount: &Amount) -> f32 {
    match amount {
        Amount::Fixed(n) => (*n as f32).clamp(0.0, 10.0),
        Amount::NegXFixed(n) => -(*n as f32).clamp(0.0, 10.0),
        Amount::X | Amount::DoubleX | Amount::XPlusCommanderCasts => 3.0,
        Amount::NegX => -3.0,
        _ => 2.0,
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "Explicit typed effect-to-channel schema"
)]
#[allow(
    clippy::manual_midpoint,
    reason = "Preserve version-2 feature arithmetic bit for bit"
)]
fn accumulate_effect(effect: &Effect, ch: &mut EffectChannels) {
    match effect {
        Effect::DrawCards { amount } | Effect::DrawCardsFor { amount, .. } => {
            ch.draw += amount_scalar(amount).max(1.0);
        }
        Effect::DungeonDrawThree => ch.draw += 3.0,
        Effect::DiscardAnyThenDraw
        | Effect::DiscardUpToThenDraw { .. }
        | Effect::Connive { .. } => {
            ch.draw += 1.0;
            ch.discard += 1.0;
        }
        Effect::LookAtTopPick { .. }
        | Effect::LookAtTopFind { .. }
        | Effect::LookAtTopMayTake { .. }
        | Effect::LookAtTopKeepBottomPlay { .. }
        | Effect::RevealTopOnePerType { .. }
        | Effect::Cascade
        | Effect::Discover { .. }
        | Effect::DiscoverAmount { .. } => {
            ch.draw += 1.0;
        }
        Effect::Scry { amount } | Effect::Surveil { amount } | Effect::ScryFor { amount, .. } => {
            ch.scry_surveil += amount_scalar(amount).max(1.0);
        }
        Effect::Explore { .. }
        | Effect::LookAtTopOf { .. }
        | Effect::ReorderTopLibrary { .. }
        | Effect::ReorderTopLibraryOf { .. } => {
            ch.scry_surveil += 1.0;
        }
        Effect::SearchLibrary { .. }
        | Effect::SearchLibraryBeforeShuffle { .. }
        | Effect::SearchSharedLandType
        | Effect::SearchWithManaDifference { .. }
        | Effect::SearchLibraryOrGraveyard { .. }
        | Effect::SearchLibraryOf { .. }
        | Effect::SearchLibraryUpTo { .. }
        | Effect::OptionalBasicLandSearchFor { .. }
        | Effect::Recruit { .. }
        | Effect::WishToHand { .. } => {
            ch.tutor_search += 1.0;
        }
        Effect::Mill { amount, .. } => {
            ch.mill += amount_scalar(amount).max(1.0);
        }
        Effect::MillMayTakeOne { .. } => {
            ch.mill += 2.0;
        }
        Effect::DiscardCount { count, .. }
        | Effect::DiscardAmountForPlayers { amount: count, .. } => {
            ch.discard += amount_scalar(count).max(1.0);
        }
        Effect::DiscardForPlayers { .. }
        | Effect::DiscardRandom { .. }
        | Effect::DiscardHand { .. }
        | Effect::RevealHandDiscard { .. }
        | Effect::DiscardMatching { .. }
        | Effect::DiscardRandomMatching { .. }
        | Effect::DiscardUpToForPlayers { .. }
        | Effect::DungeonExileTwo => {
            ch.discard += 1.5;
        }
        Effect::AddMana { amount, .. } => {
            ch.mana_add += amount_scalar(amount).max(1.0);
        }
        Effect::AddManaFor { .. }
        | Effect::ManaForColorsAmong(_)
        | Effect::AddManaLikeEvent { .. }
        | Effect::AddCombatMana { .. }
        | Effect::DelayedManaAtNextMain { .. }
        | Effect::AddManaUnlessAnyPlayerPays { .. } => {
            ch.mana_add += 1.5;
        }
        Effect::DealDamage { amount, .. }
        | Effect::DealDamageToBothTargetInstances { amount }
        | Effect::DealDamageToMatchingTargets { amount, .. }
        | Effect::DealDamageToTargetController { amount }
        | Effect::DealDamageToAttached { amount }
        | Effect::DealDamageWithCappedLifeGain { amount, .. }
        | Effect::DealDamageWithLimitedLifeGain { amount, .. }
        | Effect::EventObjectDealsDamage { amount, .. } => {
            ch.damage_any += amount_scalar(amount).max(1.0);
        }
        Effect::DealDamageDivided { amount, .. } => {
            let v = match amount {
                DivisionAmount::Fixed(n) => (*n as f32).clamp(1.0, 10.0),
                DivisionAmount::XPlus(offset) => (3.0 + *offset as f32).clamp(1.0, 10.0),
                DivisionAmount::XDoubleFrom(_) => 3.0,
            };
            ch.damage_any += v;
        }
        Effect::Fight { .. }
        | Effect::DamageEqualToPower { .. }
        | Effect::EventObjectDealsDamageEqualToPower { .. } => {
            ch.damage_any += 2.5;
        }
        Effect::DealDamageEach { amount, .. }
        | Effect::DealDamageEvenly { amount, .. }
        | Effect::DealDamageEvenlyToFilter { amount, .. }
        | Effect::DealDamageToEachOpponentAndPlaneswalkerDamagedThisGame { amount } => {
            ch.damage_board += amount_scalar(amount).max(2.0);
        }
        Effect::Destroy { .. }
        | Effect::DestroyTargetSlots { .. }
        | Effect::DestroyEventThenMayReattach { .. }
        | Effect::DestroyWeakest { .. }
        | Effect::DestroyRandom { .. } => {
            ch.destroy_single += 1.0;
        }
        Effect::DestroyAll { .. }
        | Effect::DestroyAllThenForEachDestroyed { .. }
        | Effect::DestroyAllWithManaValue { .. }
        | Effect::DestroyAllExceptChosen { .. }
        | Effect::DestroyOthersNamedLike { .. }
        | Effect::DestroyChosenForPlayers { .. }
        | Effect::DestroyEachUnlessAnyPlayerPaysLife { .. } => {
            ch.destroy_all += 1.0;
        }
        Effect::Exile { .. }
        | Effect::ExileLinked { .. }
        | Effect::ExileTargetsWithSource
        | Effect::ExileTargetsCreateTokens { .. } => {
            ch.exile_single += 1.0;
        }
        Effect::ExileAll { .. }
        | Effect::ExileGraveyard { .. }
        | Effect::ExileLibraryAndShuffleHand { .. } => {
            ch.exile_all_or_gy += 1.0;
        }
        Effect::ReturnToHand { .. }
        | Effect::ReturnAllToHand { .. }
        | Effect::ReturnAllOwnedToHand { .. }
        | Effect::ReturnChosenToHand { .. }
        | Effect::PutTargetOnBottomOfLibrary => {
            ch.bounce += 1.0;
        }
        Effect::CounterTargetSpell
        | Effect::CounterTargetSpellToExile
        | Effect::CounterTargetAbility
        | Effect::CounterTargetSpellOrAbility
        | Effect::CounterEventAbility => {
            ch.counter_spell += 1.0;
        }
        Effect::TapTarget
        | Effect::TapEventObject
        | Effect::ToggleTapTarget
        | Effect::TapAll { .. }
        | Effect::TapAllOf { .. }
        | Effect::UntapTarget
        | Effect::UntapSelf
        | Effect::UntapFound
        | Effect::UntapAll { .. }
        | Effect::UntapChosen { .. } => {
            ch.tap_untap += 1.0;
        }
        Effect::PumpTarget {
            power, toughness, ..
        }
        | Effect::PumpTargetSlot {
            power, toughness, ..
        }
        | Effect::PumpFilter {
            power, toughness, ..
        } => {
            ch.pump_pt += (amount_scalar(power) + amount_scalar(toughness)) * 0.5;
        }
        Effect::ModifyTargets { .. }
        | Effect::ModifySource { .. }
        | Effect::ModifyAttached { .. }
        | Effect::ModifyEvent { .. } => {
            ch.pump_pt += 1.5;
        }
        Effect::AddCounter { amount, .. }
        | Effect::AddCounterSource { amount, .. }
        | Effect::AddCounterFilter { amount, .. } => {
            ch.counters_add += amount_scalar(amount).max(1.0);
        }
        Effect::Proliferate
        | Effect::AddCountersUpTo { .. }
        | Effect::DoubleCountersFilter { .. }
        | Effect::MoveCounter { .. }
        | Effect::MoveCounterToEvent { .. }
        | Effect::Amass { .. }
        | Effect::Energy { .. }
        | Effect::Poison { .. } => {
            ch.counters_add += 1.5;
        }
        Effect::CreateToken { .. }
        | Effect::CreateTokenN { .. }
        | Effect::CreateTokensFor { .. }
        | Effect::CreateTokenModified { .. }
        | Effect::CreateTokenAttacking { .. }
        | Effect::CreateTokenThenCounters { .. }
        | Effect::CreateTokenThenAttachSelf { .. }
        | Effect::CreateTokenPtPerCount { .. }
        | Effect::CreateTokenForTargetController { .. }
        | Effect::CreateTokenCopyOf { .. }
        | Effect::CreateTokenCopyOfEquipped { .. }
        | Effect::CreateTokenCopyOfTarget { .. }
        | Effect::CreateTokenCopyOfSource { .. }
        | Effect::CreateTokenCopyOfFirstToken
        | Effect::Fabricate { .. }
        | Effect::Populate => {
            ch.tokens_create += 1.5;
        }
        Effect::GraveyardToBattlefield { .. }
        | Effect::GraveyardToBattlefieldLinked { .. }
        | Effect::YourGraveyardToBattlefield { .. }
        | Effect::AllGraveyardCreaturesToBattlefield
        | Effect::EachPlayerGraveyardToBattlefield { .. }
        | Effect::ReturnSourceFromGraveyard { .. }
        | Effect::GraveyardToHand { .. }
        | Effect::GraveyardAllToHand { .. }
        | Effect::RandomGraveyardToHand { .. }
        | Effect::ReturnFromGraveyardOf { .. }
        | Effect::ReanimateEnchanted => {
            ch.reanimate += 1.0;
        }
        Effect::GainLife { amount } | Effect::GainLifeFor { amount, .. } => {
            ch.life_gain += amount_scalar(amount).max(1.0);
        }
        Effect::GainLifeDoubleX => ch.life_gain += 4.0,
        Effect::LoseLife { amount, .. } => {
            ch.life_lose += amount_scalar(amount).max(1.0);
        }
        Effect::LoseHalfLife { .. } => ch.life_lose += 4.0,
        Effect::SacrificeAmountForPlayers { amount, .. } => {
            ch.sacrifice_force += amount_scalar(amount).max(1.0);
        }
        Effect::SacrificeFilter { .. }
        | Effect::SacrificeAll { .. }
        | Effect::SacrificeChosenByOpponent { .. }
        | Effect::SacrificeObject { .. }
        | Effect::SacrificeEvent
        | Effect::DungeonSacrificeThree => {
            ch.sacrifice_force += 1.0;
        }
        Effect::Blink { .. }
        | Effect::CopyTargetSpell { .. }
        | Effect::CopyTargetAbility
        | Effect::CopyEventSpellOrAbility
        | Effect::CopyResolvingSpell { .. }
        | Effect::CopyThisSpell { .. }
        | Effect::ChangeController { .. }
        | Effect::GainControlAll { .. }
        | Effect::ExchangeControl
        | Effect::BecomeCopyOfTarget { .. } => {
            ch.blink_copy_control += 1.0;
        }
        Effect::TakeExtraTurn | Effect::WinGame => {
            ch.extra_turn_or_win += 1.0;
        }
        _ => {}
    }
}

fn walk_effects(effects: &'static [Effect], ch: &mut EffectChannels) {
    let mut seen = 0;
    Effect::walk(effects, &mut seen, &mut |eff| accumulate_effect(eff, ch));
}
