//! The effects that ask a question before they can be carried out.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary
use super::*;

/// Operations that suspend on a player choice.
#[allow(clippy::too_many_lines)] // the choice-op dispatch table is naturally flat
pub(super) fn exec_choice(
    state: &mut GameState,
    res: &mut Resolution,
    op: Effect,
) -> Option<Pending> {
    let you = res.controller;
    match op {
        Effect::ChooseCardName {
            player,
            types,
            excluded_types,
            remember,
        } => {
            let player = players_of(player, state, you, res).first().copied()?;
            res.awaiting = Some(AwaitingOp::ChooseCardName { remember });
            Some(Pending::ChooseCardName {
                player,
                types,
                excluded_types,
            })
        }
        Effect::GuessChosenNameInHand { who, guesser } => {
            let hand_owner = players_of(who, state, you, res).first().copied()?;
            let guesser = players_of(guesser, state, you, res).first().copied()?;
            let (_, name) = res.subject.named?;
            let actual = state
                .zones
                .list(ZoneLocation::Hand(hand_owner))
                .iter()
                .filter_map(|&card| state.object(card))
                .any(|card| card.characteristics().name == name);
            res.awaiting = Some(AwaitingOp::GuessNamedCardInHand { actual });
            Some(Pending::YesNo {
                player: guesser,
                prompt: YesNoPrompt::Generic,
                source: resolving_ability(state, res),
            })
        }
        Effect::RevealHandForGuess => {
            res.subject.guess.as_mut()?.revealed = true;
            zones::exec(
                state,
                res,
                Effect::RevealHand {
                    who: PlayerRel::You,
                },
            )
        }
        Effect::DrawIfGuessWrong => {
            static DRAW: &[Effect] = &[Effect::draw(1)];
            if res
                .subject
                .guess
                .is_some_and(|guess| guess.revealed && guess.actual != guess.claimed)
            {
                return super::run_nested(state, res, DRAW);
            }
            None
        }
        Effect::ChooseNumber { min, max } => {
            res.awaiting = Some(AwaitingOp::ChooseNumber);
            Some(Pending::ChooseNumber {
                player: you,
                min: u32::from(min),
                max: u32::from(max),
                reason: crate::choice::NumberPrompt::Choice,
            })
        }
        Effect::DestroyWeakest { filter, no_regen } => {
            super::weakest::start(state, res, filter, no_regen)
        }
        Effect::ExchangeGreatestManaValue { filter, with } => {
            super::greatest_exchange::start(state, res, filter, with)
        }
        Effect::Explore { target } => super::explore::start(state, res, target),
        Effect::MayBehold { filter, then } => super::behold::start(state, res, filter, then),
        Effect::ChooseOne { choices } => {
            // CR 608.2d and 701.26b: an optional untap cannot replace a scry
            // when the original permanent is gone or already untapped.
            let choices: Vec<_> = choices
                .iter()
                .filter(|effect| {
                    !matches!(effect, Effect::UntapSelf)
                        || subjects::source(state, res).is_some_and(|source| {
                            subjects::on_battlefield(state, source)
                                && state.object(source.object).is_some_and(|object| {
                                    object.status.contains(crate::object::Status::TAPPED)
                                })
                        })
                })
                .collect();
            if choices.is_empty() {
                return None;
            }
            let pending = Pending::ChooseCastMode {
                player: you,
                object: res.source,
                options: choices
                    .iter()
                    .enumerate()
                    .map(|(i, _)| crate::choice::CastModeDesc {
                        index: u32::try_from(i).expect("at most 256 choices"),
                        kind: crate::choice::CastModeKind::Mode(i),
                        cost: baylee_core::mana::ManaCost::ZERO,
                    })
                    .collect(),
            };
            res.awaiting = Some(AwaitingOp::ChooseEffect { choices });
            Some(pending)
        }
        Effect::PayManaToPreventDamage { player, amount } => {
            let player = players_of(player, state, you, res).first().copied()?;
            let amount = amount2(&amount, state, you, res);
            res.awaiting = Some(AwaitingOp::ManaForDamage { player, amount });
            // The driver fills this mana-only offer before exposing it.
            Some(Pending::Priority {
                player,
                legal: Box::default(),
            })
        }
        Effect::SacrificeChosenByOpponent { player, filter } => {
            let player = players_of(player, state, you, res).first().copied()?;
            chosen::opponent_sacrifice(state, res, player, filter)
        }
        Effect::SearchSharedLandType => shared_land::begin(state, res),
        Effect::SearchWithManaDifference { filter, value } => {
            upgrade::begin(state, res, filter, value)
        }
        Effect::TargetedGraveyardToTop => graveyard_top::begin(state, res),
        Effect::EachPlayerSearchBeforeShuffle { filter, find } => {
            search_group::begin(state, res, filter, find)
        }
        Effect::SearchLibrary {
            filter,
            finds,
            optional,
        }
        | Effect::SearchLibraryBeforeShuffle {
            filter,
            finds,
            optional,
        } => begin_search(
            state,
            res,
            Search {
                library: you,
                searcher: you,
                filter,
                bound: None,
                finds,
                count: None,
                distinct_names: false,
                split: None,
                optional,
            },
        ),
        Effect::SearchLibraryUpTo {
            filter,
            count,
            find,
        } => {
            // Read once as the search begins, as `SearchLibraryOf`'s bound
            // is; an X of nought finds nothing and still searches, so the
            // library is shuffled all the same.
            let count = u8::try_from(amount2(&count, state, you, res)).unwrap_or(u8::MAX);
            begin_search(
                state,
                res,
                Search {
                    library: you,
                    searcher: you,
                    filter,
                    bound: None,
                    finds: core::slice::from_ref(find),
                    count: Some(count),
                    distinct_names: false,
                    split: None,
                    optional: true,
                },
            )
        }
        Effect::SearchOpponentSplits {
            filter,
            up_to,
            chosen,
        } => begin_search(
            state,
            res,
            Search {
                library: you,
                searcher: you,
                filter,
                bound: None,
                // Placed by the split, never by a find; a taken-over search
                // (Opposition Agent) exiles whatever it finds either way.
                finds: &[baylee_cards_dsl::effect::Find::HAND],
                count: Some(up_to),
                distinct_names: true,
                split: Some(chosen),
                optional: true,
            },
        ),
        Effect::SearchLibraryOf {
            library,
            owner_searches,
            filter,
            mana_value,
            finds,
            optional,
        } => {
            // A player the relation cannot name (a target that has gone)
            // searches nothing.
            let library = players_of(library, state, you, res).first().copied()?;
            // Ours to search and to take, or theirs to do both.
            let searcher = if owner_searches { library } else { you };
            // The number is the resolution's, read once as the search
            // begins: the sacrifice was paid before any of this, and a
            // search does not change it.
            let bound = mana_value.map(|b| (b.cmp, amount2(&b.amount, state, you, res)));
            begin_search(
                state,
                res,
                Search {
                    library,
                    searcher,
                    filter,
                    bound,
                    finds,
                    count: None,
                    distinct_names: false,
                    split: None,
                    optional,
                },
            )
        }
        Effect::ScryFor { player, amount } => {
            let player = players_of(player, state, you, res).first().copied()?;
            let n = eval::amount_with_context(&amount, state, player, res.rule_context(), res.x)
                as usize;
            let looked: Vec<ObjectId> = state
                .zones
                .list(ZoneLocation::Library(player))
                .iter()
                .rev()
                .take(n)
                .copied()
                .collect();
            if looked.is_empty() {
                return None;
            }
            res.awaiting = Some(AwaitingOp::Scry { player });
            // Two players, two roles. Jace's +2 prints "Look at the top card
            // of **target player's** library. **You** may put that card on
            // the bottom of **that player's** library" — so the library is
            // the target's and the decision is the controller's. Asking
            // `player` handed the opponent the choice of whether to keep
            // their own card, which is the opposite of what the card does.
            Some(look_question(you, looked, ArrangePlace::LibraryBottom))
        }
        Effect::Scry { amount } => {
            let n = amount2(&amount, state, you, res) as usize;
            let looked: Vec<ObjectId> = state
                .zones
                .list(ZoneLocation::Library(you))
                .iter()
                .rev()
                .take(n)
                .copied()
                .collect();
            if looked.is_empty() {
                return None;
            }
            res.awaiting = Some(AwaitingOp::Scry { player: you });
            Some(look_question(you, looked, ArrangePlace::LibraryBottom))
        }
        Effect::Surveil { amount } => {
            let n =
                eval::amount_with_context(&amount, state, you, res.rule_context(), res.x) as usize;
            let looked: Vec<ObjectId> = state
                .zones
                .list(ZoneLocation::Library(you))
                .iter()
                .rev()
                .take(n)
                .copied()
                .collect();
            // CR 701.25c: surveil 0 is not a surveil event at all, and an
            // empty library is the same nothing. Returning `None` here is
            // what makes that true — the resolution simply goes on.
            if looked.is_empty() {
                return None;
            }
            res.awaiting = Some(AwaitingOp::Surveil);
            Some(look_question(you, looked, ArrangePlace::Graveyard))
        }
        Effect::PutFromHandOnTop { count } => {
            let hand = state.zones.list(ZoneLocation::Hand(you)).clone();
            let n = (count as usize).min(hand.len());
            if n == 0 {
                return None;
            }
            res.awaiting = Some(AwaitingOp::PutBackOnTop);
            Some(Pending::ChooseCards {
                player: you,
                options: hand,
                min: n as u8,
                max: n as u8,
                prompt: ChoicePrompt::PutBackOnTop,
                total: None,
            })
        }
        Effect::PutFromHandOntoBattlefield { .. }
        | Effect::MayPutFromHandOntoBattlefieldAttachIfEquipment { .. } => {
            let (tapped, filter, mana_value, optional, attach_to_source_if_equipment) = match op {
                Effect::PutFromHandOntoBattlefield {
                    tapped,
                    filter,
                    mana_value,
                    optional,
                } => (tapped, filter, mana_value, optional, false),
                Effect::MayPutFromHandOntoBattlefieldAttachIfEquipment { filter } => {
                    (false, filter, None, true, true)
                }
                _ => unreachable!("matched PutFromHandOntoBattlefield"),
            };
            let bound = mana_value.map(|b| (b.cmp, amount2(&b.amount, state, you, res)));
            let options: Vec<ObjectId> = state
                .zones
                .list(ZoneLocation::Hand(you))
                .iter()
                .copied()
                .filter(|id| {
                    state.object(*id).is_some_and(|o| {
                        eval::matches_with_context(filter, state, o, you, res.rule_context())
                            && within(o, bound)
                    })
                })
                .collect();
            // A hand is not a hidden zone to its owner, so "put a creature
            // card" with one in hand is not a search that may fail; an empty
            // menu is simply nothing to put.
            if options.is_empty() {
                return None;
            }
            res.awaiting = Some(AwaitingOp::PutOntoBattlefield {
                tapped,
                attach_to_source_if_equipment,
            });
            Some(Pending::ChooseCards {
                player: you,
                options,
                min: u8::from(!optional),
                max: 1,
                prompt: ChoicePrompt::Generic,
                total: None,
            })
        }
        Effect::PlayerMayPayOr {
            player: rel,
            mana,
            effect,
        } => {
            // `players_of`, not `eval::players`: ward names the *caster*
            // (`ControllerOfTarget`), which the state alone cannot answer.
            let player = payment_player(state, res, rel, op)?;
            // Evaluated here rather than written into the card, because
            // Esper Sentinel's tax is its own power and a creature's power
            // is not known until the ability resolves. `u16` is what the
            // prompt and the suspended op carry; the clamp is a formality
            // (no power in the pool is near it) and not a rules choice.
            let mana = u16::try_from(amount2(&mana, state, you, res)).unwrap_or(u16::MAX);
            // The question is put whether or not the mana is already
            // floating, because CR 605.3a lets the player make it now: a
            // mana ability may be activated "whenever a rule or effect asks
            // for a mana payment, even if it's in the middle of casting or
            // resolving a spell". This used to run the fallback outright
            // against an empty pool and never ask at all, which is the
            // wrong outcome for the six cards in this pool that tax an
            // opponent — an opponent who has usually just tapped out to
            // cast the very spell being taxed. Three tests worked around it
            // by seating extra lands and said so in their own comments.
            //
            // Whether the pool covers it is decided when the answer comes
            // back, in `Engine::apply`, which is where a window can be
            // opened; nothing here can open one, because a `Resolution` has
            // no access to the engine's priority machinery.
            res.awaiting = Some(AwaitingOp::PlayerMayPay {
                player,
                cost: baylee_core::mana::ManaCost::from_symbol_generic(u32::from(mana)),
                outcome: ManaPaymentOutcome::Effects {
                    effects: std::slice::from_ref(effect),
                    on_payment: false,
                },
            });
            Some(Pending::YesNo {
                player,
                prompt: YesNoPrompt::PayTax { mana },
                source: resolving_ability(state, res),
            })
        }
        // The tax above with a printed, coloured price: the same question,
        // put for the same reason (CR 605.3a) whether or not the mana is
        // floating, and the same answer checked against the pool in
        // `Engine::apply`. Only the prompt differs, because "Pay {2}?" is a
        // number and "Pay {U}?" is not.
        Effect::PlayerMayPayManaOr {
            player,
            cost,
            effect,
        } => {
            let player = players_of(player, state, you, res).first().copied()?;
            res.awaiting = Some(AwaitingOp::PlayerMayPay {
                player,
                cost,
                outcome: ManaPaymentOutcome::Effects {
                    effects: std::slice::from_ref(effect),
                    on_payment: false,
                },
            });
            Some(Pending::YesNo {
                player,
                prompt: YesNoPrompt::PayMana { cost },
                source: resolving_ability(state, res),
            })
        }
        Effect::PlayerMayPayManaThen {
            player,
            cost,
            effects,
        } => {
            let player = players_of(player, state, you, res).first().copied()?;
            res.awaiting = Some(AwaitingOp::PlayerMayPay {
                player,
                cost,
                outcome: ManaPaymentOutcome::Effects {
                    effects,
                    on_payment: true,
                },
            });
            Some(Pending::YesNo {
                player,
                prompt: YesNoPrompt::PayMana { cost },
                source: resolving_ability(state, res),
            })
        }
        // The same question and the same payment as the tax above, with the
        // effects on the other answer: "you may pay {1}. If you do, you gain
        // 1 life." A player who cannot pay is still asked, for the tax's
        // reason (CR 605.3a lets them make the mana now), and one who says
        // yes and then cannot pay has not paid.
        Effect::PlayerMayPayThenElse {
            player,
            mana,
            paid,
            unpaid,
        } => {
            let player = players_of(player, state, you, res).first().copied()?;
            let mana = u16::try_from(amount2(&mana, state, you, res)).unwrap_or(u16::MAX);
            let cost = baylee_core::mana::ManaCost::from_symbol_generic(u32::from(mana));
            res.awaiting = Some(AwaitingOp::PlayerMayPay {
                player,
                cost,
                outcome: ManaPaymentOutcome::Branches { paid, unpaid },
            });
            Some(Pending::YesNo {
                player,
                prompt: YesNoPrompt::PayTax { mana },
                source: resolving_ability(state, res),
            })
        }
        // Magnetic Mountain: the choice first, then one payment for all.
        Effect::UntapChosenPayingEach { who, filter, each } => {
            let player = players_of(who, state, you, res).first().copied()?;
            let options: Vec<ObjectId> = state
                .battlefield_seen()
                .filter(|id| {
                    state.object(*id).is_some_and(|o| {
                        o.controller == player
                            && o.status.contains(crate::object::Status::TAPPED)
                            && eval::matches_with_context(filter, state, o, you, res.rule_context())
                    })
                })
                .collect();
            if options.is_empty() {
                return None;
            }
            let max = u8::try_from(options.len()).unwrap_or(u8::MAX);
            res.awaiting = Some(AwaitingOp::UntapPayingEach { player, each });
            Some(Pending::ChooseCards {
                player,
                options,
                min: 0,
                max,
                prompt: ChoicePrompt::Untap,
                total: None,
            })
        }
        // Cyclone's "{G} for each wind counter": the printed price repeated,
        // asked and paid as `PlayerMayPayManaOr`'s is. Nothing to pay is paid.
        Effect::PlayerMayPayManaEachThenElse {
            player,
            each,
            times,
            paid,
            unpaid,
        } => {
            let player = players_of(player, state, you, res).first().copied()?;
            let times = amount2(&times, state, you, res);
            if times == 0 {
                return run_nested(state, res, paid);
            }
            let cost = (0..times.min(u32::from(u8::MAX)))
                .fold(baylee_core::mana::ManaCost::ZERO, |total, _| {
                    total.combine(&each)
                });
            res.awaiting = Some(AwaitingOp::PlayerMayPay {
                player,
                cost,
                outcome: ManaPaymentOutcome::Branches { paid, unpaid },
            });
            Some(Pending::YesNo {
                player,
                prompt: YesNoPrompt::PayMana { cost },
                source: resolving_ability(state, res),
            })
        }
        Effect::PlayerMayPayThen {
            player,
            mana,
            effects,
        } => {
            let player = players_of(player, state, you, res).first().copied()?;
            let mana = u16::try_from(amount2(&mana, state, you, res)).unwrap_or(u16::MAX);
            res.awaiting = Some(AwaitingOp::PlayerMayPay {
                player,
                cost: baylee_core::mana::ManaCost::from_symbol_generic(u32::from(mana)),
                outcome: ManaPaymentOutcome::Effects {
                    effects,
                    on_payment: true,
                },
            });
            Some(Pending::YesNo {
                player,
                prompt: YesNoPrompt::PayTax { mana },
                source: resolving_ability(state, res),
            })
        }
        Effect::MayPayEnergyOr { amount, effect } => {
            if state.players[usize::from(you.get())].energy < amount {
                return run_nested(state, res, std::slice::from_ref(effect));
            }
            res.awaiting = Some(AwaitingOp::PayEnergy { amount, effect });
            Some(Pending::YesNo {
                player: you,
                prompt: YesNoPrompt::PayEnergy { amount },
                source: resolving_ability(state, res),
            })
        }
        Effect::PlayerMayPayLifeOr {
            player,
            life,
            effect,
        } => {
            let player = players_of(player, state, you, res).first().copied()?;
            let amount = amount2(&life, state, you, res);
            if i32::try_from(amount)
                .ok()
                .is_none_or(|amount| !state.can_pay_life(player, amount))
            {
                return run_nested(state, res, std::slice::from_ref(effect));
            }
            res.awaiting = Some(AwaitingOp::PlayerMayPayLife {
                player,
                amount,
                effect,
            });
            Some(Pending::YesNo {
                player,
                prompt: YesNoPrompt::PayLife { amount },
                source: resolving_ability(state, res),
            })
        }
        Effect::PlayerMayGetPoisonOr {
            player,
            poison,
            effect,
        } => {
            let player = players_of(player, state, you, res).first().copied()?;
            let amount = amount2(&poison, state, you, res);
            res.awaiting = Some(AwaitingOp::PlayerMayGetPoison {
                player,
                amount,
                effect,
            });
            Some(Pending::YesNo {
                player,
                prompt: YesNoPrompt::Generic,
                source: resolving_ability(state, res),
            })
        }
        Effect::MayExileFromYourGraveyardThen {
            count,
            filter,
            then,
        } => {
            let options: Vec<ObjectId> = state
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
            if options.len() < usize::from(count) {
                return None;
            }
            res.awaiting = Some(AwaitingOp::MayExileFromGraveyardThen {
                options,
                count,
                then,
            });
            Some(Pending::YesNo {
                player: you,
                prompt: YesNoPrompt::MayDo,
                source: resolving_ability(state, res),
            })
        }
        Effect::ReturnChosenLinkedToBattlefield {
            filter,
            other_to_hand,
        } => {
            let version = super::source_version(state, res);
            let all_linked = eval::linked_exile_cards(
                state,
                you,
                res.rule_context(),
                version,
                &baylee_cards_dsl::Filter::Any,
            );
            let options = eval::linked_exile_cards(state, you, res.rule_context(), version, filter);
            if options.is_empty() {
                if other_to_hand {
                    for card in all_linked {
                        if let Some(owner) = state.object(card).map(|o| o.owner) {
                            let _ = state.move_object(
                                card,
                                ZoneLocation::Hand(owner),
                                ZonePosition::Top,
                                Cause::Effect,
                            );
                        }
                    }
                }
                return None;
            }
            res.awaiting = Some(AwaitingOp::ReturnChosenLinked {
                all_linked,
                other_to_hand,
            });
            Some(Pending::ChooseCards {
                player: you,
                options,
                min: 1,
                max: 1,
                prompt: ChoicePrompt::Generic,
                total: None,
            })
        }
        Effect::PlayerMaySacrificeNOr {
            player,
            filter,
            count,
            effect,
        } => super::sacrifices::offer_payment(state, res, player, filter, count, effect, false),
        Effect::PlayerMaySacrificeNThen {
            player,
            filter,
            count,
            effect,
        } => super::sacrifices::offer_payment(state, res, player, filter, count, effect, true),
        Effect::SacrificeAndIf {
            filter,
            check,
            effect,
        } => super::sacrifices::offer_conditional(state, res, filter, check, effect),
        Effect::PlayerMayPayCostOr {
            player,
            cost,
            effect,
        }
        | Effect::PlayerMayPayCostThen {
            player,
            cost,
            effect,
        } => {
            let on_payment = matches!(op, Effect::PlayerMayPayCostThen { .. });
            // `players_of` for `PlayerMayPayOr`'s reason: the payer is named
            // relative to the ability, and a Karoo names its own controller
            // where a ward names the caster.
            let player = players_of(player, state, you, res).first().copied()?;
            if let baylee_cards_dsl::CostPart::TapN { count, .. } = cost {
                if *count == 0 {
                    // Paying nothing is still optional, so retain both answers.
                    let cost = baylee_core::mana::ManaCost::ZERO;
                    res.awaiting = Some(AwaitingOp::PlayerMayPay {
                        player,
                        cost,
                        outcome: ManaPaymentOutcome::Effects {
                            effects: std::slice::from_ref(effect),
                            on_payment,
                        },
                    });
                    return Some(Pending::YesNo {
                        player,
                        prompt: YesNoPrompt::PayMana { cost },
                        source: resolving_ability(state, res),
                    });
                }
                let pending = super::tap_group::offer(state, res, player, cost, on_payment, effect);
                return if pending.is_none() && !on_payment {
                    run_nested(state, res, std::slice::from_ref(effect))
                } else {
                    pending
                };
            }
            let options =
                cost_wizard::options_with_context(state, player, res.rule_context(), cost);
            if options.is_empty() {
                // No legal answer, so no question: a Karoo under a player
                // with no other land to return sacrifices itself, and
                // asking would be a prompt with one button on it. The
                // fallback runs inline through the same door a declined
                // payment takes.
                return if on_payment {
                    None
                } else {
                    run_nested(state, res, std::slice::from_ref(effect))
                };
            }
            res.awaiting = Some(AwaitingOp::PlayerMayPayCost {
                on_payment,
                player,
                cost,
                effect,
            });
            Some(Pending::ChooseCards {
                player,
                options,
                min: 0,
                max: 1,
                prompt: cost_wizard::prompt(cost),
                total: None,
            })
        }
        Effect::ReorderTopLibrary { .. } | Effect::ReorderTopLibraryOf { .. } => {
            let (library, count) = match op {
                Effect::ReorderTopLibraryOf { who, count } => {
                    (players_of(who, state, you, res).first().copied()?, count)
                }
                Effect::ReorderTopLibrary { count } => (you, count),
                _ => unreachable!("the arm's two variants"),
            };
            let options: Vec<ObjectId> = state
                .zones
                .list(ZoneLocation::Library(library))
                .iter()
                .rev()
                .take(count as usize)
                .copied()
                .collect();
            // One card has no order to choose and is asked anyway: a card
            // is shown to its player only while a question about it is open,
            // and a look the card prints is not skipped for being short.
            // None has nothing to put back.
            if options.is_empty() {
                return None;
            }
            res.awaiting = Some(AwaitingOp::ReorderTopLibrary { player: library });
            let n = u32::try_from(options.len()).unwrap_or(u32::MAX);
            Some(Pending::Arrange {
                player: you,
                cards: options,
                piles: vec![ArrangePile::all_of(ArrangePlace::LibraryTop, n)],
                prompt: ArrangePrompt::Order,
            })
        }
        Effect::OptionalBasicLandSearchFor { player } => {
            let player = players_of(player, state, you, res).first().copied()?;
            if !search::allowed(state, res, player, player) {
                return None;
            }
            res.awaiting = Some(AwaitingOp::MaySearchBasicLand { player, count: 1 });
            Some(Pending::YesNo {
                player,
                prompt: YesNoPrompt::MayDo,
                source: resolving_ability(state, res),
            })
        }
        Effect::AddManaUnlessAnyPlayerPays { .. } | Effect::AddMana { .. } => {
            mana::exec(state, res, op)
        }
        Effect::MayDo { effects } | Effect::MayDoFor { effects, .. } => {
            let player = match op {
                Effect::MayDoFor { player, .. } => {
                    players_of(player, state, you, res).first().copied()?
                }
                _ => you,
            };
            if !may_clause_possible(state, res, effects) {
                return None;
            }
            res.awaiting = Some(AwaitingOp::MayDo {
                effects,
                once_each_turn: None,
            });
            Some(Pending::YesNo {
                player,
                prompt: YesNoPrompt::MayDo,
                source: resolving_ability(state, res),
            })
        }
        Effect::OwnerPutsOnTopOrBottom { target: _ } => {
            // The chosen target; CR 608.2b has already dropped the ability
            // if it is gone. A spell or a permanent, and nothing else:
            // anything the target moved to since is a new object anyway.
            let card = res.targets.first().copied()?;
            let obj = state.object(card)?;
            if !matches!(
                obj.zone,
                crate::zone::Zone::Stack | crate::zone::Zone::Battlefield
            ) {
                return None;
            }
            let owner = obj.owner;
            // CR 903.9b before the end is picked: a commander its owner
            // sends home goes to the command zone, and which end of the
            // library it would have gone to is no longer a question.
            if let Some(pending) =
                ask_commander_replace(state, res, &[(card, ZoneLocation::Library(owner))])
            {
                return Some(pending);
            }
            if state
                .commander_redirect
                .iter()
                .any(|(o, home)| *o == card && *home)
            {
                if let Some(obj) = state.object_mut(card) {
                    obj.kind = ObjectKind::Card;
                }
                let _ = state.move_object(
                    card,
                    ZoneLocation::Library(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
                return None;
            }
            res.awaiting = Some(AwaitingOp::TopOrBottom { card, owner });
            // No handle a standing answer could be filed under: the owner is
            // answering about somebody else's ability, and "always the top"
            // for a card they do not control is not an answer they gave.
            Some(Pending::YesNo {
                player: owner,
                prompt: YesNoPrompt::TopOfLibrary { card },
                source: None,
            })
        }
        Effect::MayDoOnceEachTurn { effects } => {
            // The ability on the stack names its source and its index; a
            // spell has neither, and no spell prints the sentence.
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
                });
            if key.is_some_and(|key| state.ability_fires.contains_key(&key))
                || !may_clause_possible(state, res, effects)
            {
                return None;
            }
            res.awaiting = Some(AwaitingOp::MayDo {
                effects,
                once_each_turn: key,
            });
            Some(Pending::YesNo {
                player: you,
                prompt: YesNoPrompt::MayDo,
                source: resolving_ability(state, res),
            })
        }
        Effect::PayLifeOrEnterTapped { amount } => {
            // Not payable at all → no choice, enters tapped (CR 614.1c).
            if !state.can_pay_life(you, i32::from(amount)) {
                state.set_tapped(res.source, true);
                return None;
            }
            res.awaiting = Some(AwaitingOp::PayLifeOrTapSelf { amount });
            Some(Pending::YesNo {
                player: you,
                prompt: YesNoPrompt::PayLifeOrEnterTapped { amount },
                source: state.object(res.source).and_then(|o| o.card).map(|c| {
                    baylee_core::ids::AbilityRef::new(c.index, baylee_core::ids::AbilityRef::ENTERS)
                }),
            })
        }
        Effect::MayRemoveCountersSelfCreateTokens { kind, token } => {
            let source = subjects::source(state, res)
                .filter(|source| subjects::on_battlefield(state, *source))?;
            let max = u32::from(state.object(source.object)?.counters.get(kind));
            if max == 0 {
                return None;
            }
            res.awaiting = Some(AwaitingOp::RemoveCountersSelfCreateTokens {
                source,
                kind,
                token,
            });
            Some(Pending::ChooseNumber {
                player: you,
                min: 0,
                max,
                reason: crate::choice::NumberPrompt::Counters {
                    target: source.object,
                    kind,
                },
            })
        }
        Effect::MayExileTokensCreatedWithThisForCounters { kind } => {
            let source = subjects::source(state, res)?;
            let options: Vec<ObjectId> = state
                .battlefield_seen()
                .filter(|&id| {
                    state.object(id).is_some_and(|o| {
                        o.card.is_none()
                            && o.riders.contains(&crate::object::Rider::CreatedBy(source))
                    })
                })
                .collect();
            if options.is_empty() {
                return None;
            }
            let max = u8::try_from(options.len()).unwrap_or(u8::MAX);
            res.awaiting = Some(AwaitingOp::ExileTokensCreatedWithThisForCounters { source, kind });
            Some(Pending::ChooseCards {
                player: you,
                options,
                min: 0,
                max,
                prompt: ChoicePrompt::Generic,
                total: None,
            })
        }
        Effect::EachPlayerMayPutFromHandOntoBattlefieldRepeatUntilNone { filter } => {
            let seats = state.players.len();
            let initial_round: Vec<PlayerId> = (0..seats)
                .map(|offset| PlayerId::new(((usize::from(you.get()) + offset) % seats) as u8))
                .filter(|&p| !state.has_left(p))
                .collect();
            advance_eureka(state, res, initial_round, false, filter)
        }
        Effect::PlayerMayMillOr {
            player,
            count,
            effect,
        } => {
            let player = players_of(player, state, you, res).first().copied()?;
            if state.zones.list(ZoneLocation::Library(player)).len() < usize::from(count) {
                return run_nested(state, res, std::slice::from_ref(effect));
            }
            res.awaiting = Some(AwaitingOp::PlayerMayMill {
                player,
                count,
                effect,
            });
            Some(Pending::YesNo {
                player,
                prompt: YesNoPrompt::MayDo,
                source: resolving_ability(state, res),
            })
        }
        Effect::RaidingParty => {
            let remaining =
                eval::players(PlayerRel::EachPlayer, state, state.turn.active).unwrap_or_default();
            advance_raiding_party(state, res, remaining, Vec::new())
        }
        Effect::ChoosePhaseToSkipThisTurn => {
            let player = players_of(PlayerRel::EventPlayer, state, you, res)
                .first()
                .copied()?;
            res.awaiting = Some(AwaitingOp::ChoosePhaseToSkipThisTurn);
            Some(Pending::ChooseNumber {
                player,
                min: 0,
                max: 2,
                reason: crate::choice::NumberPrompt::Choice,
            })
        }
        _ => unreachable!("not a choice op"),
    }
}

