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
        Effect::SearchLibrary {
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
        Effect::PutFromHandOntoBattlefield {
            filter,
            mana_value,
            optional,
        } => {
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
            res.awaiting = Some(AwaitingOp::PutOntoBattlefield);
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
            player,
            mana,
            effect,
        } => {
            // `players_of`, not `eval::players`: ward names the *caster*
            // (`ControllerOfTarget`), which the state alone cannot answer.
            let player = players_of(player, state, you, res).first().copied()?;
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
                effects: std::slice::from_ref(effect),
                on_payment: false,
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
                effects: std::slice::from_ref(effect),
                on_payment: false,
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
                effects,
                on_payment: true,
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
                effects,
                on_payment: true,
            });
            Some(Pending::YesNo {
                player,
                prompt: YesNoPrompt::PayTax { mana },
                source: resolving_ability(state, res),
            })
        }
        Effect::PlayerMayPayLifeOr {
            player,
            life,
            effect,
        } => {
            let player = players_of(player, state, you, res).first().copied()?;
            let amount = u16::try_from(amount2(&life, state, you, res)).unwrap_or(u16::MAX);
            if !state.can_pay_life(player, i32::from(amount)) {
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
        Effect::PlayerMayPayCostOr {
            player,
            cost,
            effect,
        } => {
            // `players_of` for `PlayerMayPayOr`'s reason: the payer is named
            // relative to the ability, and a Karoo names its own controller
            // where a ward names the caster.
            let player = players_of(player, state, you, res).first().copied()?;
            let options =
                cost_wizard::options_with_context(state, player, res.rule_context(), cost);
            if options.is_empty() {
                // No legal answer, so no question: a Karoo under a player
                // with no other land to return sacrifices itself, and
                // asking would be a prompt with one button on it. The
                // fallback runs inline through the same door a declined
                // payment takes.
                return run_nested(state, res, std::slice::from_ref(effect));
            }
            res.awaiting = Some(AwaitingOp::PlayerMayPayCost {
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
            // Ashiok, Dream Render: opponents can't search libraries.
            if state.effects.iter().any(|fx| {
                matches!(fx.modifier, baylee_cards_dsl::Modifier::OpponentsCantSearch)
                    && state.is_opponent(fx.controller, you)
            }) {
                return None;
            }
            let player = players_of(player, state, you, res).first().copied()?;
            let options: Vec<ObjectId> = state
                .zones
                .list(ZoneLocation::Library(player))
                .iter()
                .filter(|id| {
                    state.object(**id).is_some_and(|o| {
                        o.characteristics()
                            .types
                            .contains(baylee_core::types::TypeSet::LAND)
                            && o.characteristics()
                                .supertypes
                                .contains(baylee_core::types::SupertypeSet::BASIC)
                    })
                })
                .copied()
                .collect();
            if options.is_empty() {
                return None;
            }
            // Onto the battlefield, where everyone sees it anyway.
            // The searcher's own library, and theirs to shuffle.
            res.awaiting = Some(AwaitingOp::SearchLibrary {
                finds: ONTO_BATTLEFIELD_TAPPED,
                reveal: false,
                library: player,
                receiver: player,
                split: None,
            });
            Some(Pending::ChooseCards {
                player,
                options,
                min: 0,
                max: 1,
                prompt: ChoicePrompt::SearchLibrary,
                total: None,
            })
        }
        Effect::AddMana { .. } => mana::exec(state, res, op),
        Effect::MayDo { effects } => {
            if !may_clause_possible(state, res, effects) {
                return None;
            }
            res.awaiting = Some(AwaitingOp::MayDo {
                effects,
                once_each_turn: None,
            });
            Some(Pending::YesNo {
                player: you,
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
        _ => unreachable!("not a choice op"),
    }
}
