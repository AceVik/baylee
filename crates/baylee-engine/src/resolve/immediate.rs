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
        Effect::Sequence(_) => unreachable!("sequences are flattened"),
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
        Effect::LoseGame => {
            let _ = sba::lose_by_effect(state, you);
            None
        }
        Effect::ReanimateEnchanted => {
            if let Some(finish) = crate::aura_bindings::begin_reanimation(state, res, false) {
                state.reanimation_finishes.push(finish);
            }
            None
        }
        Effect::BecomeCopyOfTarget { mods } => {
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
                    until_eot: false,
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
        | Effect::GainLifeFor { .. }
        | Effect::GainLifeDoubleX
        | Effect::LoseLife { .. }
        | Effect::DealDamage { .. }
        | Effect::DealDamageWithCappedLifeGain { .. }
        | Effect::DealDamageEvenly { .. }
        | Effect::Fight { .. }
        | Effect::DamageEqualToPower { .. }
        | Effect::EventObjectDealsDamageEqualToPower { .. }
        | Effect::DealDamageToTargetController { .. }
        | Effect::DealDamageToAttached { .. }
        | Effect::DealDamageDivided { .. }
        | Effect::DealDamageEach { .. }
        | Effect::GrantSpecialActionUntilEndOfTurn { .. }
        | Effect::RedirectNextDamage { .. }
        | Effect::LoseHalfLife { .. }
        | Effect::PreventNextDamage { .. }
        | Effect::PreventAllCombatDamageThisTurn
        | Effect::PreventNextFromChosenSource { .. }
        | Effect::RedirectNextFromChosenSource { .. } => life::exec(state, res, op),
        Effect::Exile { .. }
        | Effect::Blink { .. }
        | Effect::ReturnToHand { .. }
        | Effect::ReturnAllToHand { .. }
        | Effect::DestroyAll { .. }
        | Effect::ExileAll { .. }
        | Effect::ChooseYoursThen { .. }
        | Effect::DestroyOthersNamedLike { .. }
        | Effect::ExileGraveyard { .. }
        | Effect::GraveyardToHand { .. }
        | Effect::GraveyardAllToHand { .. }
        | Effect::GraveyardToTop { .. }
        | Effect::GraveyardToBattlefield { .. }
        | Effect::PutSourceOnTopOfLibrary
        | Effect::BottomCardFromHand { .. }
        | Effect::ShuffleGraveyardIntoLibrary
        | Effect::PhaseOut { .. }
        | Effect::ExileLinked { .. }
        | Effect::ExileTargetsWithSource
        | Effect::SacrificeSelf
        | Effect::SacrificeObject { .. }
        | Effect::PutTargetOnBottomOfLibrary
        | Effect::PutOnBottomOfLibraryFromGraveyard { .. }
        | Effect::ExileSource
        | Effect::ExileAndReturnAtEndStep
        | Effect::ExileLibraryAndShuffleHand { .. }
        | Effect::Mill { .. }
        | Effect::Destroy { .. }
        | Effect::Regenerate { .. }
        | Effect::RegenerateAll { .. }
        | Effect::DestroyChosenForPlayers { .. }
        | Effect::EqualizePermanents { .. }
        | Effect::EqualizeHands
        | Effect::DiscardForPlayers { .. }
        | Effect::DiscardRandom { .. }
        | Effect::DiscardHand { .. }
        | Effect::ShuffleIntoLibrary { .. }
        | Effect::ShuffleLibrary { .. }
        | Effect::RevealHandDiscard { .. }
        | Effect::LookAtChosenHand
        | Effect::SacrificeFilter { .. }
        | Effect::ReturnChosenToHand { .. }
        | Effect::UntapChosen { .. }
        | Effect::AllGraveyardCreaturesToBattlefield
        | Effect::YourGraveyardToBattlefield { .. }
        | Effect::ReturnToBattlefieldTapped { .. }
        | Effect::TransformSource
        | Effect::TransformSourceAtNextUpkeep
        | Effect::ExileSelfReturnAsFace { .. }
        | Effect::ReturnLinkedToBattlefield
        | Effect::ExileTargetsCreateTokens { .. }
        | Effect::CounterTargetAbility
        | Effect::CounterTargetSpellOrAbility
        | Effect::CounterTargetSpellToExile
        | Effect::CounterTargetSpell => zones::exec(state, res, op),
        Effect::DelayedManaAtNextFirstMain { .. }
        | Effect::AddManaFor { .. }
        | Effect::AddManaLikeEvent { .. } => mana::exec(state, res, op),
        Effect::AddCounter { .. }
        | Effect::AddCountersUpTo { .. }
        | Effect::RemoveCounterSelf { .. }
        | Effect::AddCounterFilter { .. }
        | Effect::DoubleCountersFilter { .. }
        | Effect::DrainAllCountersIntoSelf
        | Effect::SetPTFilter { .. }
        | Effect::PumpFilter { .. }
        | Effect::PumpTarget { .. } => counters::exec(state, res, op),
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
        | Effect::CreateTokenN { .. }
        | Effect::CreateTokenPtPerCount { .. }
        | Effect::CreateToken { .. }
        | Effect::CreateTokenFromLinked { .. } => tokens::exec(state, res, op),
        // --- Control ---------------------------------------------------
        Effect::ExchangeControlOrSacrifice => {
            let exchange = res.targets.first().copied().filter(|t| {
                state.object(*t).is_some_and(|o| {
                    o.zone == crate::zone::Zone::Battlefield && o.controller != you
                })
            });
            if let Some(target) = exchange {
                let their_controller = state.object(target).map_or(you, |o| o.controller);
                gain_control(state, &[(target, you), (res.source, their_controller)]);
            } else {
                // No exchange: sacrifice the source (Gilded Drake).
                let owner = state.object(res.source).map_or(you, |o| o.owner);
                if let Some(obj) = state.object_mut(res.source) {
                    obj.kind = ObjectKind::Card;
                }
                let _ = state.move_object(
                    res.source,
                    ZoneLocation::Graveyard(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            None
        }
        Effect::ExchangeControl => {
            // CR 608.2b has already dropped a target that became illegal, and
            // CR 701.12a makes an exchange all or nothing: one side missing
            // is no exchange. Two permanents of one player swap nothing
            // (CR 701.12b).
            let controller_of = |id: Option<&ObjectId>| {
                id.and_then(|id| state.object(*id))
                    .filter(|o| o.zone == crate::zone::Zone::Battlefield)
                    .map(|o| (o.id, o.controller))
            };
            if let (Some((a, a_ctrl)), Some((b, b_ctrl))) = (
                controller_of(res.targets.first()),
                controller_of(res.second_targets.first()),
            ) && a_ctrl != b_ctrl
            {
                gain_control(state, &[(a, b_ctrl), (b, a_ctrl)]);
            }
            None
        }
        Effect::ChangeController { new_controller } => {
            // Two readings that were both wrong, and each looked right from
            // the other side. The seat was *always* the effect's controller,
            // so `new_controller` was a field a card could write and nothing
            // would read — Wishclaw Talisman's "An opponent gains control of
            // this artifact" is the whole price of a repeatable tutor, and it
            // was handing the artifact back to the player who activated it.
            // And the object was always `res.targets.first()`, so an ability
            // saying "this artifact" rather than "target permanent" changed
            // the control of nothing at all and resolved quietly.
            //
            // `players_of` for `PlayerMayPayOr`'s reason: a seat named
            // relative to the ability is not a seat the state alone can
            // answer. A relation nobody is (no opponent left) changes
            // nothing, which is the honest outcome and not a panic.
            //
            // `.first()` and not a question: at a duel a relation naming an
            // opponent names exactly one seat, and the only card in this
            // pool writing this effect prints "an opponent" — which at three
            // seats is a choice the controller announces on resolution (CR
            // 608.2d). Taking the first is a duel assumption and is
            // wrong at a bigger table; it is written down here rather than
            // guessed at, because the fix is a `Pending` and not an index.
            let subject = res.targets.first().copied().unwrap_or(res.source);
            if let Some(&seat) = players_of(new_controller, state, you, res).first() {
                gain_control(state, &[(subject, seat)]);
            }
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
        Effect::DrawCards { amount } => {
            let n = amount2(&amount, state, you, res) as usize;
            state.draw_cards(you, n);
            None
        }
        Effect::DrawRevealDiscardUnless { keep } => {
            let drawn = state.draw_cards(you, 1);
            let &card = drawn.first()?;
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
        Effect::DrawCardsFor { amount, who } => {
            let n = amount2(&amount, state, you, res) as usize;
            for player in players_of(who, state, you, res) {
                state.draw_cards(player, n);
            }
            None
        }
        // --- Conditional branches ---------------------------------------
        Effect::IfKicked { then, otherwise } => {
            let kicked = state.object(res.on_stack).is_some_and(|o| o.kicked);
            let branch = if kicked { then } else { otherwise };
            run_nested(state, res, branch)
        }
        // The first target as it is now; a target that is gone was dropped
        // by CR 608.2b before anything here ran.
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
            // longer on the battlefield has not run out of counters, it has
            // stopped being a thing the sentence is about.
            if state
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
        Effect::AtNextEndStep { effects } => {
            let about_ref = subjects::this(state, res);
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
                when: crate::state::DelayedWhen::NextEndStep,
                action,
            });
            None
        }
        // The delayed trigger is about the object `about` names as this
        // resolves, as the object it is now (CR 603.7c); naming nothing, it
        // is about nothing.
        Effect::AtEndOfCombat { about, effects } => {
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
                when: crate::state::DelayedWhen::EndOfCombat,
                action,
            });
            None
        }
        Effect::CantBeRegeneratedThisTurn { target } => {
            for id in zones::spec_objects(state, res, target) {
                if let Some(obj) = state.object(id)
                    && obj.zone == crate::zone::Zone::Battlefield
                {
                    let named = (id, obj.version);
                    if !state.per_turn.cant_regenerate.contains(&named) {
                        state.per_turn.cant_regenerate.push(named);
                    }
                }
            }
            None
        }
        Effect::Discover { mana_value } => {
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
                if !c.types.contains(baylee_core::types::TypeSet::LAND)
                    && c.mana_value() <= u32::from(mana_value)
                {
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
            let branch = if crate::eval::condition_holds_with_context(
                state,
                you,
                res.rule_context(),
                condition,
            ) {
                then
            } else {
                otherwise
            };
            run_nested(state, res, branch)
        }
        // CR 705.1, 705.2: one flip, from the seeded stream, won or lost
        // by the player who flipped; the journal says which, so the log
        // and a replay agree on it.
        Effect::FlipCoin { won, lost } => {
            let heads = state.rng.below(2) == 0;
            state.journal.record(crate::event::GameEvent::CoinFlipped {
                player: you,
                won: heads,
            });
            run_nested(state, res, if heads { won } else { lost })
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
        | Effect::CopyThisSpell
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
        Effect::CreateContinuousEffect {
            layer,
            filter,
            modifier,
            duration,
        } => {
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
            crate::replacement::put_counters(state, land, baylee_cards_dsl::CounterKind::P1P1, n);
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
        Effect::BecomePrepared => {
            if let Some(obj) = state.object_mut(res.source)
                && !obj.riders.contains(&crate::object::Rider::Prepared)
            {
                obj.riders.push(crate::object::Rider::Prepared);
            }
            None
        }
        Effect::ProtectionFromChosenColor { duration } => {
            let target = res.targets.first().copied().filter(|t| {
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
                action: crate::state::DelayedAction::PayCostOrLose { cost },
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
        Effect::MayCastTarget {
            then_no_more_spells,
        } => {
            let card = res.targets.first().copied()?;
            let version = state.object(card).map(|o| o.version)?;
            res.awaiting = Some(AwaitingOp::CastTarget {
                card,
                version,
                then_no_more_spells,
            });
            Some(Pending::YesNo {
                player: you,
                prompt: YesNoPrompt::CastPaying { card },
                source: resolving_ability(state, res),
            })
        }
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
                random,
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
            // Cards outside the game, plus your own exile — the one place in
            // the game a wish can already see. The choice is optional ("you
            // may"), so the minimum is zero.
            let mut options: Vec<ObjectId> = Vec::new();
            for loc in [ZoneLocation::OutsideGame(you), ZoneLocation::Exile(you)] {
                options.extend(state.zones.list(loc).iter().copied().filter(|id| {
                    state.object(*id).is_some_and(|o| {
                        o.owner == you
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
        Effect::TapTarget => {
            for &target in &res.targets.clone() {
                state.set_tapped(target, true);
            }
            None
        }
        Effect::LeftRightPilesRestrictBlocks => super::river::begin(state, res),
        Effect::BlockInPilesAtRandomThisTurn => {
            state.per_turn.camouflage = true;
            None
        }
        // CR 506.4: "an effect specifically removes it from combat".
        Effect::RemoveTargetFromCombat { unblock } => {
            let removed: SmallVec<[ObjectId; 2]> = if res.targets.is_empty() && !res.targeted {
                this_to_affect(state, res).into_iter().collect()
            } else {
                res.targets.iter().copied().collect()
            };
            for target in removed {
                let was_blocking = state.combat.blocked_by(target);
                state.combat.remove_from_combat(target);
                if unblock {
                    // "Creatures it was blocking that had become blocked by
                    // only that creature this combat become unblocked."
                    for attacker in was_blocking {
                        if state.combat.blocked_only_by(attacker, target) {
                            state.combat.unblock(attacker);
                        }
                    }
                }
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
                .filter(|a| crate::combat::blocking_player(state, a.defending) == Some(controller))
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
                state.set_tapped(id, true);
            }
            None
        }
        Effect::ToggleTapTarget => {
            for &target in &res.targets.clone() {
                let tapped = state
                    .object(target)
                    .is_some_and(|o| o.status.contains(crate::object::Status::TAPPED));
                state.set_tapped(target, !tapped);
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
        Effect::ExileTopMayCast { who } => {
            let you = res.controller;
            for player in players_of(who, state, you, res) {
                let Some(top) = state
                    .zones
                    .list(ZoneLocation::Library(player))
                    .last()
                    .copied()
                else {
                    continue;
                };
                if state
                    .move_object(
                        top,
                        ZoneLocation::Exile(player),
                        ZonePosition::Top,
                        Cause::Effect,
                    )
                    .is_ok()
                {
                    grant_permission(state, you, top, false, true);
                }
            }
            None
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
                state.set_tapped(id, true);
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
                state.set_tapped(id, true);
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
        Effect::CopyThisSpell => {
            let &original = res.targets.first()?;
            let original = state
                .recorded_stack_target(res.on_stack, 0)
                .or_else(|| state.source_identity(original))?;
            let copy = copy_spell(state, original, you, &[], res.text)?;
            retarget::start_copy(state, res, copy)
        }
        Effect::AttachSelf { .. } => {
            // A resolving Aura spell carries its intended attachment into
            // the entry check. An Aura already on the battlefield cannot
            // move to a forbidden host (CR 303.4j).
            if let Some(&host) = res.targets.first()
                && state.object(res.source).is_some_and(|o| {
                    o.zone == crate::zone::Zone::Battlefield
                        && o.characteristics()
                            .subtypes
                            .contains(baylee_core::generated::subtypes::enchantment::AURA)
                })
                && !eval::permits_enchantment(state, host, res.source)
            {
                return None;
            }
            if let Some(&target_id) = res.targets.first() {
                state.attach(res.source, target_id);
            }
            None
        }
        Effect::GrantSubtype { .. } => None, // M2 (continuous effects)
        Effect::SearchLibrary { .. }
        | Effect::Scry { .. }
        | Effect::Surveil { .. }
        | Effect::ScryFor { .. }
        | Effect::PutFromHandOnTop { .. }
        | Effect::PutFromHandOntoBattlefield { .. }
        | Effect::OptionalBasicLandSearchFor { .. }
        | Effect::SearchLibraryOf { .. }
        | Effect::SearchLibraryUpTo { .. }
        | Effect::SearchOpponentSplits { .. }
        | Effect::PlayerMayPayOr { .. }
        | Effect::PlayerMayPayThen { .. }
        | Effect::PlayerMayPayManaOr { .. }
        | Effect::PlayerMayPayManaThen { .. }
        | Effect::PlayerMayPayLifeOr { .. }
        | Effect::PlayerMayPayCostOr { .. }
        | Effect::PayManaToPreventDamage { .. }
        | Effect::SacrificeChosenByOpponent { .. }
        | Effect::ReorderTopLibrary { .. }
        | Effect::ReorderTopLibraryOf { .. }
        | Effect::AddMana { .. }
        | Effect::MayDo { .. }
        | Effect::MayDoOnceEachTurn { .. }
        | Effect::OwnerPutsOnTopOrBottom { .. }
        | Effect::PayLifeOrEnterTapped { .. } => {
            unreachable!("choice ops dispatch to exec_choice")
        }
    }
}