pub(super) fn advance_raiding_party(
    state: &mut GameState,
    res: &mut Resolution,
    mut remaining: Vec<PlayerId>,
    protected: Vec<ObjectId>,
) -> Option<Pending> {
    while !remaining.is_empty() {
        let asked = remaining.remove(0);
        if state.has_left(asked) {
            continue;
        }
        let options: Vec<ObjectId> = state
            .battlefield_seen()
            .filter(|&id| {
                state.object(id).is_some_and(|o| {
                    o.controller == asked
                        && !o.status.contains(crate::object::Status::TAPPED)
                        && o.characteristics()
                            .types
                            .contains(baylee_core::types::TypeSet::CREATURE)
                        && o.characteristics()
                            .colors
                            .contains(baylee_core::color::Color::White)
                })
            })
            .collect();
        if !options.is_empty() {
            let max = u8::try_from(options.len()).unwrap_or(u8::MAX);
            res.awaiting = Some(AwaitingOp::RaidingParty {
                asked,
                remaining,
                step: 0,
                tap_count: 0,
                protected,
            });
            return Some(Pending::ChooseCards {
                player: asked,
                options,
                min: 0,
                max,
                prompt: ChoicePrompt::CostTap,
                total: None,
            });
        }
    }
    let to_destroy: Vec<ObjectId> = state
        .battlefield_seen()
        .filter(|&id| {
            !protected.contains(&id)
                && state.object(id).is_some_and(|o| {
                    o.characteristics()
                        .subtypes
                        .contains(baylee_core::generated::subtypes::land::PLAINS)
                })
        })
        .collect();
    if !to_destroy.is_empty() {
        crate::sba::destroy_all(state, &to_destroy, false);
    }
    None
}

