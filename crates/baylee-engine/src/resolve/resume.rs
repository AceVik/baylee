//! Resuming a resolution with the objects a player chose.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary
use super::*;

/// Resumes a suspended resolution with the chosen cards.
///
/// # Panics
/// When called without a suspended operation (engine invariant).
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn resume(state: &mut GameState, res: &mut Resolution, chosen: &[ObjectId]) -> Flow {
    subjects::begin_resume(state, res);
    let flow = resume_inner(state, res, chosen);
    subjects::flush(state, res);
    flow
}

#[allow(clippy::too_many_lines)] // suspended effect dispatch table
pub(super) fn resume_inner(
    state: &mut GameState,
    res: &mut Resolution,
    chosen: &[ObjectId],
) -> Flow {
    let since = state.journal.last_seq();
    let awaiting = res.awaiting.take().expect("resume without awaiting op");
    match awaiting {
        AwaitingOp::SacrificeChosen { player } => {
            if let Some(&victim) = chosen.first()
                && let Some(owner) = state
                    .object(victim)
                    .filter(|o| o.zone == crate::zone::Zone::Battlefield && o.controller == player)
                    .map(|o| o.owner)
            {
                let _ = state.move_object(
                    victim,
                    ZoneLocation::Graveyard(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
        }
        AwaitingOp::SearchLibrary {
            finds: _,
            reveal: _,
            library,
            receiver,
            split: Some(count),
        } => {
            if let Some(question) = begin_split(state, res, chosen, count, library, receiver) {
                return next_choice(state, res, since, question);
            }
        }
        AwaitingOp::SearchLibrary {
            finds,
            reveal,
            library,
            receiver,
            split: None,
        } => {
            if reveal && !chosen.is_empty() {
                // Shown from the library, before they go anywhere.
                state.journal.record(GameEvent::Revealed {
                    player: receiver,
                    cards: chosen.to_vec(),
                });
            }
            // The shuffle comes **before** the cards are placed, and that is
            // the whole of what Mystical Tutor prints: "search your library
            // for an instant or sorcery card, reveal it, then shuffle and put
            // that card on top." Shuffling afterwards put the found card on
            // top and then shuffled it straight back in, so the tutor
            // returned a random card to the top of the library — which is
            // every tutor-to-top in the pool.
            //
            // For a find that leaves the library (Cultivate's battlefield and
            // hand) the order is unobservable: the card is gone either way,
            // and the rest is a shuffled library in both readings.
            state.shuffle_library(library);
            // Positional: the first card found takes the first destination.
            // Cultivate names the battlefield first and the hand second, and
            // finding only one card then puts that one onto the battlefield —
            // the same order the printed text reads in. A search whose count
            // the resolution read (`SearchLibraryUpTo`) lists one find, and
            // every card it produces goes there (`find_for`).
            // Where each card goes is read before any of them moves: a fork
            // asks the card as it is in the library.
            let placed: Vec<(ObjectId, baylee_cards_dsl::effect::Find)> = chosen
                .iter()
                .enumerate()
                .filter_map(|(at, &card)| {
                    find_for(state, res, receiver, finds, at, card).map(|find| (card, find))
                })
                .collect();
            for (card, find) in placed {
                let (dest, tapped) = (find.dest, find.tapped);
                match dest {
                    SearchDest::Hand => {
                        let _ = state.move_object(
                            card,
                            ZoneLocation::Hand(receiver),
                            ZonePosition::Top,
                            Cause::Effect,
                        );
                    }
                    SearchDest::TopOfLibrary => {
                        let _ = state.move_object(
                            card,
                            ZoneLocation::Library(library),
                            ZonePosition::Top,
                            Cause::Effect,
                        );
                    }
                    SearchDest::Battlefield => {
                        // Under the receiver's control, written where it
                        // arrives: the searcher's own card for every search
                        // but Bribery's, where "under your control" is the
                        // point of the card.
                        if let Some(obj) = state.object_mut(card) {
                            obj.kind = ObjectKind::Permanent;
                            obj.set_controller(receiver);
                        }
                        if tapped {
                            state.set_tapped(card, true);
                        }
                        let _ = state.move_object(
                            card,
                            ZoneLocation::Battlefield,
                            ZonePosition::Top,
                            Cause::Effect,
                        );
                        // After the move, for `GraveyardToBattlefield`'s
                        // reason: the move clears a permanent's counters.
                        if let Some((kind, n)) = find.counter
                            && n > 0
                            && state
                                .object(card)
                                .is_some_and(|o| o.zone == crate::zone::Zone::Battlefield)
                        {
                            crate::replacement::put_counters(state, card, kind, n);
                        }
                    }
                }
            }
        }
        AwaitingOp::SplitToGraveyard {
            found,
            library,
            receiver,
        } => finish_split(state, &found, chosen, library, receiver),
        AwaitingOp::FirstPile { cards } => {
            let (first, second): (Vec<ObjectId>, Vec<ObjectId>) =
                cards.into_iter().partition(|card| chosen.contains(card));
            let piles = vec![first, second];
            res.awaiting = Some(AwaitingOp::TakePile {
                piles: piles.clone(),
            });
            return Flow::Wait(Pending::ChoosePile {
                player: res.controller,
                piles,
            });
        }
        AwaitingOp::InspectHand => {}
        AwaitingOp::LinkedCounterCleanup { version, kind } => {
            linked_counters::finish_cleanup(state, res.source, version, kind, chosen);
        }
        AwaitingOp::TakeMilled => {
            for &card in chosen {
                let owner = state
                    .object(card)
                    .filter(|o| {
                        matches!(
                            o.zone,
                            crate::zone::Zone::Graveyard | crate::zone::Zone::Exile
                        )
                    })
                    .map(|o| o.owner);
                if let Some(owner) = owner {
                    let _ = state.move_object(
                        card,
                        ZoneLocation::Hand(owner),
                        ZonePosition::Top,
                        Cause::Effect,
                    );
                }
            }
        }
        AwaitingOp::GraveyardOrLibrary { filter, find } => {
            let you = res.controller;
            match chosen.first() {
                // A graveyard card, still there, is the whole search.
                Some(&card)
                    if state
                        .zones
                        .list(ZoneLocation::Graveyard(you))
                        .contains(&card) =>
                {
                    if find.tapped && find.dest == SearchDest::Battlefield {
                        state.set_tapped(card, true);
                    }
                    put_found(state, you, card, find.dest);
                }
                Some(_) => {}
                None => {
                    if let Some(question) = search_library_for_one(state, res, filter, find) {
                        return next_choice(state, res, since, question);
                    }
                }
            }
        }
        AwaitingOp::DiscardThenDraw => {
            // "If you do, draw that many": what was discarded, counted as it
            // happens — a card that is no longer in the hand is not.
            let you = res.controller;
            let mut discarded = 0;
            for &card in chosen {
                if !state.zones.list(ZoneLocation::Hand(you)).contains(&card) {
                    continue;
                }
                state.journal.record(GameEvent::Discarded {
                    object: card,
                    player: you,
                });
                let _ = state.move_object(
                    card,
                    ZoneLocation::Graveyard(you),
                    ZonePosition::Top,
                    Cause::Effect,
                );
                discarded += 1;
            }
            if discarded > 0 {
                // Drawing is a later instruction: first settle the discarded
                // cards' order, even if a draw will ask another question.
                res.effects.insert(
                    res.pc + 1,
                    Effect::DrawCards {
                        amount: baylee_cards_dsl::Amount::Fixed(discarded),
                    },
                );
            }
        }
        AwaitingOp::PickSeparator { .. } => {
            unreachable!("the separator is a player, answered via resume_pick_splitter")
        }
        AwaitingOp::TakePile { .. } => {
            unreachable!("a pile is an index, answered via resume_pile")
        }
        AwaitingOp::PickSplitter { .. } => {
            unreachable!("the splitter is a player, answered via resume_pick_splitter")
        }
        AwaitingOp::MayPutTop {
            card,
            matched,
            otherwise,
        } => {
            // Still the card that was looked at, still on top: nothing can
            // have moved it between the question and the answer, but the
            // library is asked rather than trusted.
            let you = res.controller;
            if state
                .zones
                .list(ZoneLocation::Library(you))
                .last()
                .is_some_and(|&top| top == card)
            {
                if chosen.contains(&card) {
                    if matched.tapped && matched.dest == SearchDest::Battlefield {
                        state.set_tapped(card, true);
                    }
                    put_found(state, you, card, matched.dest);
                } else {
                    put_found(state, you, card, otherwise);
                }
            }
        }
        AwaitingOp::PutOntoBattlefield => {
            for &card in chosen {
                if let Some(obj) = state.object_mut(card) {
                    obj.kind = ObjectKind::Permanent;
                    obj.set_controller(res.controller);
                }
                let _ = state.move_object(
                    card,
                    ZoneLocation::Battlefield,
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
        }
        AwaitingOp::PutBackOnTop => {
            // Chosen cards go on top in chosen order (last chosen = top).
            for &card in chosen {
                let _ = state.move_object(
                    card,
                    ZoneLocation::Library(res.controller),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
        }
        AwaitingOp::KeepThenBottom { looked } => {
            for &card in chosen {
                let _ = state.move_object(
                    card,
                    ZoneLocation::Hand(res.controller),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            let rest: Vec<ObjectId> = looked.into_iter().filter(|c| !chosen.contains(c)).collect();
            if rest.len() > 1 {
                res.awaiting = Some(AwaitingOp::BottomThenPlay { rest: rest.clone() });
                return next_choice(
                    state,
                    res,
                    since,
                    Pending::ChooseCards {
                        player: res.controller,
                        options: rest,
                        min: 1,
                        max: 1,
                        prompt: ChoicePrompt::PutOnBottom,
                        total: None,
                    },
                );
            }
            // One card left is the bottom card: the sentence puts one there
            // before it exiles any, so a library of two exiles nothing.
            for card in rest {
                let _ = state.move_object(
                    card,
                    ZoneLocation::Library(res.controller),
                    ZonePosition::Bottom,
                    Cause::Effect,
                );
            }
        }
        AwaitingOp::BottomThenPlay { rest } => {
            for &card in chosen {
                let _ = state.move_object(
                    card,
                    ZoneLocation::Library(res.controller),
                    ZonePosition::Bottom,
                    Cause::Effect,
                );
            }
            for card in rest.into_iter().filter(|c| !chosen.contains(c)) {
                let _ = state.move_object(
                    card,
                    ZoneLocation::Exile(res.controller),
                    ZonePosition::Top,
                    Cause::Effect,
                );
                grant_play(state, res.controller, card, false);
            }
        }
        AwaitingOp::ChooseDrawn { life } => {
            if let Some(pending) = put_back_question(state, res, chosen.to_vec(), life) {
                return next_choice(state, res, since, pending);
            }
        }
        AwaitingOp::PayOrPutBack { cards, life } => {
            // Last named is the top card, as `PutBackOnTop` reads it.
            for &card in chosen {
                let _ = state.move_object(
                    card,
                    ZoneLocation::Library(res.controller),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            for card in cards.into_iter().filter(|c| !chosen.contains(c)) {
                // The minimum already sent back every card the life total
                // could not cover; this re-asks CR 119.4 all the same, and a
                // card it refuses goes back rather than being kept for free.
                if state.can_pay_life(res.controller, i32::from(life)) {
                    state.change_life(res.controller, -i32::from(life), Cause::Cost);
                } else {
                    let _ = state.move_object(
                        card,
                        ZoneLocation::Library(res.controller),
                        ZonePosition::Top,
                        Cause::Effect,
                    );
                }
            }
        }
        AwaitingOp::GrantPlay { free } => {
            for &card in chosen {
                grant_play(state, res.controller, card, free);
            }
        }
        AwaitingOp::DigRest { rest, random } => {
            for &card in chosen {
                let _ = state.move_object(
                    card,
                    ZoneLocation::Hand(res.controller),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            // "The rest on the bottom in any order": the player chooses
            // the order whenever there is one to choose. "In a random
            // order" asks nobody: the table's generator orders them.
            let mut remaining: Vec<ObjectId> =
                rest.into_iter().filter(|c| !chosen.contains(c)).collect();
            if random {
                state.rng.shuffle(&mut remaining);
            }
            if remaining.len() > 1 && !random {
                res.awaiting = Some(AwaitingOp::DigBottom);
                let n = u32::try_from(remaining.len()).unwrap_or(u32::MAX);
                return Flow::Wait(Pending::Arrange {
                    player: res.controller,
                    cards: remaining,
                    piles: vec![ArrangePile::all_of(ArrangePlace::LibraryBottom, n)],
                    prompt: ArrangePrompt::Order,
                });
            }
            for card in remaining {
                let _ = state.move_object(
                    card,
                    ZoneLocation::Library(res.controller),
                    ZonePosition::Bottom,
                    Cause::Effect,
                );
            }
        }
        AwaitingOp::BottomFromHand { player, then } => {
            // "That player reveals the chosen card" (Vendilion Clique): the
            // chooser has seen the hand, the rest of the table has not, and
            // a reveal shows it to every player (CR 701.20a) while it is
            // still in the hand (701.20b), before it goes under the library.
            if !chosen.is_empty() {
                state.journal.record(GameEvent::Revealed {
                    player,
                    cards: chosen.to_vec(),
                });
            }
            for &card in chosen {
                let _ = state.move_object(
                    card,
                    ZoneLocation::Library(player),
                    ZonePosition::Bottom,
                    Cause::Effect,
                );
            }
            // "If you do, …": what follows runs next, in the order written
            // (CR 608.2c), and only when a card was chosen. Declined, it is
            // skipped, as it is when nothing could be chosen and nobody was
            // asked.
            if !chosen.is_empty() {
                let next = res.pc + 1;
                res.effects.splice(next..next, then.iter().copied());
            }
        }
        AwaitingOp::WishToHand => {
            if let Some(&card) = chosen.first() {
                // "You may reveal a card you own from outside the game": shown
                // to every player (CR 701.20a) on its way into the hand. A
                // face-up card chosen from exile is public already and is
                // chosen, not revealed.
                if state
                    .object(card)
                    .is_some_and(|o| o.zone == crate::zone::Zone::OutsideGame)
                {
                    state.journal.record(GameEvent::Revealed {
                        player: res.controller,
                        cards: vec![card],
                    });
                }
                let _ = state.move_object(
                    card,
                    ZoneLocation::Hand(res.controller),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
        }
        AwaitingOp::OnePerType { revealed, next } => {
            if let Some(&card) = chosen.first() {
                let _ = state.move_object(
                    card,
                    ZoneLocation::Hand(res.controller),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            if let Some(pending) = one_per_type(state, res, revealed, next) {
                return next_choice(state, res, since, pending);
            }
        }
        AwaitingOp::NewTargets(_) => {
            unreachable!("a change of targets resumes via resume_targets")
        }
        AwaitingOp::SearchTakeover { agent, .. } => {
            for &card in chosen {
                let _ = state.move_object(
                    card,
                    ZoneLocation::Exile(agent),
                    ZonePosition::Top,
                    Cause::Effect,
                );
                if let Some(obj) = state.object_mut(card) {
                    obj.riders
                        .push(crate::object::Rider::PlayableFromExileFor(agent));
                }
            }
        }
        AwaitingOp::Equalize(selection) => {
            if let Some(pending) = equalize::resume(state, res, *selection, chosen) {
                return next_choice(state, res, since, pending);
            }
        }
        AwaitingOp::DiscardChain {
            player,
            count,
            remaining,
        } => {
            for &card in chosen {
                state.journal.record(GameEvent::Discarded {
                    object: card,
                    player,
                });
                let _ = state.move_object(
                    card,
                    ZoneLocation::Graveyard(player),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            let mut remaining = remaining;
            while let Some(player) = remaining.first().copied() {
                remaining.remove(0);
                let hand: Vec<ObjectId> = state.zones.list(ZoneLocation::Hand(player)).clone();
                if hand.is_empty() {
                    continue;
                }
                let n = (count as usize).min(hand.len()) as u8;
                res.awaiting = Some(AwaitingOp::DiscardChain {
                    player,
                    count,
                    remaining,
                });
                return next_choice(
                    state,
                    res,
                    since,
                    Pending::ChooseCards {
                        player,
                        options: hand,
                        min: n,
                        max: n,
                        prompt: ChoicePrompt::Generic,
                        total: None,
                    },
                );
            }
        }
        AwaitingOp::DestroyChosen { filter, remaining } => {
            if let Some(&victim) = chosen.first() {
                crate::sba::destroy(state, victim);
            }
            let mut remaining = remaining;
            if let Some((player, options)) =
                chosen::next_asked(state, &mut remaining, filter, res.controller, res.source)
            {
                res.awaiting = Some(AwaitingOp::DestroyChosen { filter, remaining });
                return next_choice(
                    state,
                    res,
                    since,
                    Pending::ChooseCards {
                        player,
                        options,
                        min: 0,
                        max: 1,
                        prompt: ChoicePrompt::Generic,
                        total: None,
                    },
                );
            }
        }
        AwaitingOp::SacrificeFilter { filter, remaining } => {
            if let Some(&victim) = chosen.first() {
                let owner = state.object(victim).map_or(res.controller, |o| o.owner);
                if let Some(obj) = state.object_mut(victim) {
                    obj.kind = ObjectKind::Card;
                }
                let _ = state.move_object(
                    victim,
                    ZoneLocation::Graveyard(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            // Ask the next player who still has a legal sacrifice.
            let mut remaining = remaining;
            if let Some((player, options)) =
                chosen::next_asked(state, &mut remaining, filter, res.controller, res.source)
            {
                res.awaiting = Some(AwaitingOp::SacrificeFilter { filter, remaining });
                return next_choice(
                    state,
                    res,
                    since,
                    Pending::ChooseCards {
                        player,
                        options,
                        min: 1,
                        max: 1,
                        prompt: ChoicePrompt::Generic,
                        total: None,
                    },
                );
            }
        }
        AwaitingOp::SacrificeAllChosen => sacrifice_together(state, chosen),
        AwaitingOp::ReturnChosen { filter, remaining } => {
            if let Some(&returned) = chosen.first() {
                // CR 400.3: its owner's hand, whoever was controlling it.
                let owner = state.object(returned).map_or(res.controller, |o| o.owner);
                if let Some(obj) = state.object_mut(returned) {
                    obj.kind = ObjectKind::Card;
                }
                // No `ask_commander_replace` here, and this arm is the only
                // one of the three that would ever want it: CR 903.9b is a
                // replacement for a hand or a library, while a commander
                // reaching a *graveyard* is CR 903.9a, a state-based action
                // — so the two siblings, which both end in a graveyard, have
                // nothing to ask.
                //
                // What stops it is the shape rather than the rule. The
                // question re-enters its operation at the same program
                // counter with nothing yet mutated (`resume_yes_no` returns
                // `run` with no `pc += 1`), which a start block can survive
                // and a continuation cannot: the operation here is the whole
                // per-player chain, so the re-run would ask the first player
                // to choose all over again. Unreachable today — the only
                // filter the pool writes for this effect is `Land.YouCtrl`
                // and no commander is a land — and listed in
                // `docs/engine-internals.md` beside the other paths the rule
                // does not reach, so the day a card writes
                // `Creature.YouCtrl` here it is a known gap rather than a
                // surprise.
                let _ = state.move_object(
                    returned,
                    ZoneLocation::Hand(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            let mut remaining = remaining;
            if let Some((player, options)) =
                chosen::next_asked(state, &mut remaining, filter, res.controller, res.source)
            {
                res.awaiting = Some(AwaitingOp::ReturnChosen { filter, remaining });
                return next_choice(
                    state,
                    res,
                    since,
                    Pending::ChooseCards {
                        player,
                        options,
                        min: 1,
                        max: 1,
                        prompt: ChoicePrompt::Generic,
                        total: None,
                    },
                );
            }
        }
        AwaitingOp::UntapChosen => {
            for &id in chosen {
                untap(state, id);
            }
        }
        AwaitingOp::AttachAura { aura } => {
            crate::aura_bindings::resume_attach(state, aura, chosen);
        }
        // The chosen permanent is what `then` is about, so it runs as a
        // list of its own with the choice as its object — `Filter::This`
        // names it there, as it names a target, which is what `targeted`
        // says. The lint that keeps `then` from asking anything is what
        // makes the nested run whole (`lints::chosen_then_fault`).
        AwaitingOp::ChooseYoursThen { then } => {
            if let Some(&id) = chosen.first() {
                let targeted = std::mem::replace(&mut res.targeted, true);
                let pending = run_nested_with(state, res, flatten(then), smallvec::smallvec![id]);
                res.targeted = targeted;
                if let Some(pending) = pending {
                    return next_choice(state, res, since, pending);
                }
            }
        }
        AwaitingOp::Populate => {
            if let Some(&id) = chosen.first() {
                tokens::populate(state, res.controller, id);
            }
        }
        AwaitingOp::ShieldFromChosenSource { .. } | AwaitingOp::RedirectFromChosenSource { .. } => {
            unreachable!("source decisions resume through resume_source")
        }
        AwaitingOp::GraveyardOrder { .. }
        | AwaitingOp::ReorderTopLibrary { .. }
        | AwaitingOp::DigBottom
        | AwaitingOp::Scry { .. }
        | AwaitingOp::Surveil => {
            unreachable!("arrangements resume via resume_arranged")
        }
        AwaitingOp::MaskedCast
        | AwaitingOp::ControlledCard { .. }
        | AwaitingOp::LandMana { .. }
        | AwaitingOp::TextReplacement { .. }
        | AwaitingOp::ControlRotation { .. }
        | AwaitingOp::Damage(_)
        | AwaitingOp::ManaForDamage { .. }
        | AwaitingOp::DamagePayment { .. }
        | AwaitingOp::SacrificeOpponent { .. }
        | AwaitingOp::ManaChoice { .. }
        | AwaitingOp::ProtectionColor { .. }
        | AwaitingOp::Counters { .. }
        | AwaitingOp::PayLifeOrTapSelf { .. }
        | AwaitingOp::PlayerMayPayLife { .. }
        | AwaitingOp::MayDo { .. }
        | AwaitingOp::CascadeCast { .. }
        | AwaitingOp::CastTarget { .. }
        | AwaitingOp::TopOrBottom { .. }
        | AwaitingOp::SkipDraw { .. }
        | AwaitingOp::CommanderReplace { .. } => {
            unreachable!("color/yes-no choices resume via their own functions")
        }
        AwaitingOp::PlayerMayPay { .. } => {
            unreachable!("tax choices resume via resume_tax_choice")
        }
        AwaitingOp::PlayerMayPayCost {
            player,
            cost,
            effect,
        } => {
            // Naming nothing is declining. `min: 0` is what makes the
            // question a "may", so an empty answer is the not-paid branch
            // rather than an error — and it is the only shape that can
            // express "I could pay and would rather not", which a `YesNo`
            // followed by a second question could not without asking twice.
            //
            // A payment that fails takes the fallback as well. It is the
            // same reading as `resume_tax_choice`: the answer was legal
            // when it was offered, so a refusal here is a rules outcome
            // and never a reason to abandon the resolution.
            let paid = chosen
                .first()
                .is_some_and(|&chosen| cost_wizard::pay(state, player, cost, chosen).is_ok());
            if !paid {
                return run_fallback(state, res, std::slice::from_ref(effect));
            }
        }
    }
    finish_choice(state, res, since)
}

/// Resume the exact source decision; `None` skips an impossible departed-seat choice.
///
/// # Panics
/// Panics unless the resolution is suspended at a source decision.
pub fn resume_source(
    state: &mut GameState,
    res: &mut Resolution,
    chosen: Option<baylee_core::ids::DamageSourceRef>,
) -> Flow {
    subjects::begin_resume(state, res);
    let flow = resume_source_inner(state, res, chosen);
    subjects::flush(state, res);
    flow
}

pub(super) fn resume_source_inner(
    state: &mut GameState,
    res: &mut Resolution,
    chosen: Option<baylee_core::ids::DamageSourceRef>,
) -> Flow {
    let since = state.journal.last_seq();
    match res.awaiting.take().expect("source choice suspended") {
        AwaitingOp::ShieldFromChosenSource {
            sources,
            combat_only,
            all_but,
            gain_life,
        } => {
            let you = res.controller;
            if let Some(source) = chosen.and_then(|id| {
                crate::prevention::ChosenSource::new_with_context(
                    state,
                    id,
                    sources,
                    you,
                    res.rule_context(),
                )
            }) {
                let origin = crate::prevention::ShieldOrigin {
                    source: res.source,
                    ability: resolving_ability(state, res),
                };
                state.shields.push_from(
                    crate::prevention::Shield {
                        protects: crate::prevention::Shielded::Player(you),
                        kind: crate::prevention::ShieldKind::NextFrom {
                            source,
                            all_but: u32::from(all_but),
                            gain_life,
                            combat_only,
                        },
                        controller: you,
                    },
                    Some(origin),
                );
            }
        }
        AwaitingOp::RedirectFromChosenSource { protects } => {
            let you = res.controller;
            if let Some(source) = chosen.and_then(|id| {
                crate::prevention::ChosenSource::new(
                    state,
                    id,
                    &baylee_cards_dsl::Filter::Any,
                    you,
                    res.source,
                )
            }) {
                let origin = crate::prevention::ShieldOrigin {
                    source: res.source,
                    ability: resolving_ability(state, res),
                };
                state.shields.push_from(
                    crate::prevention::Shield {
                        protects,
                        kind: crate::prevention::ShieldKind::RedirectNextFrom { source, to: you },
                        controller: you,
                    },
                    Some(origin),
                );
            }
        }
        _ => unreachable!("not a source decision"),
    }
    finish_choice(state, res, since)
}
