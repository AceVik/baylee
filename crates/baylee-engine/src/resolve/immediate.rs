//! The effects carried out at once, without asking.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary
use super::*;

/// Operations that complete immediately. This is only the dispatcher:
/// effect families live in their own modules (life/damage, zones, mana,
/// counters/P-T, tokens); control, draw, conditions, and the misc tail
/// stay below.
#[allow(clippy::too_many_lines)] // dispatch table + misc tail
pub(super) fn exec_immediate(
    state: &mut GameState,
    res: &mut Resolution,
    op: Effect,
) -> Option<Pending> {
    let you = res.controller;
    match op {
        Effect::ExileMostVotedNotControlledByYou { filter } => vote::start(state, res, filter),
        Effect::ExileAttackingForThenMaySearchBasicLands { player } => {
            settle::start(state, res, player)
        }
        Effect::ChooseSourceColor => {
            let (source, pending) = source_color::choice(state, res)?;
            res.awaiting = Some(AwaitingOp::SourceColor { source });
            Some(pending)
        }
        Effect::PlayerMayPayManaAndLifeEachThenElse {
            player,
            mana,
            life,
            times,
            paid,
            unpaid,
        } => mana_life_payment::offer(state, res, player, mana, life, times, paid, unpaid),
        Effect::AnyPlayerMayPayManaOr { cost, effects } => {
            any_player_payment::offer(state, res, cost, effects, 0)
        }
        Effect::ForEachCreatureDamagedBySourceThatDiedThisTurn { effects } => {
            super::damage_returns::for_each(state, res, effects, op);
            None
        }
        Effect::GrantTriggeredIfAbsent {
            target,
            trigger,
            effects,
            duration,
        } => {
            super::conditional_grants::grant(state, res, target, trigger, effects, duration);
            None
        }
        Effect::PlayerMayGainControl {
            player,
            filter,
            duration,
        } => super::control_choice::start(state, res, player, filter, duration),
        Effect::SacrificeNOrElse {
            filter,
            count,
            otherwise,
        } => super::control_choice::sacrifice_n_or_else(state, res, filter, count, otherwise),
        Effect::DrawThenDiscardDrawn { draw, discard } => {
            loot::drawn_only(state, res, draw, discard)
        }
        Effect::ExileTopFaceDownUntilYourNextEndStep => {
            super::hidden_exile::exile_top_until_own_end(state, res);
            None
        }
        Effect::ExileDiscardedCardFromYourGraveyard => {
            super::hidden_exile::exile_discarded(state, res);
            None
        }

        Effect::ExchangeBlockersOfTargetAttackers => {
            if let Some((&first, &second)) = res
                .targets
                .first()
                .zip(res.targets.get(1).or_else(|| res.second_targets.first()))
            {
                crate::combat::exchange_blockers(state, first, second);
            }
            None
        }
        Effect::CreateDelayedObserver {
            watch,
            when,
            about,
            effects,
        } => {
            super::control_observers::register(state, res, watch, when, about, effects);
            None
        }
        Effect::TapAndPayEnergyForCounters { who, filter, kind } => {
            super::tap_energy::start(state, res, who, filter, kind)
        }
        Effect::Radiation => {
            super::milling::radiation(state, res);
            None
        }
        Effect::Proliferate => super::proliferate::start(state, res),
        Effect::Hideaway(count) => super::hidden_exile::hideaway(state, res, count),
        Effect::ExileTopFaceDownLinked => {
            super::hidden_exile::exile_top(state, res);
            None
        }
        Effect::MayPlayLinkedExile => super::hidden_exile::offer_play(state, res),
        Effect::MayCopyCastLinkedExile => super::hidden_exile::offer_copy(state, res),
        Effect::ReturnLinkedExile { to } => super::hidden_exile::return_cards(state, res, to),
        Effect::SacrificeSelfThen { then } => {
            if super::zones::can_sacrifice_self(state, res) {
                let before = state.object(res.source).map(|o| o.version);
                super::zones::exec(state, res, Effect::SacrificeSelf);
                if state.object(res.source).map(|o| o.version) != before {
                    return run_nested(state, res, then);
                }
            }
            None
        }

        Effect::Sequence(_) => unreachable!("sequences are flattened"),
        Effect::TakklemaggotDeath => takklemaggot_death(state, res),
        Effect::MeldWithNamed {
            source_name,
            partner_name,
            melded,
        } => super::meld::meld_with_named(state, res, source_name, partner_name, melded),
        Effect::DungeonSacrificeThree => super::dungeon::sacrifice_three(state, res),
        Effect::DungeonDrawThree => super::dungeon::draw_three(state, res),
        Effect::DungeonExileTwo => super::dungeon::exile_two(state, res),
        Effect::DungeonTax { discard } => super::dungeon::tax(state, res, discard),
        Effect::MergeMutatingSpell => Some(super::mutate::start(res)),
        Effect::VentureIntoTheDungeon => super::dungeon::venture_into_the_dungeon(state, res),
        Effect::DestroyEventThenMayReattach { to } => {
            crate::aura_bindings::destroy_event_and_offer(state, res, to)
        }
        Effect::SacrificeEvent => {
            crate::aura_bindings::sacrifice_event(state, res);
            None
        }
        Effect::SacrificeAmountOrLose { filter, amount } => {
            sacrifice_amount_or_lose(state, res, filter, &amount)
        }
        Effect::Poison { who, amount }
        | Effect::Energy { who, amount }
        | Effect::Rad { who, amount } => {
            let amount = amount2(&amount, state, you, res);
            let kind = match op {
                Effect::Poison { .. } => baylee_cards_dsl::CounterKind::Poison,
                Effect::Energy { .. } => baylee_cards_dsl::CounterKind::Energy,
                Effect::Rad { .. } => baylee_cards_dsl::CounterKind::Rad,
                _ => unreachable!("matched player counter effect"),
            };
            for player in players_of(who, state, you, res) {
                state.put_player_counters_by(res.controller, player, kind, amount);
            }
            None
        }
        Effect::LoseGame => {
            let _ = sba::lose_by_effect(state, you);
            None
        }
        // CR 104.2b: the controller wins, and with them their team; the
        // game ends at once (CR 104.1), which the engine reads before
        // anything else (`Engine::game_result`). The first ending stands.
        Effect::WinGame => {
            if !state.has_left(you) && !sba::player_cant_win(state, you) && state.game_end.is_none()
            {
                let winner = match state.side_of(you) {
                    crate::state::Side::Team(team) => crate::win::Victor::Team(team),
                    crate::state::Side::Solo(player) => crate::win::Victor::Player(player),
                };
                state.game_end = Some(crate::win::GameResult {
                    winner: Some(winner),
                    reason: crate::win::EndReason::EffectWin,
                });
            }
            None
        }
        // CR 104.4c.
        Effect::DrawGame => {
            if state.game_end.is_none() {
                state.game_end = Some(crate::win::GameResult {
                    winner: None,
                    reason: crate::win::EndReason::Draw,
                });
            }
            None
        }
        Effect::ReanimateEnchanted | Effect::ReanimateEnchantedTapped => {
            if let Some(finish) = crate::aura_bindings::begin_reanimation(
                state,
                res,
                false,
                matches!(op, Effect::ReanimateEnchantedTapped),
            ) {
                state.reanimation_finishes.push(finish);
            }
            None
        }
        Effect::BecomeCopyOfTarget { mods, until_eot } => {
            let source = subjects::source(state, res)?;
            if !subjects::is_current(state, source) {
                return None;
            }
            let target = *res.targets.first()?;
            let loc = state.object(res.on_stack).and_then(|object| object.ability);
            let resolving = loc.and_then(|loc| {
                state
                    .printed_ability_list(res.on_stack)?
                    .entry(loc.index as usize)
            });
            crate::copiable_abilities::apply_copy(
                state,
                crate::copiable_abilities::CopyApplication {
                    source: source.object,
                    target,
                    mods,
                    copy_index: loc.map_or(0, |loc| loc.index),
                    until_eot,
                    resolving,
                    text: res.text,
                },
            );
            None
        }
        Effect::ControlPlayerPlayCard { player } => {
            let player = players_of(player, state, you, res).first().copied()?;
            let options = state.zones.list(ZoneLocation::Hand(player)).clone();
            if options.is_empty() {
                return None;
            }
            res.awaiting = Some(AwaitingOp::ControlledCard { player });
            Some(Pending::ChooseCards {
                player: you,
                options,
                min: 1,
                max: 1,
                prompt: crate::choice::ChoicePrompt::CommandCard,
                total: None,
            })
        }
        Effect::CastFaceDownUsingSpentX => {
            let options = state
                .zones
                .list(ZoneLocation::Hand(you))
                .iter()
                .copied()
                .filter(|&card| {
                    let Some(object) = state.object(card) else {
                        return false;
                    };
                    if !object
                        .characteristics()
                        .types
                        .contains(baylee_core::types::TypeSet::CREATURE)
                    {
                        return false;
                    }
                    let Some(paid) = state
                        .object(res.on_stack)
                        .and_then(|object| object.paid.as_ref())
                    else {
                        return object.characteristics().mana_cost.with_x(0)
                            == baylee_core::mana::ManaCost::ZERO;
                    };
                    crate::mana_pay::paid_subset_can_pay(
                        &paid.mana_paid,
                        &object.characteristics().mana_cost.with_x(0),
                        res.x.unwrap_or(0),
                        &paid.fixed_mana_cost,
                        paid.mana_spending,
                    )
                })
                .collect();
            res.awaiting = Some(AwaitingOp::MaskedCast);
            Some(Pending::ChooseCards {
                player: you,
                options,
                min: 0,
                max: 1,
                prompt: crate::choice::ChoicePrompt::CastFaceDown {
                    x: res.x.unwrap_or(0),
                    paid: state
                        .object(res.on_stack)
                        .and_then(|object| object.paid.as_ref())
                        .map_or([0; 6], |paid| paid.mana_types_spent),
                    fixed_cost: state
                        .object(res.on_stack)
                        .and_then(|object| object.paid.as_ref())
                        .map_or(baylee_core::mana::ManaCost::ZERO, |paid| {
                            paid.fixed_mana_cost
                        }),
                },
                total: None,
            })
        }
        Effect::ActivateLandsAndTakeMana { player } => {
            let player = players_of(player, state, you, res).first().copied()?;
            res.awaiting = Some(AwaitingOp::LandMana {
                player,
                beneficiary: you,
            });
            Some(Pending::ChooseManaAbility {
                player,
                choice: crate::choice::ManaChoiceId {
                    source: state.source_identity(res.on_stack)?,
                    step: 0,
                },
                options: Vec::new(),
            })
        }
        Effect::ChangeTextWord { kind } => {
            let object = state.object(*res.targets.first()?)?;
            res.awaiting = Some(AwaitingOp::TextReplacement {
                target: baylee_core::ids::DamageSourceRef {
                    object: object.id,
                    version: object.version,
                },
                kind,
            });
            Some(Pending::ChooseNumber {
                player: you,
                min: 0,
                max: 19,
                reason: crate::choice::NumberPrompt::TextReplacement {
                    kind,
                    target: baylee_core::ids::DamageSourceRef {
                        object: object.id,
                        version: object.version,
                    },
                },
            })
        }

        Effect::GainLife { .. }
        | Effect::SacrificeEventDealDamageToEachBlocker { .. }
        | Effect::GainLifeFor { .. }
        | Effect::SetLifeTotal { .. }
        | Effect::RaiseLifeTotalTo { .. }
        | Effect::GainLifeDoubleX
        | Effect::PlayersWhoCastRevealedNameThisTurnLoseLife { .. }
        | Effect::LoseLife { .. }
        | Effect::DealDamageToBothTargetInstances { .. }
        | Effect::DealDamage { .. }
        | Effect::DealDamageWithoutPreventionOrRedirection { .. }
        | Effect::PreventNextDamageToAttached { .. }
        | Effect::DealDamageToMatchingTargets { .. }
        | Effect::DealDamageWithCappedLifeGain { .. }
        | Effect::DealDamageWithLimitedLifeGain { .. }
        | Effect::DealDamageEvenly { .. }
        | Effect::Fight { .. }
        | Effect::DamageEqualToPower { .. }
        | Effect::EventObjectDealsDamageEqualToPower { .. }
        | Effect::EventObjectDealsDamage { .. }
        | Effect::DealDamageEvenlyToFilter { .. }
        | Effect::DealDamageToTargetController { .. }
        | Effect::DealDamageToAttached { .. }
        | Effect::DealDamageDivided { .. }
        | Effect::DealDamageEach { .. }
        | Effect::GrantSpecialActionUntilEndOfTurn { .. }
        | Effect::RedirectNextDamage { .. }
        | Effect::LoseHalfLife { .. }
        | Effect::PreventNextDamage { .. }
        | Effect::PreventNextDamageFromTarget { .. }
        | Effect::PreventAllCombatDamageThisTurn
        | Effect::ChooseDamageSourcePreventAllThisTurn
        | Effect::ChooseDamageSourcePreventAllThisTurnMatching(_)
        | Effect::PreventNextFromChosenSource { .. }
        | Effect::RedirectNextFromChosenSource { .. }
        | Effect::DealDamageToEachOpponentAndPlaneswalkerDamagedThisGame { .. } => {
            life::exec(state, res, op)
        }
        Effect::Exile { .. }
        | Effect::Blink { .. }
        | Effect::ReturnToHand { .. }
        | Effect::ReturnAllToHand { .. }
        | Effect::ReturnAllOwnedToHand { .. }
        | Effect::DestroyAllWithManaValue { .. }
        | Effect::DestroyAll { .. }
        | Effect::ExileAll { .. }
        | Effect::ChooseYoursThen { .. }
        | Effect::DestroyOthersNamedLike { .. }
        | Effect::ExileGraveyard { .. }
        | Effect::GraveyardToHand { .. }
        | Effect::GraveyardAllToHand { .. }
        | Effect::GraveyardToTop { .. }
        | Effect::ReturnSourceFromGraveyard { .. }
        | Effect::GraveyardToBattlefieldLinked { .. }
        | Effect::GraveyardToBattlefield { .. }
        | Effect::Assimilate { .. }
        | Effect::PutSourceOnTopOfLibrary
        | Effect::BottomCardFromHand { .. }
        | Effect::GraveyardToLibraryBottom { .. }
        | Effect::PutSelfOntoBattlefield
        | Effect::PutSourceFromHandOntoBattlefield
        | Effect::ShuffleTargetsIntoOwnersLibraries
        | Effect::ShuffleGraveyardIntoLibrary
        | Effect::PhaseOut { .. }
        | Effect::PhaseOutUntilSourceLeaves { .. }
        | Effect::PhaseOutFilter(_)
        | Effect::ExileLinked { .. }
        | Effect::ExileTargetsWithSource
        | Effect::ExileChosenFromHandWithSource(_)
        | Effect::RevealLibrarySearch { .. }
        | Effect::SacrificeSelf
        | Effect::SacrificeAll { .. }
        | Effect::SacrificeObject { .. }
        | Effect::PutTargetOnTopOfLibrary
        | Effect::PutTargetOnBottomOfLibrary
        | Effect::RollDiePutTargetIntoLibraryBeneathTop { .. }
        | Effect::PutTargetIntoLibraryBeneathTop { .. }
        | Effect::PutOnBottomOfLibraryFromGraveyard { .. }
        | Effect::ExileSource
        | Effect::ExileAndReturnAtEndStep
        | Effect::ExileAndReturnTappedAtEndStep
        | Effect::ReturnExiledGroupToOwners { .. }
        | Effect::ExileLibraryAndShuffleHand { .. }
        | Effect::Mill { .. }
        | Effect::Destroy { .. }
        | Effect::Regenerate { .. }
        | Effect::RegenerateAll { .. }
        | Effect::DestroyChosenForPlayers { .. }
        | Effect::EqualizePermanents { .. }
        | Effect::EqualizeHands
        | Effect::DiscardForPlayers { .. }
        | Effect::DiscardAmountForPlayers { .. }
        | Effect::SacrificeAmountForPlayers { .. }
        | Effect::DiscardEachOrDraw { .. }
        | Effect::DiscardRandom { .. }
        | Effect::RandomGraveyardToHand { .. }
        | Effect::DiscardHand { .. }
        | Effect::ShuffleIntoLibrary { .. }
        | Effect::ShuffleLibrary { .. }
        | Effect::RevealHand { .. }
        | Effect::RevealHandDiscard { .. }
        | Effect::DiscardMatching { .. }
        | Effect::DiscardRandomMatching { .. }
        | Effect::LookAtChosenHand
        | Effect::LookAtRandomFromHand { .. }
        | Effect::LookHandDiscard { .. }
        | Effect::SacrificeFilter { .. }
        | Effect::SacrificeOneThen { .. }
        | Effect::ReturnChosenToHand { .. }
        | Effect::UntapChosen { .. }
        | Effect::AllGraveyardCreaturesToBattlefield
        | Effect::EachPlayerGraveyardToBattlefield { .. }
        | Effect::YourGraveyardToBattlefield { .. }
        | Effect::ReturnToBattlefieldTapped { .. }
        | Effect::TransformSource
        | Effect::TransformSourceAtNextUpkeep
        | Effect::ExileSelfReturnAsFace { .. }
        | Effect::ExileSourceWithCounters { .. }
        | Effect::MoveSourceToOwnerGraveyard
        | Effect::TawnossCoffinExile
        | Effect::ReturnTawnossCoffinExile
        | Effect::ReturnLinkedToBattlefield
        | Effect::ExileTargetsCreateTokens { .. }
        | Effect::CounterTargetAbility
        | Effect::CounterTargetSpellOrAbility
        | Effect::CounterTargetSpellToExile
        | Effect::CounterTargetSpell => zones::exec(state, res, op),
        Effect::DelayedManaAtNextMain { .. }
        | Effect::AddManaFor { .. }
        | Effect::AddManaAmountFor { .. }
        | Effect::ManaForColorsAmong(_)
        | Effect::AddManaLikeEvent { .. } => mana::exec(state, res, op),
        Effect::ModifySource { .. }
        | Effect::SetSourcePTFromTarget { .. }
        | Effect::SetSourceToughness { .. }
        | Effect::MoveCounter { .. }
        | Effect::MoveCounterToEvent { .. }
        | Effect::AddCounterSource { .. }
        | Effect::AddCounter { .. }
        | Effect::AddCountersUpTo { .. }
        | Effect::RemoveCounterSelf { .. }
        | Effect::RemoveAllCountersSelf { .. }
        | Effect::RemoveAllCountersFilter { .. }
        | Effect::RemoveAllCountersTarget
        | Effect::ChangeAttachedCounters { .. }
        | Effect::ModifyAttached { .. }
        | Effect::AddCounterFilter { .. }
        | Effect::DoubleCountersFilter { .. }
        | Effect::DrainAllCountersIntoSelf
        | Effect::SetPTFilter { .. }
        | Effect::ModifyFilter { .. }
        | Effect::PumpFilter { .. }
        | Effect::PumpFilterPerKeyword { .. }
        | Effect::PumpTarget { .. }
        | Effect::PumpTargetSlot { .. } => counters::exec(state, res, op),
        Effect::MarkLandWithCounter { .. }
        | Effect::ScheduleLinkedCounterCleanup { .. }
        | Effect::CleanLinkedCounters { .. } => linked_counters::exec(state, res, op),
        Effect::CreateTokenForTargetController { .. }
        | Effect::Amass { .. }
        | Effect::CreateTokenCopyOf { .. }
        | Effect::Populate
        | Effect::CreateTokenCopyOfFirstToken
        | Effect::CreateTokenCopyOfEquipped { .. }
        | Effect::CreateTokenCopyOfTarget { .. }
        | Effect::CreateTokenCopyOfSource { .. }
        | Effect::CreateTokensFor { .. }
        | Effect::CreateTokenN { .. }
        | Effect::CreateTokenModified { .. }
        | Effect::CreateTokenAttacking { .. }
        | Effect::CreateTokenThenCounters { .. }
        | Effect::Fabricate { .. }
        | Effect::CreateTokenThenAttachSelf { .. }
        | Effect::CreateTokenPtPerCount { .. }
        | Effect::CreateToken { .. }
        | Effect::CreateTokenFromLinked { .. } => tokens::exec(state, res, op),
        // --- Control ---------------------------------------------------
        Effect::ExchangeControlOrSacrifice => {
            let source = subjects::source(state, res)
                .filter(|source| subjects::on_battlefield(state, *source));
            let exchanged = source
                .zip(res.targets.first().copied())
                .is_some_and(|(source, target)| control::exchange(state, source.object, target));
            if !exchanged && let Some(source) = source {
                let _ = state.sacrifice_object(source.object, Cause::Effect);
            }
            None
        }
        Effect::ExchangeControl => {
            // Both legs use the same pre-exchange board; neither is applied on failure.
            if let (Some(&a), Some(&b)) = (res.targets.first(), res.second_targets.first()) {
                control::exchange(state, a, b);
            }
            None
        }
        Effect::WhenNextSpellCastThisTurn { filter, effects } => {
            bird_entry::register(state, res, filter, effects);
            None
        }
        Effect::EventSpellEntersWithP1P1 { n } => {
            bird_entry::grant(state, res, n);
            None
        }
        Effect::AddCombatMana { color, amount } => {
            mana::add_combat_mana(state, res, color, amount);
            None
        }
        Effect::ChangeController { new_controller } => control::change(state, res, new_controller),
        Effect::ControlPlayerNextTurn { player } => {
            control::next_turn(state, res, player);
            None
        }
        Effect::ControlRotation => control::ask(state, res),
        Effect::AllCreaturesToOwner => {
            let creatures: Vec<ObjectId> = state
                .battlefield_seen()
                .filter(|id| {
                    state.object(*id).is_some_and(|o| {
                        o.characteristics()
                            .types
                            .contains(baylee_core::types::TypeSet::CREATURE)
                    })
                })
                .collect();
            let changes: Vec<(ObjectId, PlayerId)> = creatures
                .into_iter()
                .filter_map(|id| state.object(id).map(|o| (id, o.owner)))
                .collect();
            gain_control(state, &changes);
            None
        }
        // --- Cards drawn -------------------------------------------------
        Effect::PutHandOnTop { player } => hand_transfer::all_to_top(state, res, player),
        Effect::SearchLibraryFor {
            library,
            receiver,
            count,
            filter,
            find,
            optional,
        } => directed_search::start(state, res, library, receiver, count, filter, find, optional),
        Effect::RechooseGraveyardCard {
            owner,
            chooser,
            cost,
        } => repeated_graveyard_choice::start(state, res, owner, chooser, cost),
        Effect::DrawCards { amount } => {
            let n = amount2(&amount, state, you, res) as usize;
            state.draw_cards(you, n);
            None
        }
        Effect::LookAtTopPickThenReorder { count, pick } => {
            private_library::pick_top(state, res, count, pick)
        }
        Effect::LookAtTopExileRandom { count, exile } => {
            private_library::exile_random_top(state, res, count, exile)
        }
        Effect::PutFromHandOnTopFor { player, count } => {
            private_library::hand_on_top(state, res, player, count)
        }
        Effect::ExileTop { who, count } => names::exile_top(state, res, who, count),
        Effect::RevealUntilNamed {
            player,
            matched,
            otherwise,
        } => names::reveal_until_named(state, res, player, matched, otherwise),
        Effect::RevealUntilMatching {
            player,
            filter,
            matched,
            otherwise,
        } => {
            if let Some(player) = players_of(player, state, you, res).first().copied() {
                search::reveal_until_matching(
                    state, player, res.source, filter, matched, otherwise,
                );
            }
            None
        }
        Effect::RevealTopByNameThen {
            player,
            matched,
            otherwise,
            otherwise_effects,
        } => names::reveal_top_then(state, res, player, matched, otherwise, otherwise_effects),
        Effect::DrawRevealDiscardUnless { keep } => {
            let card = if let Some((drawn_before, replaced_before)) = res.draw_watch.take() {
                // The draw waited for its replacements and is settled now.
                // A card a replacement brought is not "it" (CR 614.11b).
                if state.draws_replaced != replaced_before {
                    return None;
                }
                state
                    .per_turn
                    .drawn
                    .get(drawn_before)
                    .map(|&(card, _)| card)
                    .filter(|&card| {
                        state
                            .object(card)
                            .is_some_and(|o| o.owner == you && o.zone == crate::zone::Zone::Hand)
                    })?
            } else {
                let watch = (state.per_turn.drawn.len(), state.draws_replaced);
                let drawn = state.draw_cards(you, 1);
                if drawn.is_empty() && !state.draws_to_offer.is_empty() {
                    // Run again, with the answer, once the draw is settled.
                    res.draw_watch = Some(watch);
                    return None;
                }
                *drawn.first()?
            };
            state.journal.record(GameEvent::Revealed {
                player: you,
                cards: vec![card],
            });
            let fits = state.object(card).is_some_and(|o| {
                eval::matches_with_context(keep, state, o, you, res.rule_context())
            });
            if fits {
                return None;
            }
            super::discard::discard_or_ask(state, res, &[(card, you)])
        }
        Effect::Connive { amount } => loot::start(state, res, amount, None),
        Effect::Recruit { token } => loot::start(state, res, Amount::Fixed(1), Some(token)),
        Effect::DrawCardsFor { amount, who } => {
            // Every count is read before anybody draws: a draw is a later
            // event than the instruction's own numbers (CR 608.2c).
            let draws: Vec<(PlayerId, usize)> = players_of(who, state, you, res)
                .into_iter()
                .map(|player| {
                    let n = super::amount_for_player(&amount, state, you, res, player);
                    (player, n as usize)
                })
                .collect();
            for (player, n) in draws {
                state.draw_cards(player, n);
            }
            None
        }
        // --- Conditional branches ---------------------------------------
        Effect::IfCastFromGraveyard { effects } => {
            if super::copying::was_cast_from_graveyard(state, res) {
                run_nested(state, res, effects)
            } else {
                None
            }
        }
        Effect::CopyResolvingSpell { controller } => {
            super::copying::copy_resolving(state, res, controller)
        }
        Effect::ExileSourceThenCastTransformed { cast } => {
            super::battle_defeat::start(state, res, cast)
        }
        Effect::PreventNextDamageFromSource { .. }
        | Effect::RedirectCombatDamageToYouThisTurn { .. } => {
            life::ice_combat::exec(state, res, op)
        }
        Effect::RedirectTargetSourceDamageThisTurn { .. }
        | Effect::RedirectNextChosenSourceDamageToTarget { .. }
        | Effect::ReflectNextChosenSourceDamage
        | Effect::PreventDamageFromTargetingCausesThisTurn { .. } => {
            life::batch57::exec(state, res, op)
        }
        Effect::IfKicked { then, otherwise } => {
            let kicked = state.object(res.on_stack).is_some_and(|o| o.kicked);
            let branch = if kicked { then } else { otherwise };
            run_nested(state, res, branch)
        }
        // The first target as it is now; a target that is gone was dropped
        // by CR 608.2b before anything here ran.
        Effect::IfTargetHadType { types, then } => {
            if res.targets.first().is_some_and(|first| {
                res.target_lki.as_ref().is_some_and(|targets| {
                    targets
                        .iter()
                        .any(|t| t.id == *first && t.chars.types.contains(types))
                })
            }) {
                return run_nested(state, res, then);
            }
            None
        }
        Effect::IfTargetMatchesElse {
            filter,
            then,
            otherwise,
        } => {
            let holds = this_to_affect(state, res)
                .and_then(|id| state.object(id))
                .is_some_and(|object| {
                    eval::matches_with_context(filter, state, object, you, res.rule_context())
                });
            run_nested(state, res, if holds { then } else { otherwise })
        }
        Effect::IfTargetMatches { filter, then } => {
            let holds = res
                .targets
                .first()
                .and_then(|&t| state.object(t))
                .is_some_and(|o| {
                    eval::matches_with_context(filter, state, o, you, res.rule_context())
                });
            if holds {
                return run_nested(state, res, then);
            }
            None
        }
        // CR 603.7c: an event object that has left its zone is none here,
        // and nothing about it holds.
        Effect::IfEventObjectMatches { filter, then } => {
            let holds = res
                .event_object
                .and_then(|t| state.object(t))
                .is_some_and(|o| {
                    eval::matches_with_context(filter, state, o, you, res.rule_context())
                });
            if holds {
                return run_nested(state, res, then);
            }
            None
        }
        Effect::IfCreaturesDiedAtLeast { n, then } => {
            if state.per_turn.creatures_died >= n {
                return run_nested(state, res, then);
            }
            None
        }
        Effect::IfResolvedTimesThisTurn { times, then } => {
            // The ability resolving is the stack object's; its count was
            // taken as it began to resolve (`resolve_stack_top`).
            let resolved = state
                .object(res.on_stack)
                .and_then(|o| o.ability)
                .map_or(0, |loc| {
                    let version = state.object(loc.source).map_or(0, |o| o.version);
                    state.per_turn.resolutions(loc.source, version, loc.index)
                });
            if resolved == times {
                return run_nested(state, res, then);
            }
            None
        }
        Effect::IfActivatedThisTurnAtLeast { n, then } => {
            // Counted as the ability was activated (CR 602.2, the
            // activation in `engine/abilities.rs`), this one included, in
            // the turn's tally of that ability of that object — which a
            // source retains even after leaving (CR 608.2h–i).
            let version = source_version(state, res)
                .or_else(|| state.object(res.source).map(|o| o.version))
                .unwrap_or(0);
            let activated = state
                .object(res.on_stack)
                .and_then(|o| o.ability)
                .and_then(|loc| {
                    state
                        .ability_fires
                        .get(&(
                            baylee_core::ids::DamageSourceRef {
                                object: loc.source,
                                version: source_version(state, res).unwrap_or_else(|| {
                                    state.object(loc.source).expect("ability source").version
                                }),
                            },
                            loc.index,
                        ))
                        .copied()
                })
                .or_else(|| {
                    (res.on_stack == res.source).then(|| {
                        state
                            .ability_fires
                            .iter()
                            .filter_map(|((src, _), &count)| {
                                (src.object == res.source && src.version == version)
                                    .then_some(count)
                            })
                            .max()
                            .unwrap_or(0)
                    })
                })
                .unwrap_or(0);
            if activated >= u32::from(n) {
                return run_nested(state, res, then);
            }
            None
        }
        Effect::IfNotLostLifeThisTurn { then } => {
            // Set by `GameState::change_life` for every loss, whether it
            // came from damage, an effect or a payment, and cleared at every
            // turn start.
            if !state.per_turn.life_lost[you.get() as usize] {
                return run_nested(state, res, then);
            }
            None
        }
        Effect::IfControlGreatestCmc { filter, then } => {
            // Greatest cmc among filter-matching permanents; condition
            // holds when you control one of them (Padeem).
            let mut greatest = 0u32;
            let mut holds = false;
            for id in state.battlefield_seen() {
                let Some(obj) = state.object(id) else {
                    continue;
                };
                if !eval::matches_with_context(filter, state, obj, you, res.rule_context()) {
                    continue;
                }
                let cmc = obj.characteristics().mana_value();
                if cmc > greatest {
                    greatest = cmc;
                    holds = obj.controller == you;
                } else if cmc == greatest && obj.controller == you {
                    holds = true;
                }
            }
            if holds {
                return run_nested(state, res, then);
            }
            None
        }
        Effect::IfNoCountersOnSelf { kind, then } => {
            // `is_some_and` and not `map_or(true, …)`: a source that is no
            // longer on the battlefield or in exile has not run out of
            // counters, it has stopped being a thing the sentence is about.
            if subjects::source(state, res).is_some_and(|r| subjects::is_current(state, r))
                && state
                    .object(res.source)
                    .is_some_and(|o| o.counters.get(kind) == 0)
            {
                return run_nested(state, res, then);
            }
            None
        }
        Effect::ExileIfDiesThisTurn { target } => {
            for id in zones::spec_objects(state, res, target) {
                if let Some(obj) = state.object(id)
                    && obj.zone == crate::zone::Zone::Battlefield
                {
                    let named = (id, obj.version);
                    if !state.per_turn.exile_if_dies.contains(&named) {
                        state.per_turn.exile_if_dies.push(named);
                    }
                }
            }
            None
        }
        // The delayed trigger retains the exact subject of this resolution.
        // An ability that never said "target" is about its source
        // ("sacrifice this creature", Dragon Whelp); one that targeted
        // and has no object target left is about nothing.
        Effect::WhenTargetDiesThisTurn { effects } => {
            let &card = res.targets.first()?;
            let version = state
                .object(card)
                .filter(|o| o.zone == crate::zone::Zone::Battlefield)?
                .version;
            let after = state.journal.last_seq();
            state.delayed.push(crate::state::DelayedTrigger {
                controller: you,
                when: crate::state::DelayedWhen::DiesThisTurn {
                    card,
                    version,
                    after,
                },
                action: crate::state::DelayedAction::Trigger {
                    source: res.source,
                    source_version: source_version(state, res)
                        .or_else(|| state.object(res.source).map(|o| o.version))
                        .unwrap_or(0),
                    effects,
                    text: res.text,
                },
            });
            None
        }
        Effect::ChooseColors {
            subject,
            max,
            duration,
        } => colors::choose(state, res, subject, max, duration),
        Effect::ChooseSourceLandwalk { duration } => Some(landwalk::choose(
            state,
            res,
            TargetSpec::ThisObject,
            true,
            &baylee_cards_dsl::Filter::Any,
            duration,
        )),
        Effect::ChooseLandwalk {
            target,
            basic_only,
            lands,
            duration,
        } => Some(landwalk::choose(
            state, res, target, basic_only, lands, duration,
        )),
        Effect::AtNextUpkeep { effects } => {
            state.delayed.push(crate::state::DelayedTrigger {
                controller: you,
                when: crate::state::DelayedWhen::NextUpkeep,
                action: crate::state::DelayedAction::Trigger {
                    source: res.source,
                    source_version: source_version(state, res)
                        .or_else(|| state.object(res.source).map(|o| o.version))
                        .unwrap_or(0),
                    effects,
                    text: res.text,
                },
            });
            None
        }
        Effect::AtNextTurnsUpkeep { effects } => {
            state.delayed.push(crate::state::DelayedTrigger {
                controller: you,
                when: crate::state::DelayedWhen::NextUpkeepOfAnyone,
                action: crate::state::DelayedAction::Trigger {
                    source: res.source,
                    source_version: source_version(state, res)
                        .or_else(|| state.object(res.source).map(|o| o.version))
                        .unwrap_or(0),
                    effects,
                    text: res.text,
                },
            });
            None
        }
        Effect::ReturnSelfDuringNextUntap => {
            super::delayed_returns::schedule_untap(state, res);
            None
        }
        Effect::AtNextEndStepForCreatedTokens { effects } => {
            let source_version = source_version(state, res)
                .or_else(|| state.object(res.source).map(|object| object.version))
                .unwrap_or(0);
            for token in super::this_way::created_tokens(state, res) {
                if let Some(version) = state
                    .object(token)
                    .filter(|object| object.zone == crate::zone::Zone::Battlefield)
                    .map(|object| object.version)
                {
                    state.delayed.push(crate::state::DelayedTrigger {
                        controller: you,
                        when: crate::state::DelayedWhen::NextEndStep,
                        action: crate::state::DelayedAction::TriggerAbout {
                            source: res.source,
                            source_version,
                            effects,
                            text: res.text,
                            object: token,
                            version,
                            subject: None,
                        },
                    });
                }
            }
            None
        }
        Effect::AtNextEndStepAboutSubject { effects } => {
            if let Some(subject) = subjects::event_subject(state, res) {
                state.delayed.push(crate::state::DelayedTrigger {
                    controller: you,
                    when: crate::state::DelayedWhen::NextEndStep,
                    action: crate::state::DelayedAction::TriggerAbout {
                        source: res.source,
                        source_version: source_version(state, res)
                            .or_else(|| state.object(res.source).map(|o| o.version))
                            .unwrap_or(0),
                        effects,
                        text: res.text,
                        object: subject.object,
                        version: subject.version,
                        subject: None,
                    },
                });
            }
            None
        }
        Effect::AtNextEndStep { effects } | Effect::AtNextCleanup { effects } => {
            // A source's own departure trigger may find its exact public-zone
            // successor (CR 400.7e). Remember that incarnation for the delayed
            // action (CR 603.7c), not the former battlefield incarnation.
            // Explicit targets retain their own identity rules.
            let about_ref = if res.targets.is_empty() && !res.targeted {
                subjects::moving_source(state, res).or_else(|| subjects::source(state, res))
            } else {
                subjects::this(state, res)
            };
            let action = match about_ref.map(|r| (r.object, r.version)) {
                Some((object, version)) => crate::state::DelayedAction::TriggerAbout {
                    source: res.source,
                    source_version: source_version(state, res)
                        .or_else(|| state.object(res.source).map(|o| o.version))
                        .unwrap_or(0),
                    effects,
                    text: res.text,
                    object,
                    version,
                    subject: subjects::event_subject(state, res),
                },
                None => crate::state::DelayedAction::Trigger {
                    source: res.source,
                    source_version: source_version(state, res)
                        .or_else(|| state.object(res.source).map(|o| o.version))
                        .unwrap_or(0),
                    effects,
                    text: res.text,
                },
            };
            state.delayed.push(crate::state::DelayedTrigger {
                controller: you,
                when: if matches!(op, Effect::AtNextCleanup { .. }) {
                    crate::state::DelayedWhen::NextCleanup
                } else {
                    crate::state::DelayedWhen::NextEndStep
                },
                action,
            });
            None
        }
        // The delayed trigger is about the object `about` names as this
        // resolves, as the object it is now (CR 603.7c); naming nothing, it
        // is about nothing.
        Effect::AtEndOfCombat { about, effects }
        | Effect::AtEndOfCombatThisTurn { about, effects } => {
            let when = if matches!(op, Effect::AtEndOfCombatThisTurn { .. }) {
                crate::state::DelayedWhen::EndOfCombatThisTurn
            } else {
                crate::state::DelayedWhen::EndOfCombat
            };
            let action = match zones::spec_object(state, res, about)
                .and_then(|t| state.object(t).map(|o| (t, o.version)))
            {
                Some((object, version)) => crate::state::DelayedAction::TriggerAbout {
                    source: res.source,
                    source_version: source_version(state, res)
                        .or_else(|| state.object(res.source).map(|o| o.version))
                        .unwrap_or(0),
                    effects,
                    text: res.text,
                    object,
                    version,
                    subject: subjects::event_subject(state, res),
                },
                None => crate::state::DelayedAction::Trigger {
                    source: res.source,
                    source_version: source_version(state, res)
                        .or_else(|| state.object(res.source).map(|o| o.version))
                        .unwrap_or(0),
                    effects,
                    text: res.text,
                },
            };
            state.delayed.push(crate::state::DelayedTrigger {
                controller: you,
                when,
                action,
            });
            None
        }
        Effect::DamagedThisWayCantBeRegenerated => {
            super::regeneration::prevent_for_damaged(state, res);
            None
        }
        Effect::CantBeRegeneratedThisTurn { target } => {
            super::regeneration::prevent_for_targets(state, res, target);
            None
        }
        Effect::Discover { mana_value } => discover(state, res, u32::from(mana_value)),
        Effect::DiscoverAmount { mana_value } => {
            discover(state, res, amount2(&mana_value, state, you, res))
        }
        Effect::DiscardAnyThenDraw => super::discard_any::offer(state, res),
        Effect::ReturnFromGraveyardOf { whose, filter } => {
            let owner = players_of(whose, state, you, res).first().copied()?;
            let options: Vec<ObjectId> = state
                .zones
                .list(ZoneLocation::Graveyard(owner))
                .iter()
                .copied()
                .filter(|id| {
                    state.object(*id).is_some_and(|o| {
                        eval::matches_with_context(filter, state, o, you, res.rule_context())
                    })
                })
                .collect();
            if options.is_empty() {
                return None;
            }
            res.awaiting = Some(AwaitingOp::ReturnFromGraveyardOf);
            Some(Pending::ChooseCards {
                player: you,
                options,
                min: 1,
                max: 1,
                prompt: ChoicePrompt::FromGraveyard,
                total: None,
            })
        }
        // As `WhenTargetDiesThisTurn`, about the source.
        Effect::WhenSourceDiesThisTurn { effects } => {
            let source = subjects::source(state, res)
                .filter(|source| subjects::on_battlefield(state, *source))?;
            let after = state.journal.last_seq();
            state.delayed.push(crate::state::DelayedTrigger {
                controller: you,
                when: crate::state::DelayedWhen::DiesThisTurn {
                    card: source.object,
                    version: source.version,
                    after,
                },
                action: crate::state::DelayedAction::Trigger {
                    source: res.source,
                    source_version: source.version,
                    effects,
                    text: res.text,
                },
            });
            None
        }
        Effect::DestroyTargetSlots { targets, no_regen } => {
            super::target_groups::destroy(state, res, targets, no_regen);
            None
        }
        Effect::DestroyRandom { filter } => {
            let options: Vec<ObjectId> = state
                .battlefield_seen()
                .filter(|id| {
                    state.object(*id).is_some_and(|o| {
                        eval::matches_with_context(filter, state, o, you, res.rule_context())
                    })
                })
                .collect();
            if options.is_empty() {
                return None;
            }
            let picked = options[state.rng.below(options.len() as u64) as usize];
            sba::destroy(state, picked);
            None
        }
        Effect::RevealTopPutOntoBattlefield { count, picks } => {
            let n = amount2(&count, state, you, res);
            super::reveal_put::start(state, res, n, picks)
        }
        // Sword of the Ages: "then exile this artifact and those creature
        // cards", each from where its cost put it and only while it is still
        // that card there (CR 400.7).
        Effect::ExileSourceAndSacrificed => {
            let paid = state.object(res.on_stack).and_then(|o| o.paid.clone());
            let mut cards: Vec<(ObjectId, u32)> = paid
                .as_ref()
                .and_then(|p| p.source_after_cost)
                .map(|r| (r.object, r.version))
                .into_iter()
                .collect();
            if let Some(paid) = &paid {
                cards.extend(
                    paid.sacrificed_many
                        .iter()
                        .map(|(id, version, _)| (*id, *version)),
                );
            }
            for (id, version) in cards {
                let Some(owner) = state
                    .object(id)
                    .filter(|o| o.version == version && o.zone == crate::zone::Zone::Graveyard)
                    .map(|o| o.owner)
                else {
                    continue;
                };
                let _ = state.move_object(
                    id,
                    ZoneLocation::Exile(owner),
                    crate::zone::ZonePosition::Top,
                    Cause::Effect,
                );
            }
            None
        }
        Effect::NextDrawLooksAtTop { count } => {
            // X is read as the ability resolves and kept with the effect,
            // which waits for the controller's next draw this turn.
            let count = amount2(&count, state, you, res);
            let timestamp = state.next_timestamp();
            let modifier = baylee_cards_dsl::Modifier::NextDrawFromTop { count };
            state.effects.register(crate::effects::ContinuousEffect {
                id: baylee_core::ids::EffectId::new(0),
                source: subjects::source(state, res).map(|source| source.object),
                controller: you,
                origin: crate::effects::EffectOrigin::Resolution,
                layer: modifier.layer(),
                timestamp,
                duration: baylee_cards_dsl::Duration::UntilEndOfTurn,
                filter: crate::effects::EffectFilter::Dsl(&baylee_cards_dsl::Filter::Any),
                modifier,
            });
            None
        }
        Effect::NextSpellThisTurnHasImprovise => {
            if let Some(flag) = state
                .per_turn
                .improvise_next_spell
                .get_mut(usize::from(you.get()))
            {
                *flag = true;
            }
            None
        }
        Effect::WheneverYouAttackThisTurn { effects } => {
            let after = state.journal.last_seq();
            state.delayed.push(crate::state::DelayedTrigger {
                controller: you,
                when: crate::state::DelayedWhen::YouAttackThisTurn { after },
                action: crate::state::DelayedAction::Trigger {
                    source: res.source,
                    source_version: source_version(state, res)
                        .or_else(|| state.object(res.source).map(|o| o.version))
                        .unwrap_or(0),
                    effects,
                    text: res.text,
                },
            });
            None
        }
        Effect::WheneverTappedForManaThisTurn {
            by,
            filter,
            effects,
        } => {
            let after = state.journal.last_seq();
            state.delayed.push(crate::state::DelayedTrigger {
                controller: you,
                when: crate::state::DelayedWhen::TappedForManaThisTurn { by, filter, after },
                action: crate::state::DelayedAction::Trigger {
                    source: res.source,
                    source_version: source_version(state, res)
                        .or_else(|| state.object(res.source).map(|o| o.version))
                        .unwrap_or(0),
                    effects,
                    text: res.text,
                },
            });
            None
        }
        Effect::WhenTargetAttacksAndIsntBlockedThisTurn { effects } => {
            let &card = res.targets.first()?;
            let version = state
                .object(card)
                .filter(|o| o.zone == crate::zone::Zone::Battlefield)?
                .version;
            let after = state.journal.last_seq();
            state.delayed.push(crate::state::DelayedTrigger {
                controller: you,
                when: crate::state::DelayedWhen::AttacksAndIsntBlockedThisTurn {
                    card,
                    version,
                    after,
                },
                action: crate::state::DelayedAction::Trigger {
                    source: res.source,
                    source_version: source_version(state, res)
                        .or_else(|| state.object(res.source).map(|o| o.version))
                        .unwrap_or(0),
                    effects,
                    text: res.text,
                },
            });
            None
        }
        Effect::DestroyAllThenForEachDestroyed {
            filter,
            no_regen,
            effects,
        } => {
            super::destruction_receipts::execute(state, res, filter, no_regen, effects, op);
            None
        }
        Effect::WheneverCombatEventThisTurn { .. } => {
            combat_watches::start(state, res, op);
            None
        }
        Effect::ForEachMatching { effects, .. } | Effect::ForEachTarget { effects } => {
            let mut remaining = res.subject.for_each_remaining.take().unwrap_or_else(|| {
                let context = res.rule_context();
                if matches!(op, Effect::ForEachTarget { .. }) {
                    let mut targets: Vec<_> = res
                        .targets
                        .iter()
                        .filter_map(|id| subjects::target(state, res, *id))
                        .collect();
                    targets.reverse();
                    return targets;
                }
                let Effect::ForEachMatching { filter, .. } = op else {
                    unreachable!("iteration instruction");
                };
                let mut matched: Vec<baylee_core::ids::DamageSourceRef> = state
                    .battlefield_seen()
                    .filter_map(|id| {
                        let o = state.object(id)?;
                        eval::matches_with_context(filter, state, o, you, context).then_some(
                            baylee_core::ids::DamageSourceRef {
                                object: id,
                                version: o.version,
                            },
                        )
                    })
                    .collect();
                matched.reverse();
                matched
            });
            while let Some(next_ref) = remaining.pop() {
                let Some(obj) = state.object(next_ref.object).filter(|o| {
                    o.zone == crate::zone::Zone::Battlefield && o.version == next_ref.version
                }) else {
                    continue;
                };
                let controller = obj.controller;
                let power = obj.characteristics().power.unwrap_or(0);
                res.event_object = Some(next_ref.object);
                res.subject.set_event_identity(Some((next_ref, power)));
                res.subject.set_event_player(Some(controller));
                let mut spliced = flatten(effects);
                if !remaining.is_empty() {
                    res.subject.for_each_remaining = Some(remaining);
                    spliced.push(op);
                }
                let at = res.pc + 1;
                res.effects.splice(at..at, spliced);
                return None;
            }
            None
        }
        Effect::WheneverOneOrMoreDealCombatDamageToPlayerThisTurn { filter, effects } => {
            let after = state.journal.last_seq();
            state.delayed.push(crate::state::DelayedTrigger {
                controller: you,
                when: crate::state::DelayedWhen::OneOrMoreDealCombatDamageToPlayerThisTurn {
                    filter,
                    after,
                },
                action: crate::state::DelayedAction::Trigger {
                    source: res.source,
                    source_version: source_version(state, res)
                        .or_else(|| state.object(res.source).map(|o| o.version))
                        .unwrap_or(0),
                    effects,
                    text: res.text,
                },
            });
            None
        }
        Effect::WheneverCreatureAttacksYouOrYourPlaneswalkerUntilYourNextTurn { effects } => {
            let after = state.journal.last_seq();
            state.delayed.push(crate::state::DelayedTrigger {
                controller: you,
                when:
                    crate::state::DelayedWhen::CreatureAttacksYouOrYourPlaneswalkerUntilYourNextTurn {
                        after,
                    },
                action: crate::state::DelayedAction::Trigger {
                    source: res.source,
                    source_version: source_version(state, res)
                        .or_else(|| state.object(res.source).map(|o| o.version))
                        .unwrap_or(0),
                    effects,
                    text: res.text,
                },
            });
            None
        }
        Effect::WheneverTargetDealtDamageThisTurn { by, effects } => {
            let &card = res.targets.first()?;
            let version = state
                .object(card)
                .filter(|o| o.zone == crate::zone::Zone::Battlefield)?
                .version;
            let after = state.journal.last_seq();
            state.delayed.push(crate::state::DelayedTrigger {
                controller: you,
                when: crate::state::DelayedWhen::DealtDamageThisTurn {
                    card,
                    version,
                    by,
                    after,
                },
                action: crate::state::DelayedAction::Trigger {
                    source: res.source,
                    source_version: source_version(state, res)
                        .or_else(|| state.object(res.source).map(|o| o.version))
                        .unwrap_or(0),
                    effects,
                    text: res.text,
                },
            });
            None
        }
        Effect::DealHalfOfSpellDamage { spell } => super::half_damage::start(state, res, spell),
        Effect::ReturnPerBlockedDeath { filter } => {
            super::blocked_returns::start(state, res, filter)
        }
        Effect::MoveTargetAuraToAnother { types } => {
            let &aura = res.targets.first()?;
            let host = state.object(aura).and_then(|o| o.attached_to)?;
            let host_types = state
                .object(host)
                .map(|o| o.characteristics().types.intersection(types))
                .unwrap_or_default();
            if host_types.is_empty() {
                return None;
            }
            let options: Vec<ObjectId> = state
                .battlefield_seen()
                .filter(|&id| {
                    id != host
                        && id != aura
                        && state
                            .object(id)
                            .is_some_and(|o| o.characteristics().types.intersects(host_types))
                        && crate::aura_bindings::can_attach(state, aura, id)
                })
                .collect();
            if options.is_empty() {
                return None;
            }
            res.awaiting = Some(AwaitingOp::MoveAura { aura });
            Some(Pending::ChooseCards {
                player: you,
                options,
                min: 1,
                max: 1,
                prompt: ChoicePrompt::Generic,
                total: None,
            })
        }
        // CR 701.12c, all or nothing (701.12a): each side's change must be
        // one its player's life can make.
        Effect::ExchangeLifeTotals { with } => {
            let other = players_of(with, state, you, res).first().copied()?;
            if other == you {
                return None;
            }
            let mine = state.players[usize::from(you.get())].life;
            let theirs = state.players[usize::from(other.get())].life;
            let can = |player: PlayerId, delta: i32| {
                delta == 0
                    || (!state.life_total_cant_change(player)
                        && (delta > 0 || !state.cant_lose_life(player)))
            };
            if can(you, theirs - mine) && can(other, mine - theirs) {
                state.change_life(you, theirs - mine, Cause::Effect);
                state.change_life(other, mine - theirs, Cause::Effect);
            }
            None
        }
        Effect::GainControlAll { filter, to } => {
            let player = players_of(to, state, you, res).first().copied()?;
            let changes: Vec<(ObjectId, PlayerId)> =
                if matches!(filter, baylee_cards_dsl::Filter::This) {
                    subjects::source(state, res)
                        .filter(|s| subjects::on_battlefield(state, *s))
                        .map(|s| (s.object, player))
                        .into_iter()
                        .collect()
                } else {
                    state
                        .battlefield_seen()
                        .filter(|id| {
                            state.object(*id).is_some_and(|o| {
                                eval::matches_with_context(
                                    filter,
                                    state,
                                    o,
                                    you,
                                    res.rule_context(),
                                )
                            })
                        })
                        .map(|id| (id, player))
                        .collect()
                };
            super::gain_control(state, &changes);
            None
        }
        Effect::DestroyAurasOnTargets => {
            let hosts: Vec<ObjectId> = res
                .targets
                .iter()
                .chain(res.second_targets.iter())
                .copied()
                .collect();
            let auras: Vec<ObjectId> = state
                .battlefield_seen()
                .filter(|id| {
                    state.object(*id).is_some_and(|o| {
                        o.attached_to.is_some_and(|h| hosts.contains(&h))
                            && o.characteristics()
                                .subtypes
                                .contains(baylee_core::generated::subtypes::enchantment::AURA)
                    })
                })
                .collect();
            sba::destroy_all(state, &auras, false);
            None
        }
        Effect::ChoosePlayerFrom { among } => {
            let options = players_of(among, state, you, res);
            match options.as_slice() {
                [] => {
                    res.chosen_player = None;
                    None
                }
                [only] => {
                    res.chosen_player = Some(*only);
                    None
                }
                _ => {
                    res.awaiting = Some(AwaitingOp::ChoosePlayerFrom);
                    Some(Pending::ChoosePlayer {
                        player: you,
                        options,
                    })
                }
            }
        }
        // The player as this resolves; each target, until end of turn.
        Effect::CantBeBlockedByPlayerThisTurn { by } => {
            let player = players_of(by, state, you, res).first().copied()?;
            let modifier = baylee_cards_dsl::Modifier::CantBeBlockedByPlayer(player);
            let timestamp = state.next_timestamp();
            for target in res.targets.clone() {
                let Some(obj) = state.object(target) else {
                    continue;
                };
                if obj.zone != crate::zone::Zone::Battlefield {
                    continue;
                }
                let filter = crate::effects::EffectFilter::object(state, target);
                state.effects.register(crate::effects::ContinuousEffect {
                    id: baylee_core::ids::EffectId::new(0),
                    source: Some(res.source),
                    controller: you,
                    origin: crate::effects::EffectOrigin::Resolution,
                    layer: modifier.layer(),
                    timestamp,
                    duration: baylee_cards_dsl::Duration::UntilEndOfTurn,
                    filter,
                    modifier,
                });
            }
            None
        }
        Effect::DestroyAllExceptChosen { filter, keep } => {
            let options: Vec<ObjectId> = state
                .battlefield_seen()
                .filter(|id| {
                    state.object(*id).is_some_and(|o| {
                        eval::matches_with_context(filter, state, o, you, res.rule_context())
                    })
                })
                .collect();
            if options.is_empty() {
                return None;
            }
            let max = u8::try_from(options.len()).unwrap_or(u8::MAX).min(keep);
            res.awaiting = Some(AwaitingOp::DestroyAllExcept { filter });
            Some(Pending::ChooseCards {
                player: you,
                options,
                min: 0,
                max,
                prompt: ChoicePrompt::Keep,
                total: None,
            })
        }
        Effect::DiscardCount { who, count } => {
            let n = u8::try_from(amount2(&count, state, you, res)).unwrap_or(u8::MAX);
            super::discard::start_chain(state, res, who, n, false)
        }
        Effect::Repeat { times, effects } => {
            // Read once, as the instruction begins (CR 608.2c). Bounded so a
            // nonsense count is a number and not an allocation.
            let n = amount2(&times, state, you, res).min(REPEAT_CAP);
            let once = flatten(effects);
            let mut program = Vec::with_capacity(once.len() * n as usize);
            for _ in 0..n {
                program.extend_from_slice(&once);
            }
            if program.is_empty() {
                return None;
            }
            let targets = res.targets.clone();
            super::run_nested_with(state, res, program, targets)
        }
        // One pass, then the same op again as its check: the pass is spliced
        // into this program (so a question inside it suspends and resumes
        // like any other), and `this_way_from` marks where it began, which
        // is how the op tells a check from a first entry.
        Effect::RepeatWhile { effects, condition } => {
            let again = match res.this_way_from {
                None => true,
                Some(_) => resolution_condition(state, res, condition),
            };
            if again {
                res.this_way_from = Some(state.journal.len());
                let mut pass = flatten(effects);
                pass.push(op);
                let next = res.pc + 1;
                res.effects.splice(next..next, pass);
            } else {
                res.this_way_from = None;
            }
            None
        }
        Effect::PutAnyFromHandOnBottom => {
            let hand = state.zones.list(ZoneLocation::Hand(you)).clone();
            if hand.is_empty() || state.has_left(you) {
                return None;
            }
            let max = u8::try_from(hand.len()).unwrap_or(u8::MAX);
            res.awaiting = Some(AwaitingOp::HandToBottom);
            Some(Pending::ChooseCards {
                player: you,
                options: hand,
                min: 0,
                max,
                prompt: ChoicePrompt::PutOnBottom,
                total: None,
            })
        }
        Effect::ExileDamagedThisWayIfDies => {
            for id in super::this_way::damaged_permanents(state, res) {
                if let Some(obj) = state.object(id)
                    && obj.zone == crate::zone::Zone::Battlefield
                {
                    let named = (id, obj.version);
                    if !state.per_turn.exile_if_dies.contains(&named) {
                        state.per_turn.exile_if_dies.push(named);
                    }
                }
            }
            None
        }
        Effect::NthResolutionThisTurn { effects } => {
            // This resolution is the ability's nth this turn, counted in the
            // turn's per-ability tally. A spell has no ability to count.
            let key = state
                .object(res.on_stack)
                .and_then(|o| o.ability)
                .map(|loc| {
                    (
                        baylee_core::ids::DamageSourceRef {
                            object: loc.source,
                            version: source_version(state, res).unwrap_or_else(|| {
                                state.object(loc.source).expect("ability source").version
                            }),
                        },
                        loc.index,
                    )
                })?;
            let nth = state.ability_fires.get(&key).copied().unwrap_or(0) + 1;
            state.ability_fires.insert(key, nth);
            let index = usize::try_from(nth - 1).ok()?;
            let this_time = effects.get(index..=index)?;
            run_nested(state, res, this_time)
        }
        // The seat is the ability's controller and the source is its
        // object, which is the same pair `condition_holds` is handed at an
        // activation gate and at an intervening `if` — one reader, so a
        // card cannot mean two different things by one sentence depending
        // on where it printed it.
        Effect::IfCondition {
            condition,
            then,
            otherwise,
        } => {
            let branch = if resolution_condition(state, res, condition) {
                then
            } else {
                otherwise
            };
            run_nested(state, res, branch)
        }
        // CR 705.1, 705.2: one flip, from the seeded stream, won or lost
        // by the player who flipped; the journal says which, so the log
        // and a replay agree on it.
        Effect::FlipCoinLifeExchangeRepeat {
            opponent,
            initial,
            multiplier,
        } => super::coin_stakes::start(state, res, opponent, initial, multiplier),
        Effect::FlipCoin { won, lost } => {
            match super::coin::flip(
                state,
                res,
                you,
                super::CoinContinuation::FlipBranches { won, lost },
            ) {
                super::coin::Flip::Outcome(heads) => {
                    run_nested(state, res, if heads { won } else { lost })
                }
                super::coin::Flip::Wait(pending) => Some(*pending),
            }
        }
        Effect::IfEventPowerAtLeast { n, then, otherwise } => {
            let power = res
                .event_object
                .and_then(|id| state.object(id))
                .and_then(|o| o.characteristics().power)
                .unwrap_or(0);
            let branch = if power >= n { then } else { otherwise };
            let targets: SmallVec<[ObjectId; 2]> = res.event_object.into_iter().collect();
            run_nested_with(state, res, flatten(branch), targets)
        }
        // --- Effects, emblems, and the misc tail -------------------------
        // A departed player's resolution goes on without them (CR 608.2m),
        // but it creates nothing for them and gives them control of nothing.
        // An emblem is owned by the player who gets it (CR 114.2), and a
        // copy of a spell by the player it is put on the stack under
        // (CR 707.10), so neither is created (CR 800.4d); nothing changes to
        // their control (CR 800.4b).
        Effect::CreateEmblem { .. }
        | Effect::CopyTargetSpell { .. }
        | Effect::CopyTargetAbility
        | Effect::CopyEventSpellOrAbility
        | Effect::TargetsCopyEventSpellUntilEndOfTurn
        | Effect::CopyThisSpell { .. }
        | Effect::CreateContinuousEffect {
            modifier: baylee_cards_dsl::Modifier::GainControl,
            ..
        } if state.has_left(you) => None,
        // CR 611.2b: a "for as long as you control" that is already over
        // does nothing.
        Effect::CreateContinuousEffect {
            duration: baylee_cards_dsl::Duration::WhileYouControlSource,
            ..
        } if !source_still_yours(state, res, you) => None,
        // … and so does a "for as long as this remains tapped".
        Effect::CreateContinuousEffect {
            duration: baylee_cards_dsl::Duration::WhileSourceTapped { .. },
            ..
        } if !source_still_tapped(state, res) => None,
        Effect::AttackWithoutTappingWhileSourceUntappedThisCombat => {
            if let Some(source) = subjects::source(state, res)
                && subjects::on_battlefield(state, source)
            {
                let timestamp = state.next_timestamp();
                let modifier =
                    baylee_cards_dsl::Modifier::AttackWithoutTappingWhileSourceUntapped {
                        source_version: source.version,
                    };
                state.effects.register(crate::effects::ContinuousEffect {
                    id: baylee_core::ids::EffectId::new(0),
                    source: Some(source.object),
                    controller: you,
                    origin: crate::effects::EffectOrigin::Resolution,
                    layer: modifier.layer(),
                    timestamp,
                    duration: baylee_cards_dsl::Duration::UntilEndOfCombat,
                    filter: crate::effects::EffectFilter::Dsl(
                        &baylee_cards_dsl::Filter::YOUR_CREATURE,
                    ),
                    modifier,
                });
            }
            None
        }
        Effect::CreateContinuousEffect {
            layer,
            filter,
            modifier,
            duration,
        } => {
            if !super::continuous_subjects::source_duration_starts(state, res, duration) {
                return None;
            }
            if matches!(
                duration,
                baylee_cards_dsl::Duration::WhileSourceTapped { .. }
            ) && !subjects::source(state, res)
                .is_some_and(|source| subjects::on_battlefield(state, source))
            {
                return None;
            }
            let filters = if matches!(filter, baylee_cards_dsl::Filter::This) {
                // Nothing to become anything: the ability said "target" and
                // was activated with none, or its object is gone, so this
                // half of its sentence has no subject and registers nothing.
                let this = this_to_affect(state, res)?;
                smallvec::smallvec![crate::effects::EffectFilter::object(state, this)]
            } else {
                bound_now_with_context(state, filter, &modifier, you, res.rule_context(), None)
            };
            let timestamp = state.next_timestamp();
            for filter in filters {
                if !crate::permanent_guards::permits_resolution_effect(state, filter, modifier, you)
                    || !state.source_duration_holds(duration, res.source, you, filter)
                {
                    continue;
                }
                state.effects.register(crate::effects::ContinuousEffect {
                    id: baylee_core::ids::EffectId::new(0),
                    source: Some(res.source),
                    controller: you,
                    origin: crate::effects::EffectOrigin::Resolution,
                    layer,
                    timestamp,
                    duration,
                    filter,
                    modifier,
                });
            }
            if matches!(modifier, baylee_cards_dsl::Modifier::GrantStatic { .. }) {
                state.refresh_characteristics();
                crate::effects::sync_granted_statics(state);
            }
            None
        }
        Effect::EmpowerJace { token, amount } => super::empower::start(state, res, token, amount),
        Effect::ModifyEvent { modifier, duration } => {
            let subject =
                subjects::event(state, res).filter(|r| subjects::is_current(state, *r))?;
            if !crate::permanent_guards::permits_resolution_effect(
                state,
                crate::effects::EffectFilter::ObjectIs(subject.object, subject.version),
                modifier,
                you,
            ) {
                return None;
            }
            let timestamp = state.next_timestamp();
            state.effects.register(crate::effects::ContinuousEffect {
                id: baylee_core::ids::EffectId::new(0),
                source: Some(res.source),
                controller: you,
                origin: crate::effects::EffectOrigin::Resolution,
                layer: modifier.layer(),
                timestamp,
                duration,
                filter: crate::effects::EffectFilter::ObjectIs(subject.object, subject.version),
                modifier,
            });
            None
        }
        Effect::ModifyTargets { modifier, duration } => {
            if !super::continuous_subjects::source_duration_starts(state, res, duration) {
                return None;
            }
            if (matches!(modifier, baylee_cards_dsl::Modifier::GainControl) && state.has_left(you))
                || (matches!(duration, baylee_cards_dsl::Duration::WhileYouControlSource)
                    && !source_still_yours(state, res, you))
            {
                return None;
            }
            if matches!(
                duration,
                baylee_cards_dsl::Duration::WhileSourceTapped { .. }
            ) && !subjects::source(state, res)
                .is_some_and(|source| subjects::on_battlefield(state, source))
            {
                return None;
            }
            let timestamp = state.next_timestamp();
            for &target in &res.targets {
                let Some(reference) = super::continuous_subjects::target(state, res, target) else {
                    continue;
                };
                let filter =
                    crate::effects::EffectFilter::ObjectIs(reference.object, reference.version);
                if !crate::permanent_guards::permits_resolution_effect(state, filter, modifier, you)
                    || !state.source_duration_holds(duration, res.source, you, filter)
                {
                    continue;
                }
                state.effects.register(crate::effects::ContinuousEffect {
                    id: baylee_core::ids::EffectId::new(0),
                    source: Some(res.source),
                    controller: you,
                    origin: crate::effects::EffectOrigin::Resolution,
                    layer: modifier.layer(),
                    timestamp,
                    duration,
                    filter,
                    modifier,
                });
            }
            if matches!(modifier, baylee_cards_dsl::Modifier::GrantStatic { .. }) {
                state.refresh_characteristics();
                crate::effects::sync_granted_statics(state);
            }
            None
        }
        Effect::Earthbend(n) => {
            // CR 701.66a, in the order the rule says it. The land is the
            // first target, still on the battlefield — a target that became
            // illegal left the list at CR 608.2b.
            let land = res.targets.first().copied().filter(|t| {
                state
                    .object(*t)
                    .is_some_and(|o| o.zone == crate::zone::Zone::Battlefield)
            })?;
            let timestamp = state.next_timestamp();
            for modifier in EARTHBEND_ANIMATION {
                // Bound to the object, version and all: the land that comes
                // back is a new object and none of this applies to it
                // (CR 400.7), so it returns a land and not a 0/0 that dies
                // again.
                let filter = crate::effects::EffectFilter::object(state, land);
                state.effects.register(crate::effects::ContinuousEffect {
                    id: baylee_core::ids::EffectId::new(0),
                    source: Some(res.source),
                    controller: you,
                    origin: crate::effects::EffectOrigin::Resolution,
                    layer: modifier.layer(),
                    timestamp,
                    duration: baylee_cards_dsl::Duration::Indefinitely,
                    filter,
                    modifier,
                });
            }
            crate::replacement::put_counters_by(
                state,
                res.controller,
                land,
                baylee_cards_dsl::CounterKind::P1P1,
                n,
            );
            // The delayed trigger, controlled by whoever controlled this
            // ability and sourced where it is (CR 603.7d, 603.7e). Created
            // after the counters, so nothing that happened before it can
            // set it off (CR 603.7a).
            if let Some(version) = state.object(land).map(|o| o.version) {
                let after = state.journal.last_seq();
                state.delayed.push(crate::state::DelayedTrigger {
                    controller: you,
                    when: crate::state::DelayedWhen::DiesOrIsExiled {
                        card: land,
                        version,
                        after,
                    },
                    action: crate::state::DelayedAction::Trigger {
                        source: res.source,
                        source_version: source_version(state, res)
                            .or_else(|| state.object(res.source).map(|o| o.version))
                            .unwrap_or(0),
                        effects: EARTHBEND_RETURN,
                        text: crate::text_changes::TextChangeMap::IDENTITY,
                    },
                });
            }
            None
        }
        Effect::IncreaseSpeed => {
            if let Some(player) = state.players.get_mut(usize::from(you.get()))
                && (1..4).contains(&player.speed)
            {
                player.speed += 1;
            }
            None
        }
        Effect::BecomeMonarch(rel) => {
            // One monarch at a time (CR 724.3), so one player at most.
            if let Some(&player) = players_of(rel, state, you, res).first() {
                state.set_monarch(player);
            }
            None
        }
        Effect::Reflexive {
            when,
            effects,
            target,
        } => {
            reflexive::arm(state, res, when, effects, target);
            None
        }
        Effect::BecomeSaddled => {
            if let Some(source) = subjects::source(state, res)
                .filter(|source| subjects::on_battlefield(state, *source))
                && let Some(object) = state.object_mut(source.object)
                && !object.riders.contains(&crate::object::Rider::Saddled)
            {
                object.riders.push(crate::object::Rider::Saddled);
            }
            None
        }
        Effect::BecomePrepared | Effect::BecomeTargetPrepared => {
            let target = if matches!(op, Effect::BecomeTargetPrepared) {
                res.targets.first().copied()?
            } else {
                res.source
            };
            state.prepare(target);
            None
        }
        Effect::ProtectionFromChosenColor { duration } => {
            let candidate = if res.targeted {
                res.targets.first().copied()
            } else {
                // CR 400.7: returning with the same arena id does not make
                // the new permanent this ability's captured source.
                subjects::source(state, res)
                    .filter(|source| subjects::on_battlefield(state, *source))
                    .map(|source| source.object)
            };
            let target = candidate.filter(|t| {
                state
                    .object(*t)
                    .is_some_and(|o| o.zone == crate::zone::Zone::Battlefield)
            })?;
            res.awaiting = Some(AwaitingOp::ProtectionColor { target, duration });
            Some(Pending::ChooseColor {
                player: you,
                options: vec![
                    ManaColor::White,
                    ManaColor::Blue,
                    ManaColor::Black,
                    ManaColor::Red,
                    ManaColor::Green,
                ],
            })
        }
        Effect::PayCostOrLoseLater { cost } => {
            state.delayed.push(crate::state::DelayedTrigger {
                controller: you,
                when: crate::state::DelayedWhen::NextUpkeep,
                action: crate::state::DelayedAction::PayCostOrLose {
                    cost,
                    source: match super::payment_for(state, res) {
                        crate::casting::SpendFor::OtherFrom(source) => Some(source),
                        _ => None,
                    },
                },
            });
            None
        }
        Effect::RevealTopAndSort {
            filter,
            matched,
            otherwise,
        } => {
            let top = state
                .zones
                .list(ZoneLocation::Library(you))
                .last()
                .copied()?;
            // Shown from the library to every player, before it goes
            // anywhere (CR 701.20a), and asked about as the card it is
            // there.
            state.journal.record(GameEvent::Revealed {
                player: you,
                cards: vec![top],
            });
            let fits = state.object(top).is_some_and(|o| {
                eval::matches_with_context(filter, state, o, you, res.rule_context())
            });
            put_found(state, you, top, if fits { matched } else { otherwise });
            None
        }
        Effect::RevealTopAllMatching {
            count,
            filter,
            found,
        } => {
            let top: Vec<ObjectId> = state
                .zones
                .list(ZoneLocation::Library(you))
                .iter()
                .rev()
                .take(usize::from(count))
                .copied()
                .collect();
            if top.is_empty() {
                return None;
            }
            state.journal.record(GameEvent::Revealed {
                player: you,
                cards: top.clone(),
            });
            let chosen: Vec<ObjectId> = top
                .iter()
                .copied()
                .filter(|&card| {
                    state.object(card).is_some_and(|object| {
                        eval::matches_with_context(filter, state, object, you, res.rule_context())
                    })
                })
                .collect();
            super::dig::finish(state, res, found, top, false, false, &chosen)
        }
        Effect::RevealAndSeparate { count } => {
            let cards: Vec<ObjectId> = state
                .zones
                .list(ZoneLocation::Library(you))
                .iter()
                .rev()
                .take(count as usize)
                .copied()
                .collect();
            if cards.is_empty() {
                return None;
            }
            state.journal.record(GameEvent::Revealed {
                player: you,
                cards: cards.clone(),
            });
            // "An opponent separates": the controller names which at a
            // table with several, as CR 700.2e has the controller decide
            // which other player chooses a mode. With none left, nobody
            // separates and nothing moves.
            let opponents = eval::players(PlayerRel::Opponent, state, you).unwrap_or_default();
            match opponents.as_slice() {
                [] => None,
                [only] => Some(ask_separator(res, *only, cards)),
                _ => {
                    res.awaiting = Some(AwaitingOp::PickSeparator { cards });
                    Some(Pending::ChoosePlayer {
                        player: you,
                        options: opponents,
                    })
                }
            }
        }
        Effect::MillMayTakeOne { amount, filter } => {
            let top: Vec<ObjectId> = state
                .zones
                .list(ZoneLocation::Library(you))
                .iter()
                .rev()
                .take(amount as usize)
                .copied()
                .collect();
            for &card in &top {
                let _ = state.move_object(
                    card,
                    ZoneLocation::Graveyard(you),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            // "From among the milled cards", found where they went if that
            // zone is public (CR 701.17c), and of them the ones the filter
            // names.
            let milled: Vec<ObjectId> = top
                .into_iter()
                .filter(|&card| {
                    state.object(card).is_some_and(|o| {
                        matches!(
                            o.zone,
                            crate::zone::Zone::Graveyard | crate::zone::Zone::Exile
                        ) && eval::matches_with_context(filter, state, o, you, res.rule_context())
                    })
                })
                .collect();
            if milled.is_empty() {
                return None;
            }
            res.awaiting = Some(AwaitingOp::TakeMilled);
            Some(Pending::ChooseCards {
                player: you,
                options: milled,
                min: 0,
                max: 1,
                prompt: ChoicePrompt::PutIntoHand,
                total: None,
            })
        }
        Effect::Cascade => {
            // "This spell's mana value", X included while it is on the
            // stack (CR 202.3e); once it has left, the card's own.
            let bound = state.object(res.source).map_or(0, |o| {
                let cost = o.characteristics().mana_cost;
                if o.zone == crate::zone::Zone::Stack {
                    cost.with_x(o.x_value).cmc()
                } else {
                    cost.cmc()
                }
            });
            let library: Vec<ObjectId> = state.zones.list(ZoneLocation::Library(you)).clone();
            let mut rest = Vec::new();
            let mut hit = None;
            for &card in library.iter().rev() {
                let _ = state.move_object(
                    card,
                    ZoneLocation::Exile(you),
                    ZonePosition::Top,
                    Cause::Effect,
                );
                let chars = state.object(card).map(|o| o.characteristics().clone());
                if chars.is_some_and(|c| {
                    !c.types.contains(baylee_core::types::TypeSet::LAND)
                        && c.mana_cost.cmc() < bound
                }) {
                    hit = Some(card);
                    break;
                }
                rest.push(card);
            }
            let Some(hit) = hit else {
                bottom_in_random_order(state, you, rest);
                return None;
            };
            res.awaiting = Some(AwaitingOp::CascadeCast { hit, rest });
            Some(Pending::YesNo {
                player: you,
                prompt: YesNoPrompt::CastWithoutPaying { card: hit },
                source: resolving_ability(state, res),
            })
        }
        Effect::RevealUntil { filter, found } => {
            reveal_until(state, you, res.source, filter, found);
            None
        }
        // "You may cast that card": the first target, still where it was
        // targeted (CR 608.2b has dropped it otherwise).
        Effect::MayCastTarget { .. } | Effect::MayCastTargetAnyMana => {
            let (then_no_more_spells, any_mana) = match op {
                Effect::MayCastTarget {
                    then_no_more_spells,
                } => (then_no_more_spells, false),
                Effect::MayCastTargetAnyMana => (false, true),
                _ => unreachable!("matched MayCastTarget"),
            };
            let card = res.targets.first().copied()?;
            let version = state.object(card).map(|o| o.version)?;
            res.awaiting = Some(AwaitingOp::CastTarget {
                card,
                version,
                then_no_more_spells,
                any_mana,
            });
            Some(Pending::YesNo {
                player: you,
                prompt: YesNoPrompt::CastPaying { card },
                source: resolving_ability(state, res),
            })
        }
        Effect::ChooseFromYourGraveyard {
            filter,
            destination,
            tapped,
            optional,
        } => graveyard_choice::start(state, res, filter, destination, tapped, optional),
        Effect::SearchLibraryOrGraveyard { filter, find } => {
            let buried: Vec<ObjectId> = state
                .zones
                .list(ZoneLocation::Graveyard(you))
                .iter()
                .copied()
                .filter(|id| {
                    state.object(*id).is_some_and(|o| {
                        eval::matches_with_context(filter, state, o, you, res.rule_context())
                    })
                })
                .collect();
            if buried.is_empty() {
                return search_library_for_one(state, res, filter, find);
            }
            res.awaiting = Some(AwaitingOp::GraveyardOrLibrary { filter, find });
            Some(Pending::ChooseCards {
                player: you,
                options: buried,
                min: 0,
                max: 1,
                prompt: ChoicePrompt::FromGraveyard,
                total: None,
            })
        }
        Effect::DiscardUpToThenDraw { count } => {
            let hand: Vec<ObjectId> = state.zones.list(ZoneLocation::Hand(you)).clone();
            let most = u8::try_from(hand.len()).unwrap_or(u8::MAX).min(count);
            if most == 0 {
                return None;
            }
            res.awaiting = Some(AwaitingOp::DiscardThenDraw);
            Some(Pending::ChooseCards {
                player: you,
                options: hand,
                min: 0,
                max: most,
                prompt: ChoicePrompt::Discard,
                total: None,
            })
        }
        Effect::LookAtTopMayPut {
            filter,
            matched,
            otherwise,
        } => {
            let top = state
                .zones
                .list(ZoneLocation::Library(you))
                .last()
                .copied()?;
            let fits = state.object(top).is_some_and(|o| {
                eval::matches_with_context(filter, state, o, you, res.rule_context())
            });
            if !fits {
                put_found(state, you, top, otherwise);
                return None;
            }
            // "You may": the card itself is the question, so the one asked
            // is shown it (an object the engine asks about is one the seat
            // may see) and nobody else is. Naming nothing declines.
            res.awaiting = Some(AwaitingOp::MayPutTop {
                card: top,
                matched,
                otherwise,
            });
            Some(Pending::ChooseCards {
                player: you,
                options: vec![top],
                min: 0,
                max: 1,
                prompt: match matched.dest {
                    SearchDest::Battlefield => ChoicePrompt::PutOntoBattlefield,
                    SearchDest::Graveyard => ChoicePrompt::PutIntoGraveyard,
                    SearchDest::Exile => ChoicePrompt::Generic,
                    SearchDest::Hand => ChoicePrompt::PutIntoHand,
                    SearchDest::TopOfLibrary => ChoicePrompt::PutBackOnTop,
                },
                total: None,
            })
        }
        Effect::RevealTopOnePerType { count } => {
            let top: Vec<ObjectId> = state
                .zones
                .list(ZoneLocation::Library(you))
                .iter()
                .rev()
                .take(usize::from(count))
                .copied()
                .collect();
            if top.is_empty() {
                return None;
            }
            // Shown to every player where they are (CR 701.20a), before any
            // of them moves.
            state.journal.record(GameEvent::Revealed {
                player: you,
                cards: top.clone(),
            });
            one_per_type(state, res, top, 0)
        }
        Effect::RevealTopByName {
            player,
            matched,
            otherwise,
        } => names::reveal_top(state, res, player, matched, otherwise),
        Effect::RevealRandomDiscardNamed { player, count } => {
            names::random_discard(state, res, player, count)
        }
        Effect::LookAtTopOf { player, count } => {
            super::top_card::inspect(state, res, player, count)
        }
        Effect::RevealTopIf { filter, find } => {
            super::top_card::reveal_if(state, res, filter, find);
            None
        }
        Effect::LookAtTopMayTake { filter, rest, then } => {
            super::top_card::start(state, res, filter, rest, then)
        }
        Effect::CommanderToHand => super::top_card::commander(state, res),
        Effect::LookAtTopFind {
            found,
            count,
            filter,
            random,
        } => {
            let top: Vec<ObjectId> = state
                .zones
                .list(ZoneLocation::Library(you))
                .iter()
                .rev()
                .take(usize::from(count))
                .copied()
                .collect();
            if top.is_empty() {
                return None;
            }
            let options: Vec<ObjectId> = top
                .iter()
                .copied()
                .filter(|id| {
                    state.object(*id).is_some_and(|o| {
                        eval::matches_with_context(filter, state, o, you, res.rule_context())
                    })
                })
                .collect();
            let max = u8::from(!options.is_empty());
            res.awaiting = Some(AwaitingOp::DigRest {
                rest: top,
                reveal: true,
                random,
                found,
            });
            Some(Pending::ChooseCards {
                player: you,
                options,
                min: 0,
                max,
                prompt: match found.dest {
                    SearchDest::Hand => ChoicePrompt::PutIntoHand,
                    SearchDest::Battlefield => ChoicePrompt::PutOntoBattlefield,
                    SearchDest::TopOfLibrary => ChoicePrompt::PutBackOnTop,
                    SearchDest::Graveyard | SearchDest::Exile => ChoicePrompt::Generic,
                },
                total: None,
            })
        }
        Effect::LookAtTopPick {
            count,
            pick,
            random,
        } => {
            let count = amount2(&count, state, you, res);
            let top: Vec<ObjectId> = state
                .zones
                .list(ZoneLocation::Library(you))
                .iter()
                .rev()
                .take(count as usize)
                .copied()
                .collect();
            if top.is_empty() {
                return None;
            }
            // "Put two of them into your hand" over a library of one puts
            // the one: an effect that attempts the impossible does only as
            // much as possible (CR 609.3), and a player can't choose what is
            // impossible (CR 608.2d). Asking for `pick` regardless was a
            // question with no answer — Dig Through Time late in a game
            // asked for two cards out of one, and the table stopped (r002
            // games 368 and 2675).
            let pick = pick.min(u8::try_from(top.len()).unwrap_or(u8::MAX));
            res.awaiting = Some(AwaitingOp::DigRest {
                rest: top.clone(),
                reveal: false,
                random,
                found: baylee_cards_dsl::Find::HAND,
            });
            Some(Pending::ChooseCards {
                player: you,
                options: top,
                min: pick,
                max: pick,
                prompt: ChoicePrompt::PutIntoHand,
                total: None,
            })
        }
        Effect::LookAtTopKeepBottomPlay { count } => {
            let looked: Vec<ObjectId> = state
                .zones
                .list(ZoneLocation::Library(you))
                .iter()
                .rev()
                .take(count as usize)
                .copied()
                .collect();
            if looked.is_empty() {
                return None;
            }
            res.awaiting = Some(AwaitingOp::KeepThenBottom {
                looked: looked.clone(),
            });
            Some(Pending::ChooseCards {
                player: you,
                options: looked,
                min: 1,
                max: 1,
                prompt: ChoicePrompt::PutIntoHand,
                total: None,
            })
        }
        Effect::PayLifeOrPutBackDrawn { count, life } => {
            // The drawn cards that are still the objects they were drawn as,
            // and still in this hand.
            let drawn: Vec<ObjectId> = state
                .per_turn
                .drawn
                .iter()
                .filter(|(id, version)| {
                    state.object(*id).is_some_and(|o| {
                        o.version == *version
                            && o.zone == crate::zone::Zone::Hand
                            && o.zone_owner == Some(you)
                    })
                })
                .map(|(id, _)| *id)
                .collect();
            if drawn.len() > usize::from(count) {
                res.awaiting = Some(AwaitingOp::ChooseDrawn { life });
                return Some(Pending::ChooseCards {
                    player: you,
                    options: drawn,
                    min: count,
                    max: count,
                    prompt: ChoicePrompt::Generic,
                    total: None,
                });
            }
            put_back_question(state, res, drawn, life)
        }
        Effect::ChooseExiledToPlay {
            owner,
            counter,
            free,
        } => {
            // Every exile, because a card lies in its owner's and the
            // owner here is somebody else. Seat order, then each pile's own.
            let mut options: Vec<ObjectId> = Vec::new();
            for seat in 0..state.players.len() {
                let pile = ZoneLocation::Exile(PlayerId::new(seat as u8));
                options.extend(state.zones.list(pile).iter().copied().filter(|id| {
                    state.object(*id).is_some_and(|o| {
                        o.card.is_some()
                            && owner_is(state, owner, o.owner, you)
                            && counter.is_none_or(|kind| o.counters.get(kind) > 0)
                    })
                }));
            }
            if options.is_empty() {
                return None;
            }
            res.awaiting = Some(AwaitingOp::GrantPlay { free });
            Some(Pending::ChooseCards {
                player: you,
                options,
                min: 1,
                max: 1,
                prompt: ChoicePrompt::PlayFromExile,
                total: None,
            })
        }
        Effect::WishToHand { filter } => {
            // Karn's choice explicitly includes face-up cards in exile.
            // Commander disallows only its outside-game branch (CR 903.11).
            let mut options: Vec<ObjectId> = Vec::new();
            for loc in [ZoneLocation::OutsideGame(you), ZoneLocation::Exile(you)] {
                if matches!(loc, ZoneLocation::OutsideGame(_))
                    && state.format == baylee_core::preset::FormatId::Commander
                {
                    continue;
                }
                options.extend(state.zones.list(loc).iter().copied().filter(|id| {
                    state.object(*id).is_some_and(|o| {
                        o.owner == you
                            && o.is_card()
                            && !o.status.contains(crate::object::Status::FACE_DOWN)
                            && eval::matches_with_context(filter, state, o, you, res.rule_context())
                    })
                }));
            }
            if options.is_empty() {
                return None;
            }
            res.awaiting = Some(AwaitingOp::WishToHand);
            Some(Pending::ChooseCards {
                player: you,
                options,
                min: 0,
                max: 1,
                prompt: ChoicePrompt::Wish,
                total: None,
            })
        }
        // The new targets are chosen at resolution (CR 115.7).
        Effect::ChangeTarget { to } => retarget::start(state, res, Some(to)),
        Effect::ChooseNewTargets => retarget::start(state, res, None),
        // Added directly after this turn, ahead of any already queued:
        // "The most recently created turn will be taken first" (CR 500.7).
        Effect::LoseLifeAtNextDrawUnlessPaidBefore {
            player,
            amount,
            cost,
        } => {
            super::draw_debts::schedule(state, res, player, amount, cost);
            None
        }
        Effect::CantAttackDuringNextControllerTurn { target } => {
            super::future_attacks::schedule(state, res, target);
            None
        }
        Effect::SkipNextCombatPhase { player } => {
            for player in players_of(player, state, you, res) {
                let skips = &mut state.players[usize::from(player.get())].combats_to_skip;
                if let Some(next) = skips.checked_add(1) {
                    *skips = next;
                } else {
                    state.numeric_failure = Some("combat skip obligations exceed u32");
                }
            }
            None
        }
        Effect::SkipNextTurn => {
            let skips = &mut state.players[usize::from(you.get())].turns_to_skip;
            if let Some(next) = skips.checked_add(1) {
                *skips = next;
            } else {
                state.numeric_failure = Some("turn skip obligations exceed u32");
            }
            None
        }
        Effect::TakeExtraTurn => {
            state.extra_turns.push_front(you);
            None
        }
        Effect::CreateEmblem { abilities } => {
            let name = match state.object(res.source) {
                Some(o) => o.base.name,
                None => state.names.intern("emblem"),
            };
            let id = state.create_bare(you, ObjectKind::Emblem, name, ZoneLocation::Command(you));
            if let Some(obj) = state.object_mut(id) {
                // No card prints an emblem's list.
                obj.take_abilities(crate::object::AbilityList {
                    acquisition: 0,
                    token: None,
                    abilities: abilities.into(),
                    printed: None,
                });
            }
            None
        }
        Effect::GrantFlashback => {
            if let Some(&target) = res.targets.first() {
                let ts = state.next_timestamp();
                state.effects.register(crate::effects::ContinuousEffect {
                    id: baylee_core::ids::EffectId::new(0),
                    source: Some(res.source),
                    controller: you,
                    origin: crate::effects::EffectOrigin::Resolution,
                    layer: baylee_cards_dsl::Layer::Text,
                    timestamp: ts,
                    duration: baylee_cards_dsl::Duration::UntilEndOfTurn,
                    filter: crate::effects::EffectFilter::object(state, target),
                    modifier: baylee_cards_dsl::Modifier::GrantsFlashback,
                });
            }
            None
        }
        Effect::TapEventObject => {
            if let Some(reference) = subjects::event(state, res)
                && subjects::on_battlefield(state, reference)
            {
                tap(state, reference.object);
            }
            None
        }
        Effect::SuppressUntap { target, steps } => {
            super::untap_suppression::apply(state, res, target, steps);
            None
        }
        Effect::TapTarget => {
            for &target in &res.targets.clone() {
                tap(state, target);
            }
            for &target in &res.second_targets.clone() {
                tap(state, target);
            }
            None
        }
        Effect::LeftRightPilesRestrictBlocks => super::river::begin(state, res),
        Effect::BlockInPilesAtRandomThisTurn => {
            state.per_turn.camouflage = true;
            None
        }
        Effect::CounterEventAbility => {
            super::event_actions::counter(state, res);
            None
        }
        // CR 506.4: "an effect specifically removes it from combat".
        Effect::RemoveEventObjectFromCombat { unblock } => {
            if let Some(reference) = subjects::event(state, res)
                && subjects::on_battlefield(state, reference)
            {
                super::event_actions::remove_from_combat(state, reference.object, unblock);
            }
            None
        }
        Effect::RemoveTargetFromCombat { unblock } => {
            let removed: SmallVec<[ObjectId; 2]> = if res.targets.is_empty() && !res.targeted {
                this_to_affect(state, res).into_iter().collect()
            } else {
                res.targets.iter().copied().collect()
            };
            for target in removed {
                super::event_actions::remove_from_combat(state, target, unblock);
            }
            None
        }
        Effect::TargetMayBlockAttackerOfChoice => {
            let blocker = *res.targets.first()?;
            let controller = state
                .object(blocker)
                .filter(|o| {
                    o.zone == crate::zone::Zone::Battlefield
                        && o.characteristics()
                            .types
                            .contains(baylee_core::types::TypeSet::CREATURE)
                })?
                .controller;
            // Only an attacker it could be blocking: one attacking its
            // controller or a planeswalker they control (CR 506.3e, 802.4a).
            let options: Vec<ObjectId> = state
                .combat
                .attackers()
                .iter()
                .filter(|a| {
                    crate::combat::historical_defending_player(state, a.creature)
                        == Some(controller)
                })
                .map(|a| a.creature)
                .collect();
            if options.is_empty() {
                return None;
            }
            res.awaiting = Some(AwaitingOp::BlockAttacker { blocker });
            Some(Pending::ChooseCards {
                player: you,
                options,
                min: 0,
                max: 1,
                prompt: ChoicePrompt::BlockWith { blocker },
                total: None,
            })
        }
        Effect::TapSelf => {
            if let Some(id) = this_to_affect(state, res)
                && state
                    .object(id)
                    .is_some_and(|o| o.zone == crate::zone::Zone::Battlefield)
            {
                tap(state, id);
            }
            None
        }
        Effect::ToggleTapTarget => {
            for &target in &res.targets.clone() {
                let tapped = state
                    .object(target)
                    .is_some_and(|o| o.status.contains(crate::object::Status::TAPPED));
                if tapped {
                    untap(state, target);
                } else {
                    tap(state, target);
                }
            }
            None
        }
        // Cryptic Command's third mode. Nothing is targeted, and a
        // phased-out permanent is treated as though it doesn't exist (CR
        // 702.26b), which `battlefield_seen` is.
        // Ragavan's impulse: the top card of each named library goes to its
        // owner's exile face up, and the controller may cast it this turn.
        // A permission for that object and no later one (`PlayPermission`),
        // so a card that moves again is not cast under it (CR 400.7).
        Effect::ExileAndCastCopies { graveyard } => {
            super::card_copies::exile_and_cast(state, res, graveyard)
        }
        Effect::ExileTopToPlay { who, play } => {
            permissions::exile_top_to_play(state, res, who, play)
        }
        Effect::ExileTopMayCast { who, nonland_only } => {
            permissions::exile_top_may_cast(state, res, who, nonland_only)
        }
        Effect::TapAll { filter } => {
            let you = res.controller;
            let all: Vec<ObjectId> = state
                .battlefield_seen()
                .filter(|id| {
                    state.object(*id).is_some_and(|o| {
                        eval::matches_with_context(filter, state, o, you, res.rule_context())
                    })
                })
                .collect();
            for id in all {
                tap(state, id);
            }
            None
        }
        // Mana Short's "tap all lands target player controls": the seats
        // are the resolution's (a targeted player is `Chosen`), and the
        // permanents are whatever they control as this resolves.
        Effect::TapAllOf { who, filter } => {
            let seats = players_of(who, state, you, res);
            let all: Vec<ObjectId> = state
                .battlefield_seen()
                .filter(|id| {
                    state.object(*id).is_some_and(|o| {
                        seats.contains(&o.controller)
                            && eval::matches_with_context(filter, state, o, you, res.rule_context())
                    })
                })
                .collect();
            for id in all {
                tap(state, id);
            }
            None
        }
        // CR 106.4: losing mana is the pool emptying. All of it, because the
        // effect empties it and not a step ending (`ManaFlags::NO_EMPTY`
        // answers only CR 500.5).
        Effect::LoseUnspentMana { who } => {
            for seat in players_of(who, state, you, res) {
                state.players[seat.get() as usize].mana_pool = baylee_core::mana::ManaPool::new();
            }
            None
        }
        Effect::UntapAll { filter } => {
            let you = res.controller;
            let all: Vec<ObjectId> = state
                .battlefield_seen()
                .filter(|id| {
                    state.object(*id).is_some_and(|o| {
                        eval::matches_with_context(filter, state, o, you, res.rule_context())
                    })
                })
                .collect();
            for id in all {
                untap(state, id);
            }
            None
        }
        Effect::UntapTarget => {
            for &target in &res.targets {
                untap(state, target);
            }
            None
        }
        // "Untap this artifact." The source and not a target, so nothing is
        // chosen and nothing can be made an illegal choice; a source that has
        // left the battlefield untaps nothing, which `untap` answers by
        // finding no object rather than by a check here.
        Effect::UntapSelf => {
            if let Some(source) =
                subjects::source(state, res).filter(|r| subjects::on_battlefield(state, *r))
            {
                untap(state, source.object);
            }
            None
        }
        Effect::UntapFound => {
            for &found in &res.subject.found {
                if subjects::on_battlefield(state, found) {
                    untap(state, found.object);
                }
            }
            None
        }
        Effect::TargetSourceLosesAbilities { source_filter } => {
            // The permanent an earlier effect of this same resolution took
            // an ability off. Reading `res.targets` here instead is what
            // made the whole rider dead code: the ability it names has been
            // removed from the arena by then.
            if let Some(src) = res.countered_source {
                let applies = state.object(src).is_some_and(|o| {
                    o.zone == crate::zone::Zone::Battlefield
                        && eval::matches_with_context(
                            source_filter,
                            state,
                            o,
                            you,
                            res.rule_context(),
                        )
                });
                if applies {
                    let ts = state.next_timestamp();
                    state.effects.register(crate::effects::ContinuousEffect {
                        id: baylee_core::ids::EffectId::new(0),
                        source: Some(res.source),
                        controller: you,
                        origin: crate::effects::EffectOrigin::Resolution,
                        layer: baylee_cards_dsl::Layer::Ability,
                        timestamp: ts,
                        duration: baylee_cards_dsl::Duration::WhileSourceOnBattlefield,
                        filter: crate::effects::EffectFilter::object(state, src),
                        modifier: baylee_cards_dsl::Modifier::LoseAllAbilities,
                    });
                }
            }
            None
        }
        Effect::CopyTargetSpell { mods } => {
            let &original = res.targets.first()?;
            let copy = copy_spell(state, state.source_identity(original)?, you, mods, res.text)?;
            retarget::start_copy(state, res, copy)
        }
        Effect::CopyTargetAbility => copy_target_ability(state, res, you),
        Effect::CopyEventSpellOrAbility => super::copying::copy_event(state, res),
        Effect::TargetsCopyEventSpellUntilEndOfTurn => {
            super::copying::targets_copy_event(state, res);
            None
        }
        Effect::CopyThisSpell { amount } => {
            let count = amount2(&amount, state, you, res);
            // Decrement before a target choice can suspend. The next program
            // advance repeats this slot only while copies remain.
            res.effects[res.pc] = Effect::CopyThisSpell {
                amount: Amount::Fixed(count.saturating_sub(1)),
            };
            if count == 0 {
                return None;
            }
            // The trigger refers to its source spell, not whichever incarnation
            // its implicit target happens to have when it reaches the stack.
            let original = subjects::source(state, res);
            let Some(copy) =
                original.and_then(|original| copy_spell(state, original, you, &[], res.text))
            else {
                res.effects[res.pc] = Effect::CopyThisSpell {
                    amount: Amount::Fixed(0),
                };
                return None;
            };
            retarget::start_copy(state, res, copy)
        }
        Effect::DetachSource => {
            let source = subjects::source(state, res)?;
            if subjects::on_battlefield(state, source) {
                if let Some(object) = state.object_mut(source.object) {
                    object.attached_to = None;
                }
                state.invalidate_projections();
            }
            None
        }
        Effect::ReconfigureAttach => {
            super::attachments::attach(
                state,
                res,
                TargetSpec::Object(&baylee_cards_dsl::Filter::Any),
                true,
            );
            None
        }
        Effect::AttachSelf { target } => {
            super::attachments::attach(state, res, target, false);
            None
        }
        Effect::ChangeTargetToPlayerIfTargetsYou => {
            retarget::start_player_if_targets_you(state, res)
        }
        Effect::WhenTargetLeavesBattlefieldThisTurn { effects } => {
            let &card = res.targets.first()?;
            let version = state
                .object(card)
                .filter(|o| o.zone == crate::zone::Zone::Battlefield)?
                .version;
            let after = state.journal.last_seq();
            state.delayed.push(crate::state::DelayedTrigger {
                controller: you,
                when: crate::state::DelayedWhen::LeavesBattlefieldThisTurn {
                    card,
                    version,
                    after,
                },
                action: crate::state::DelayedAction::Trigger {
                    source: res.source,
                    source_version: source_version(state, res)
                        .or_else(|| state.object(res.source).map(|o| o.version))
                        .unwrap_or(0),
                    effects,
                    text: res.text,
                },
            });
            None
        }
        Effect::WhenSourceLeavesBattlefieldThisTurn { effects } => {
            let source = subjects::source(state, res)
                .filter(|source| subjects::on_battlefield(state, *source))?;
            let after = state.journal.last_seq();
            let action = match res
                .targets
                .first()
                .and_then(|&t| state.object(t).map(|o| (t, o.version)))
            {
                Some((object, version)) => crate::state::DelayedAction::TriggerAbout {
                    source: res.source,
                    source_version: source.version,
                    effects,
                    text: res.text,
                    object,
                    version,
                    subject: None,
                },
                None => crate::state::DelayedAction::Trigger {
                    source: res.source,
                    source_version: source.version,
                    effects,
                    text: res.text,
                },
            };
            state.delayed.push(crate::state::DelayedTrigger {
                controller: you,
                when: crate::state::DelayedWhen::LeavesBattlefieldThisTurn {
                    card: source.object,
                    version: source.version,
                    after,
                },
                action,
            });
            None
        }
        Effect::WhenSourceLeavesBattlefieldForCreatedTokens { effects } => {
            if let Some(source) =
                subjects::source(state, res).filter(|s| subjects::on_battlefield(state, *s))
            {
                let after = state.journal.last_seq();
                for token in super::this_way::created_tokens(state, res) {
                    if let Some(tv) = state
                        .object(token)
                        .filter(|o| o.zone == crate::zone::Zone::Battlefield)
                        .map(|o| o.version)
                    {
                        state.delayed.push(crate::state::DelayedTrigger {
                            controller: you,
                            when: crate::state::DelayedWhen::LeavesBattlefield {
                                card: source.object,
                                version: source.version,
                                after,
                            },
                            action: crate::state::DelayedAction::TriggerAbout {
                                source: res.source,
                                source_version: source.version,
                                effects,
                                text: res.text,
                                object: token,
                                version: tv,
                                subject: None,
                            },
                        });
                    }
                }
            }
            None
        }
        Effect::WhenCreatedTokenLeavesBattlefield { effects } => {
            let source_ver = source_version(state, res)
                .or_else(|| state.object(res.source).map(|o| o.version))
                .unwrap_or(0);
            let after = state.journal.last_seq();
            for token in super::this_way::created_tokens(state, res) {
                if let Some(tv) = state
                    .object(token)
                    .filter(|o| o.zone == crate::zone::Zone::Battlefield)
                    .map(|o| o.version)
                {
                    state.delayed.push(crate::state::DelayedTrigger {
                        controller: you,
                        when: crate::state::DelayedWhen::LeavesBattlefield {
                            card: token,
                            version: tv,
                            after,
                        },
                        action: crate::state::DelayedAction::Trigger {
                            source: res.source,
                            source_version: source_ver,
                            effects,
                            text: res.text,
                        },
                    });
                }
            }
            None
        }
        Effect::TargetDamagedCreaturesCantRegenerateThisTurn => {
            let &target = res.targets.first()?;
            if let Some(obj_ver) = state
                .object(target)
                .filter(|o| o.zone == crate::zone::Zone::Battlefield)
                .map(|o| o.version)
            {
                let pair = (target, obj_ver);
                if !state
                    .per_turn
                    .damage_cant_regenerate_sources
                    .contains(&pair)
                {
                    state.per_turn.damage_cant_regenerate_sources.push(pair);
                }
                let damaged: Vec<_> = state
                    .per_turn
                    .damage_log
                    .iter()
                    .filter_map(|entry| {
                        if entry.source.object == target
                            && entry.source.version == obj_ver
                            && let crate::event::DamageTarget::Object(rec) = entry.to
                            && let Some(rec_ver) = entry.to_version
                            && state.object(rec).is_some_and(|o| {
                                o.zone == crate::zone::Zone::Battlefield
                                    && o.version == rec_ver
                                    && o.characteristics()
                                        .types
                                        .contains(baylee_core::types::TypeSet::CREATURE)
                            })
                        {
                            Some((rec, rec_ver))
                        } else {
                            None
                        }
                    })
                    .collect();
                for pair in damaged {
                    if !state.per_turn.cant_regenerate.contains(&pair) {
                        state.per_turn.cant_regenerate.push(pair);
                    }
                }
            }
            None
        }
        Effect::ExileCreaturesDamagedByTargetIfTheyDieThisTurn => {
            let &target = res.targets.first()?;
            if let Some(obj_ver) = state
                .object(target)
                .filter(|o| o.zone == crate::zone::Zone::Battlefield)
                .map(|o| o.version)
            {
                let pair = (target, obj_ver);
                if !state.per_turn.damage_exile_if_dies_sources.contains(&pair) {
                    state.per_turn.damage_exile_if_dies_sources.push(pair);
                }
                let damaged: Vec<_> = state
                    .per_turn
                    .damage_log
                    .iter()
                    .filter_map(|entry| {
                        if entry.source.object == target
                            && entry.source.version == obj_ver
                            && let crate::event::DamageTarget::Object(rec) = entry.to
                            && let Some(rec_ver) = entry.to_version
                            && state.object(rec).is_some_and(|o| {
                                o.zone == crate::zone::Zone::Battlefield
                                    && o.version == rec_ver
                                    && o.characteristics()
                                        .types
                                        .contains(baylee_core::types::TypeSet::CREATURE)
                            })
                        {
                            Some((rec, rec_ver))
                        } else {
                            None
                        }
                    })
                    .collect();
                for pair in damaged {
                    if !state.per_turn.exile_if_dies.contains(&pair) {
                        state.per_turn.exile_if_dies.push(pair);
                    }
                }
            }
            None
        }
        Effect::FlipCoinsDamageTailsRepeatUntilBothHeads { damage } => {
            super::coin::damage_contest(state, res, damage, None, None)
        }
        Effect::RevealRandomFromHand { who, count } => {
            let n = amount2(&count, state, you, res) as usize;
            if n == 0 {
                return None;
            }
            for player in players_of(who, state, you, res) {
                let mut pool = state.zones.list(ZoneLocation::Hand(player)).clone();
                if pool.is_empty() {
                    continue;
                }
                let take = n.min(pool.len());
                let mut cards = Vec::with_capacity(take);
                for _ in 0..take {
                    let idx = state.rng.below(pool.len() as u64) as usize;
                    cards.push(pool.remove(idx));
                }
                state.journal.record(GameEvent::Revealed { player, cards });
            }
            None
        }
        Effect::DiscardRevealedThisWay => {
            let pairs = super::this_way::revealed_hand_cards(state, res);
            super::discard_or_ask(state, res, &pairs)
        }
        Effect::DestroyEachUnlessAnyPlayerPaysLife { filter, life } => {
            let context = res.rule_context();
            let permanents: Vec<ObjectId> = state
                .battlefield_seen()
                .filter(|&id| {
                    state
                        .object(id)
                        .is_some_and(|o| eval::matches_with_context(filter, state, o, you, context))
                })
                .collect();
            let remaining_players =
                eval::players(PlayerRel::EachPlayer, state, state.turn.active).unwrap_or_default();
            cleansing_advance(
                state,
                res,
                permanents,
                0,
                remaining_players,
                Vec::new(),
                life,
            )
        }
        Effect::DiscardUpToForPlayers { who, max } => {
            let remaining = players_of(who, state, you, res);
            discard_up_to_advance(state, res, remaining, Vec::new(), max)
        }
        Effect::SwapBlockers => {
            let &b1 = res.targets.first()?;
            let b2 = res
                .targets
                .get(1)
                .or_else(|| res.second_targets.first())
                .copied()?;
            if b1 == b2 {
                return None;
            }
            let (Some(o1), Some(o2)) = (
                state
                    .object(b1)
                    .filter(|o| o.zone == crate::zone::Zone::Battlefield),
                state
                    .object(b2)
                    .filter(|o| o.zone == crate::zone::Zone::Battlefield),
            ) else {
                return None;
            };
            let defending = o1.controller;
            if o2.controller != defending {
                return None;
            }
            let a1 = state.combat.blocked_by(b1);
            let a2 = state.combat.blocked_by(b2);
            if a1.is_empty() || a2.is_empty() {
                return None;
            }
            if !a2
                .iter()
                .all(|&a| crate::combat::can_block(state, defending, b1, a))
                || !a1
                    .iter()
                    .all(|&a| crate::combat::can_block(state, defending, b2, a))
            {
                return None;
            }
            state
                .combat
                .blockers
                .retain(|b| b.blocker != b1 && b.blocker != b2);
            for a in a2 {
                state.declare_block(b1, a);
            }
            for a in a1 {
                state.declare_block(b2, a);
            }
            None
        }
        Effect::AnyPlayerMaySacrificeOrTakeDamageThen {
            filter,
            count,
            damage,
            effects,
        } => {
            let remaining =
                eval::players(PlayerRel::EachPlayer, state, state.turn.active).unwrap_or_default();
            any_player_sac_or_damage_advance(state, res, remaining, filter, count, damage, effects)
        }
        Effect::GrantSubtype { .. } => None, // M2 (continuous effects)
        Effect::SearchLibrary { .. }
        | Effect::SearchLibraryBeforeShuffle { .. }
        | Effect::SearchSharedLandType
        | Effect::SearchWithManaDifference { .. }
        | Effect::TargetedGraveyardToTop
        | Effect::EachPlayerSearchBeforeShuffle { .. }
        | Effect::Scry { .. }
        | Effect::Surveil { .. }
        | Effect::Explore { .. }
        | Effect::ScryFor { .. }
        | Effect::PutFromHandOnTop { .. }
        | Effect::PutFromHandOntoBattlefield { .. }
        | Effect::MayPutFromHandOntoBattlefieldAttachIfEquipment { .. }
        | Effect::OptionalBasicLandSearchFor { .. }
        | Effect::SearchLibraryOf { .. }
        | Effect::SearchLibraryUpTo { .. }
        | Effect::SearchOpponentSplits { .. }
        | Effect::PlayerMayPayOr { .. }
        | Effect::PlayerMayPayThen { .. }
        | Effect::PlayerMayPayThenElse { .. }
        | Effect::PlayerMayPayManaEachThenElse { .. }
        | Effect::UntapChosenPayingEach { .. }
        | Effect::PlayerMayPayManaOr { .. }
        | Effect::PlayerMayPayManaThen { .. }
        | Effect::PlayerMayPayLifeOr { .. }
        | Effect::PlayerMayGetPoisonOr { .. }
        | Effect::MayExileFromYourGraveyardThen { .. }
        | Effect::ReturnChosenLinkedToBattlefield { .. }
        | Effect::MayPayEnergyOr { .. }
        | Effect::ChooseOne { .. }
        | Effect::MayBehold { .. }
        | Effect::ChooseCardName { .. }
        | Effect::GuessChosenNameInHand { .. }
        | Effect::RevealHandForGuess
        | Effect::DrawIfGuessWrong
        | Effect::ChooseNumber { .. }
        | Effect::DestroyWeakest { .. }
        | Effect::ExchangeGreatestManaValue { .. }
        | Effect::PlayerMayPayCostThen { .. }
        | Effect::PlayerMaySacrificeNOr { .. }
        | Effect::PlayerMaySacrificeNThen { .. }
        | Effect::SacrificeAndIf { .. }
        | Effect::PlayerMayPayCostOr { .. }
        | Effect::PayManaToPreventDamage { .. }
        | Effect::SacrificeChosenByOpponent { .. }
        | Effect::ReorderTopLibrary { .. }
        | Effect::ReorderTopLibraryOf { .. }
        | Effect::AddManaUnlessAnyPlayerPays { .. }
        | Effect::AddMana { .. }
        | Effect::MayDo { .. }
        | Effect::MayDoFor { .. }
        | Effect::MayDoOnceEachTurn { .. }
        | Effect::OwnerPutsOnTopOrBottom { .. }
        | Effect::MayRemoveCountersSelfCreateTokens { .. }
        | Effect::MayExileTokensCreatedWithThisForCounters { .. }
        | Effect::EachPlayerMayPutFromHandOntoBattlefieldRepeatUntilNone { .. }
        | Effect::PlayerMayMillOr { .. }
        | Effect::RaidingParty
        | Effect::ChoosePhaseToSkipThisTurn
        | Effect::PayLifeOrEnterTapped { .. } => {
            unreachable!("choice ops dispatch to exec_choice")
        }
        Effect::TargetPlayersCreaturesDontUntapNextUntap => {
            for player in players_of(PlayerRel::TargetPlayerOrController, state, you, res) {
                let timestamp = state.next_timestamp();
                state.effects.register(crate::effects::ContinuousEffect {
                    id: baylee_core::ids::EffectId::new(0),
                    source: Some(res.source),
                    controller: player,
                    origin: crate::effects::EffectOrigin::Resolution,
                    layer: baylee_cards_dsl::Modifier::DoesNotUntap.layer(),
                    timestamp,
                    duration: baylee_cards_dsl::Duration::UntilYourNextUntapStep,
                    filter: crate::effects::EffectFilter::Dsl(
                        &baylee_cards_dsl::Filter::YOUR_CREATURE,
                    ),
                    modifier: baylee_cards_dsl::Modifier::DoesNotUntap,
                });
            }
            None
        }
        Effect::ExchangeControlOfEventAndTarget => {
            let event = subjects::event(state, res)
                .filter(|r| subjects::on_battlefield(state, *r))
                .map(|r| r.object);
            let target = res
                .targets
                .first()
                .copied()
                .and_then(|id| subjects::target(state, res, id))
                .filter(|r| subjects::on_battlefield(state, *r))
                .map(|r| r.object);
            if let (Some(a), Some(b)) = (event, target) {
                control::exchange(state, a, b);
            }
            None
        }
        Effect::UnattachAllEquipmentFromTarget => {
            let &host = res.targets.first()?;
            if !state
                .object(host)
                .is_some_and(|o| o.zone == crate::zone::Zone::Battlefield)
            {
                return None;
            }
            let equipped: Vec<ObjectId> = state
                .battlefield_seen()
                .filter(|&id| {
                    state.object(id).is_some_and(|o| {
                        o.attached_to == Some(host)
                            && o.characteristics()
                                .subtypes
                                .contains(baylee_core::generated::subtypes::artifact::EQUIPMENT)
                    })
                })
                .collect();
            if !equipped.is_empty() {
                for id in equipped {
                    if let Some(obj) = state.object_mut(id) {
                        obj.attached_to = None;
                    }
                }
                state.invalidate_projections();
            }
            None
        }
        Effect::FlipCoinsUntilLossOrStopCumulative => {
            match super::coin::flip(
                state,
                res,
                you,
                super::CoinContinuation::UntilLossOrStop { wins: 1 },
            ) {
                super::coin::Flip::Outcome(won) => {
                    if !won {
                        return None;
                    }
                    res.awaiting = Some(AwaitingOp::FlipCoinsUntilLossOrStopCumulative { wins: 1 });
                    Some(Pending::YesNo {
                        player: you,
                        prompt: YesNoPrompt::MayDo,
                        source: resolving_ability(state, res),
                    })
                }
                super::coin::Flip::Wait(pending) => Some(*pending),
            }
        }
        Effect::AttachedGainControlBy { new_controller } => {
            let host = crate::eval::attached_for_ability(
                state,
                res.source,
                source_version(state, res),
                source_attachment_lki(state, res.on_stack),
            )
            .map(|o| o.id)?;
            if let Some(&player) = players_of(new_controller, state, you, res).first() {
                gain_control(state, &[(host, player)]);
            }
            None
        }
        Effect::RevealTopPlayerMayPutOntoBattlefield { player, filter } => {
            let player = players_of(player, state, you, res).first().copied()?;
            let card = *state.zones.list(ZoneLocation::Library(player)).last()?;
            state.journal.record(GameEvent::Revealed {
                player,
                cards: vec![card],
            });
            let matches = state.object(card).is_some_and(|o| {
                eval::matches_with_context(filter, state, o, player, res.rule_context())
            });
            if !matches {
                return None;
            }
            res.awaiting = Some(AwaitingOp::RevealTopPlayerMayPut { player, card });
            Some(Pending::YesNo {
                player,
                prompt: YesNoPrompt::MayDo,
                source: resolving_ability(state, res),
            })
        }
        Effect::RevealUntilLandDealDamage { target } => life::reveal_until_land(state, res, target),
    }
}

pub(super) fn cleansing_advance(
    state: &mut GameState,
    res: &mut Resolution,
    permanents: Vec<ObjectId>,
    mut current_idx: usize,
    mut remaining_players: Vec<PlayerId>,
    mut to_destroy: Vec<ObjectId>,
    life: u16,
) -> Option<Pending> {
    while current_idx < permanents.len() {
        let perm = permanents[current_idx];
        if !state
            .object(perm)
            .is_some_and(|o| o.zone == crate::zone::Zone::Battlefield)
        {
            current_idx += 1;
            remaining_players =
                eval::players(PlayerRel::EachPlayer, state, state.turn.active).unwrap_or_default();
            continue;
        }
        while !remaining_players.is_empty() {
            let asked = remaining_players.remove(0);
            if state.can_pay_life(asked, i32::from(life)) {
                res.awaiting = Some(AwaitingOp::Cleansing {
                    asked,
                    permanents,
                    current_idx,
                    remaining_players,
                    to_destroy,
                    life,
                });
                return Some(Pending::YesNo {
                    player: asked,
                    prompt: crate::choice::YesNoPrompt::PayLife {
                        amount: u32::from(life),
                    },
                    source: resolving_ability(state, res),
                });
            }
        }
        to_destroy.push(perm);
        current_idx += 1;
        remaining_players =
            eval::players(PlayerRel::EachPlayer, state, state.turn.active).unwrap_or_default();
    }
    if !to_destroy.is_empty() {
        sba::destroy_all(state, &to_destroy, false);
    }
    None
}

pub(super) fn discard_up_to_advance(
    state: &mut GameState,
    res: &mut Resolution,
    mut remaining: Vec<PlayerId>,
    discards: Vec<(ObjectId, PlayerId)>,
    max_cards: u8,
) -> Option<Pending> {
    while !remaining.is_empty() {
        let asked = remaining.remove(0);
        if state.has_left(asked) {
            continue;
        }
        let options = state.zones.list(ZoneLocation::Hand(asked)).clone();
        if options.is_empty() || max_cards == 0 {
            continue;
        }
        let max = u8::try_from(options.len().min(usize::from(max_cards))).unwrap_or(max_cards);
        res.awaiting = Some(AwaitingOp::DiscardUpToForPlayers {
            asked,
            remaining,
            discards,
            max: max_cards,
        });
        return Some(Pending::ChooseCards {
            player: asked,
            options,
            min: 0,
            max,
            prompt: ChoicePrompt::Discard,
            total: None,
        });
    }
    super::discard_or_ask(state, res, &discards)
}

pub(super) fn any_player_sac_or_damage_advance(
    state: &mut GameState,
    res: &mut Resolution,
    mut remaining: Vec<PlayerId>,
    filter: &'static baylee_cards_dsl::Filter,
    count: u8,
    damage: u16,
    effects: &'static [Effect],
) -> Option<Pending> {
    while !remaining.is_empty() {
        let asked = remaining.remove(0);
        if state.has_left(asked) {
            continue;
        }
        let matches = chosen::options(state, asked, filter, asked, res.source);
        if matches.len() >= usize::from(count) {
            res.awaiting = Some(AwaitingOp::AnyPlayerSacrificeOrDamage {
                asked,
                remaining,
                step: 0,
                filter,
                count,
                damage,
                effects,
            });
            return Some(Pending::YesNo {
                player: asked,
                prompt: crate::choice::YesNoPrompt::SacrificeToEnter { count },
                source: resolving_ability(state, res),
            });
        }
        res.awaiting = Some(AwaitingOp::AnyPlayerSacrificeOrDamage {
            asked,
            remaining,
            step: 2,
            filter,
            count,
            damage,
            effects,
        });
        return Some(Pending::YesNo {
            player: asked,
            prompt: crate::choice::YesNoPrompt::Generic,
            source: resolving_ability(state, res),
        });
    }
    None
}

// Resolution-only source-power tests use CR 608.2h's retained value after
// the source leaves. Activation restrictions still inspect the live source.
/// The most times an [`Effect::Repeat`] runs its list: a bound on the
/// program it builds, far above any count a card reaches.
pub(crate) const REPEAT_CAP: u32 = 4096;

fn resolution_condition(
    state: &GameState,
    res: &Resolution,
    condition: baylee_cards_dsl::Condition,
) -> bool {
    use baylee_cards_dsl::{Condition, Filter};
    match condition {
        Condition::DiscardedForCost(filter) => state
            .object(res.on_stack)
            .and_then(|object| object.paid.as_ref())
            .is_some_and(|paid| {
                paid.discarded.iter().any(|reference| {
                    state.source_object(*reference).is_some_and(|object| {
                        eval::matches_with_context(
                            filter,
                            state,
                            object,
                            res.controller,
                            res.rule_context(),
                        )
                    })
                })
            }),
        Condition::SacrificedForCost(filter) => state
            .object(res.on_stack)
            .and_then(|object| object.paid.as_ref())
            .is_some_and(|paid| {
                paid.sacrificed.is_some_and(|(object, version)| {
                    state
                        .source_object(baylee_core::ids::DamageSourceRef { object, version })
                        .is_some_and(|object| {
                            eval::matches_with_context(
                                filter,
                                state,
                                object,
                                res.controller,
                                res.rule_context(),
                            )
                        })
                })
            }),
        Condition::SourceMatches(Filter::PowerAtLeast(n)) => {
            state
                .object(res.on_stack)
                .and_then(|o| o.source_power_lki)
                .or_else(|| {
                    state
                        .object(res.source)
                        .and_then(|o| o.characteristics().power)
                })
                .unwrap_or(0)
                >= *n
        }
        // Read through the resolution's own amount reader, which knows the
        // stack object and what this resolution has done (`ThisWay`).
        Condition::AmountIsEven(amount) => {
            amount2(&amount, state, res.controller, res).is_multiple_of(2)
        }
        Condition::CreatureDamagedBySourceDiedThisTurn => source_version(state, res)
            .or_else(|| state.object(res.source).map(|object| object.version))
            .is_some_and(|version| {
                state.source_damage_caused_death(baylee_core::ids::DamageSourceRef {
                    object: res.source,
                    version,
                })
            }),
        Condition::AmountAtLeast(amount, minimum) => {
            amount2(&amount, state, res.controller, res) >= minimum
        }
        Condition::All(parts) => parts.iter().all(|c| resolution_condition(state, res, *c)),
        Condition::Any(parts) => parts.iter().any(|c| resolution_condition(state, res, *c)),
        Condition::Not(inner) => !resolution_condition(state, res, *inner),
        Condition::TargetSpellWouldDestroy(filter) => {
            super::would_destroy::target_spell_would_destroy(state, res, filter)
        }
        _ => {
            eval::condition_holds_with_context(state, res.controller, res.rule_context(), condition)
        }
    }
}

fn discover(state: &mut GameState, res: &Resolution, mana_value: u32) -> Option<Pending> {
    // CR 701.57a, the part that is done as the ability resolves: the
    // exiling, and the cards passed over put on the bottom in a
    // random order. The cast is the engine's to offer once this
    // resolution is over (`GameState::discovered`); the cards already
    // under the library are the same cards in the same random order
    // either way.
    let you = res.controller;
    let mut passed = Vec::new();
    let mut found = None;
    while let Some(&top) = state.zones.list(ZoneLocation::Library(you)).last() {
        let _ = state.move_object(
            top,
            ZoneLocation::Exile(you),
            ZonePosition::Top,
            Cause::Effect,
        );
        let Some(obj) = state.object(top) else {
            break;
        };
        // A card that did not leave the library would be exiled
        // again and again: stop where the effect stops being able to
        // do what it says.
        if obj.zone != crate::zone::Zone::Exile {
            break;
        }
        let c = obj.characteristics();
        if !c.types.contains(baylee_core::types::TypeSet::LAND) && c.mana_value() <= mana_value {
            found = Some((top, obj.version));
            break;
        }
        passed.push(top);
    }
    state.rng.shuffle(&mut passed);
    for card in passed {
        let _ = state.move_object(
            card,
            ZoneLocation::Library(you),
            ZonePosition::Bottom,
            Cause::Effect,
        );
    }
    if let Some((card, version)) = found {
        state.discovered.push((you, card, version));
    }
    None
}

static TAKKLEMAGGOT_PING: &[baylee_cards_dsl::AbilityDef] = &[baylee_cards_dsl::triggered!(
    baylee_cards_dsl::Trigger::StepBeginChosenOpponent {
        step: baylee_cards_dsl::StepKind::Upkeep,
    },
    &[Effect::DealDamage {
        amount: baylee_cards_dsl::Amount::Fixed(1),
        target: TargetSpec::Player(PlayerRel::ActivePlayer),
    }]
)];

fn takklemaggot_death(state: &mut GameState, res: &mut Resolution) -> Option<Pending> {
    let you = res.controller;
    let chooser = *players_of(PlayerRel::ControllerOfEvent, state, you, res).first()?;
    let aura = subjects::moving_source(state, res)?;
    if !state
        .object(aura.object)
        .is_some_and(|o| o.zone == crate::zone::Zone::Graveyard && o.is_card())
    {
        return None;
    }
    let options: Vec<ObjectId> = state
        .battlefield_seen()
        .filter(|&host| crate::aura_bindings::can_attach(state, aura.object, host))
        .collect();
    if !options.is_empty() {
        res.awaiting = Some(AwaitingOp::TakklemaggotAttach { aura });
        return Some(Pending::ChooseCards {
            player: chooser,
            options,
            min: 1,
            max: 1,
            prompt: crate::choice::ChoicePrompt::NewHost { aura: aura.object },
            total: None,
        });
    }
    let printed = state.object(aura.object).and_then(GameObject::printed_face);
    let abilities = crate::object::AbilityList::from_static(TAKKLEMAGGOT_PING, printed, None);
    if let Some(obj) = state.object_mut(aura.object) {
        let owner = obj.owner;
        obj.kind = ObjectKind::Permanent;
        obj.set_controller(owner);
        obj.original_base = Some(std::sync::Arc::clone(&obj.base));
        obj.base_mut()
            .subtypes
            .remove(baylee_core::generated::subtypes::enchantment::AURA);
        obj.cache.clear();
        obj.set_chosen_opponent(Some(chooser));
        obj.take_abilities(abilities);
        obj.riders.push(crate::object::Rider::EntryCopyApplied);
    }
    state.set_tapped(aura.object, false);
    let _ = state.move_object(
        aura.object,
        ZoneLocation::Battlefield,
        ZonePosition::Top,
        Cause::Effect,
    );
    None
}