pub(super) fn advance_eureka(
    state: &mut GameState,
    res: &mut Resolution,
    mut remaining_in_round: Vec<PlayerId>,
    mut any_put_this_round: bool,
    filter: &'static baylee_cards_dsl::Filter,
) -> Option<Pending> {
    loop {
        while let Some(player) = remaining_in_round.first().copied() {
            remaining_in_round.remove(0);
            if state.has_left(player) {
                continue;
            }
            let options: Vec<ObjectId> = state
                .zones
                .list(ZoneLocation::Hand(player))
                .iter()
                .copied()
                .filter(|&id| {
                    state.object(id).is_some_and(|o| {
                        o.is_card()
                            && o.characteristics().types.is_permanent()
                            && eval::matches_with_context(
                                filter,
                                state,
                                o,
                                player,
                                res.rule_context(),
                            )
                    })
                })
                .collect();
            if !options.is_empty() {
                res.awaiting = Some(AwaitingOp::Eureka {
                    player,
                    remaining_in_round,
                    any_put_this_round,
                    filter,
                });
                return Some(Pending::ChooseCards {
                    player,
                    options,
                    min: 0,
                    max: 1,
                    prompt: ChoicePrompt::Generic,
                    total: None,
                });
            }
        }
        if !any_put_this_round {
            return None;
        }
        let seats = state.players.len();
        remaining_in_round = (0..seats)
            .map(|offset| {
                PlayerId::new(((usize::from(res.controller.get()) + offset) % seats) as u8)
            })
            .filter(|&p| !state.has_left(p))
            .collect();
        any_put_this_round = false;
        if remaining_in_round.is_empty() {
            return None;
        }
    }
}

/// A group keeps its own queue across suspended nested fallback instructions.
/// The enclosing operation includes its fallback, so a finite nested operation
/// is distinct from its parent. A final continuation restores the outer event.
#[derive(Clone, Debug, Hash)]
pub(super) struct PaymentBatch {
    operation: Effect,
    remaining: Vec<PlayerId>,
    previous_player: Option<PlayerId>,
}

fn payment_player(
    state: &GameState,
    res: &mut Resolution,
    relation: PlayerRel,
    operation: Effect,
) -> Option<PlayerId> {
    if !matches!(
        relation,
        PlayerRel::EachOpponent | PlayerRel::EachPlayer | PlayerRel::EachOtherPlayer
    ) {
        return players_of(relation, state, res.controller, res)
            .first()
            .copied();
    }
    if res
        .subject
        .payment_batches
        .last()
        .is_none_or(|batch| batch.operation != operation)
    {
        let mut remaining = players_of(relation, state, res.controller, res);
        let seats = state.players.len();
        let active = usize::from(state.turn.active.get());
        // CR 101.4: active player first, then the other players in turn order.
        remaining.sort_by_key(|player| (usize::from(player.get()) + seats - active) % seats);
        remaining.reverse();
        let previous_player = res.subject.event_player();
        res.subject.payment_batches.push(PaymentBatch {
            operation,
            remaining,
            previous_player,
        });
    }
    let batch = res
        .subject
        .payment_batches
        .last_mut()
        .expect("batch initialized");
    while let Some(player) = batch.remaining.pop() {
        if state.has_left(player) {
            continue;
        }
        res.subject.set_event_player(Some(player));
        // Also schedule the final empty continuation: the last fallback may
        // suspend, and its event context must last until it has finished.
        res.effects.insert(res.pc + 1, operation);
        return Some(player);
    }
    let batch = res
        .subject
        .payment_batches
        .pop()
        .expect("batch initialized");
    res.subject.set_event_player(batch.previous_player);
    None
}

#[cfg(test)]
mod payment_group_tests;
