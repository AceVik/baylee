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
    state
        .entries
        .begin_event(state.journal.last_seq(), Some(res.controller));
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
        AwaitingOp::MeldPartner => {
            let Effect::MeldWithNamed {
                source_name,
                partner_name,
                melded,
            } = res.effects[res.pc]
            else {
                unreachable!("meld instruction")
            };
            meld::finish(state, res, chosen[0], source_name, partner_name, melded);
        }
        AwaitingOp::DungeonTax(tax) => {
            if let Some(pending) = dungeon::resume_tax(state, res, tax, chosen) {
                return next_choice(state, res, since, pending);
            }
        }
        AwaitingOp::DiscardAny(selection) => {
            return super::discard_any::resume(state, res, *selection, chosen);
        }
        AwaitingOp::TapGroup(selection) => {
            return super::tap_group::resume(state, res, &selection, chosen);
        }
        AwaitingOp::SkipDraw { source } => {
            if let Some((player, next)) = state.draw_choice_answered(source, chosen) {
                res.awaiting = Some(AwaitingOp::SkipDraw { source: next });
                return Flow::Wait(state.draw_offer_question(player, next));
            }
            return run(state, res);
        }
        AwaitingOp::Hideaway { looked } => hidden_exile::choose(state, res, looked, chosen),
        AwaitingOp::RevealLibrarySearch => {
            if !chosen.is_empty() {
                state.journal.record(GameEvent::Revealed {
                    player: res.controller,
                    cards: chosen.to_vec(),
                });
            }
        }
        AwaitingOp::ExileFromHandWithSource => {
            if let Some(&card) = chosen.first().filter(|&&card| {
                state
                    .object(card)
                    .is_some_and(|object| object.zone == crate::zone::Zone::Hand)
            }) {
                let source = state
                    .recorded_ability_source(res.on_stack)
                    .or_else(|| subjects::source(state, res))
                    .or_else(|| {
                        source_version(state, res).map(|version| {
                            baylee_core::ids::DamageSourceRef {
                                object: res.source,
                                version,
                            }
                        })
                    })
                    .unwrap_or(baylee_core::ids::DamageSourceRef {
                        object: res.source,
                        version: 0,
                    });
                let version = source.version;
                let acquisition = subjects::acquisition(state, res);
                let owner = state
                    .object(card)
                    .map_or(res.controller, |object| object.owner);
                if let Some(object) = state.object_mut(card) {
                    object.kind = ObjectKind::Card;
                    let with = crate::object::Rider::ExiledWith {
                        host: source.object,
                        version: u16::try_from(version).unwrap_or(u16::MAX),
                    };
                    if !object.riders.contains(&with) {
                        object.riders.push(with);
                    }
                    let linked = crate::object::Rider::LinkedExile {
                        source,
                        acquisition,
                    };
                    if !object.riders.contains(&linked) {
                        object.riders.push(linked);
                    }
                }
                let _ = state.move_object(
                    card,
                    ZoneLocation::Exile(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
        }
        AwaitingOp::ReturnLinkedExile { to } => {
            if let Some(question) = hidden_exile::return_cards(state, res, to) {
                return next_choice(state, res, since, question);
            }
        }
        AwaitingOp::SearchUpgrade { operation, value } => {
            return upgrade::found(state, res, *operation, value, chosen);
        }
        AwaitingOp::GraveyardToTopOrdered { player, cards } => {
            if let Some(question) = graveyard_top::place(state, res, player, cards) {
                return next_choice(state, res, since, question);
            }
        }
        AwaitingOp::SharedLandSearch(selection) => {
            return shared_land::resume(state, res, *selection, chosen);
        }
        AwaitingOp::SearchGroup(selection) => {
            if let Some(question) = search_group::resume(state, res, *selection, chosen) {
                return next_choice(state, res, since, question);
            }
        }
        AwaitingOp::NamedTop {
            card,
            player,
            destination,
        } => {
            if let Some(pending) = names::put_top(state, res, card, player, destination) {
                return next_choice(state, res, since, pending);
            }
        }
        AwaitingOp::ExchangeGreatest(selection) => {
            if let Some(question) = greatest_exchange::resume(state, res, *selection, chosen) {
                return next_choice(state, res, since, question);
            }
        }
        AwaitingOp::DestroyWeakest { no_regen } => {
            if let Some(&card) = chosen.first() {
                super::weakest::destroy(state, card, no_regen);
            }
        }
        AwaitingOp::Behold { then } => {
            if let Some(&card) = chosen.first() {
                if state
                    .object(card)
                    .is_some_and(|object| object.zone == crate::zone::Zone::Hand)
                {
                    state.journal.record(GameEvent::Revealed {
                        player: res.controller,
                        cards: vec![card],
                    });
                }
                return run_fallback(state, res, then);
            }
        }
        AwaitingOp::SacrificePayment {
            player,
            options: _,
            count,
            effect,
            on_payment,
        } => {
            if chosen.len() == usize::from(count)
                && chosen.iter().all(|id| {
                    state.object(*id).is_some_and(|o| {
                        o.zone == crate::zone::Zone::Battlefield && o.controller == player
                    })
                })
            {
                sacrifice_together(state, chosen);
                if on_payment {
                    return run_fallback(state, res, std::slice::from_ref(effect));
                }
            } else if !on_payment {
                return run_fallback(state, res, std::slice::from_ref(effect));
            }
        }
        AwaitingOp::MayExileFromGraveyardThen {
            options: _,
            count,
            then,
        } => {
            let you = res.controller;
            if chosen.len() == usize::from(count)
                && chosen.iter().all(|id| {
                    state.object(*id).is_some_and(|o| {
                        o.zone == crate::zone::Zone::Graveyard && o.zone_owner == Some(you)
                    })
                })
            {
                let source_version = super::source_version(state, res)
                    .or_else(|| state.object(res.source).map(|o| o.version));
                for &card in chosen {
                    let owner = state.object(card).map_or(you, |o| o.owner);
                    if let Some(obj) = state.object_mut(card) {
                        obj.riders.push(crate::object::Rider::Linked {
                            host: res.source,
                            until: None,
                        });
                    }
                    let _ = state.move_object(
                        card,
                        ZoneLocation::Exile(owner),
                        ZonePosition::Top,
                        Cause::Effect,
                    );
                    if let Some(version) = source_version {
                        state.remember_link(
                            card,
                            baylee_core::ids::DamageSourceRef {
                                object: res.source,
                                version,
                            },
                        );
                    }
                }
                return run_fallback(state, res, then);
            }
        }
        AwaitingOp::ReturnChosenLinked {
            all_linked,
            other_to_hand,
        } => {
            let you = res.controller;
            let picked = chosen.first().copied().filter(|id| all_linked.contains(id));
            if let Some(card) = picked {
                if let Some(obj) = state.object_mut(card) {
                    obj.kind = crate::object::ObjectKind::Permanent;
                    obj.set_controller(you);
                }
                let _ = state.move_object(
                    card,
                    ZoneLocation::Battlefield,
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            if other_to_hand {
                for other in all_linked {
                    if Some(other) != picked
                        && let Some(owner) = state
                            .object(other)
                            .filter(|o| o.zone == crate::zone::Zone::Exile)
                            .map(|o| o.owner)
                    {
                        let _ = state.move_object(
                            other,
                            ZoneLocation::Hand(owner),
                            ZonePosition::Top,
                            Cause::Effect,
                        );
                    }
                }
            }
        }
        AwaitingOp::SacrificeAndIf { check, effect } => {
            if let Some(&victim) = chosen.first() {
                let qualifies = state.object(victim).is_some_and(|object| {
                    eval::matches_with_context(
                        check,
                        state,
                        object,
                        res.controller,
                        res.rule_context(),
                    )
                });
                let eligible = state.object(victim).is_some_and(|o| {
                    o.zone == crate::zone::Zone::Battlefield && o.controller == res.controller
                });
                if eligible {
                    let sacrificed = state
                        .object(victim)
                        .map(|object| (object.version, crate::object::SacrificedLki::of(object)));
                    sacrifice_together(state, chosen);
                    let succeeded = sacrificed.filter(|(version, _)| {
                        state
                            .object(victim)
                            .is_none_or(|object| object.version != *version)
                    });
                    if let Some((_, lki)) = succeeded {
                        res.subject.sacrificed = Some(lki);
                    }
                    if qualifies && succeeded.is_some() {
                        return run_fallback(state, res, std::slice::from_ref(effect));
                    }
                }
            }
        }
        AwaitingOp::Amass { subtype, amount } => {
            if let Some(&army) = chosen.first() {
                super::amass::finish(state, res, army, subtype, amount);
            }
        }
        AwaitingOp::EmpowerJace { amount } => {
            if let Some(&jace) = chosen.first() {
                super::empower::finish(state, res, jace, amount);
            }
        }
        AwaitingOp::TapEnergy(selection) => {
            if let Some(pending) = super::tap_energy::objects(state, res, *selection, chosen) {
                return next_choice(state, res, since, pending);
            }
        }
        AwaitingOp::Proliferate(selection) => {
            if let Some(pending) = super::proliferate::objects(state, res, *selection, chosen) {
                return next_choice(state, res, since, pending);
            }
        }
        AwaitingOp::RevealDiscard(choice) => {
            return super::discard::resume_reveal(state, res, *choice, chosen);
        }
        AwaitingOp::Loot(loot) => {
            if let Some(pending) = super::loot::resume(state, res, loot, chosen) {
                return next_choice(state, res, since, pending);
            }
        }

        AwaitingOp::Fabricate {
            source,
            count,
            token,
        } => super::token_choices::finish_fabricate(state, res, chosen, source, count, token),
        AwaitingOp::AttachCreated { source } => {
            if let Some(&host) = chosen.first() {
                super::attachments::attach_to(state, source, host, false);
            }
        }
        AwaitingOp::TakeTop {
            looked,
            rest,
            reveal,
            then,
        } => {
            if let Some(question) =
                super::top_card::take(state, res, chosen, looked, rest, reveal, then)
            {
                return next_choice(state, res, since, question);
            }
        }
        AwaitingOp::RestTop { looked, rest } => {
            super::top_card::rest(state, res, chosen, looked, rest);
        }
        AwaitingOp::CommanderToHand => {
            if let Some(question) = super::top_card::take_commander(state, res, chosen) {
                return next_choice(state, res, since, question);
            }
        }
        AwaitingOp::ControlChoice(choice) => {
            super::control_choice::finish(state, &choice, chosen);
        }
        AwaitingOp::SacrificeChosen { player } => {
            if let Some(&victim) = chosen.first()
                && let Some(_owner) = state
                    .object(victim)
                    .filter(|o| o.zone == crate::zone::Zone::Battlefield && o.controller == player)
                    .map(|o| o.owner)
            {
                let _ = state.sacrifice_object(victim, Cause::Effect);
            }
        }
        AwaitingOp::SearchLibrary {
            inspected: _,
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
            inspected: _,
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
            search::finish_shuffle(state, res, library);
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
            state.entries.set_putter(receiver);
            for (card, find) in placed {
                let (dest, tapped) = (find.dest, find.tapped);
                match dest {
                    SearchDest::Graveyard | SearchDest::Exile => {
                        search::put_found(state, receiver, card, dest);
                    }
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
                        if let Some((kind, n)) = find.counter {
                            crate::entry::put_counters(state, res.controller, card, kind, n);
                        }
                        if let Some(reference) = crate::entry::battlefield_reference(state, card) {
                            res.subject.found.push(reference);
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
                label: None,
            });
        }
        AwaitingOp::InspectHand => {}
        AwaitingOp::DirectedSearch(selection) => {
            return directed_search::resume(state, res, *selection, chosen);
        }
        AwaitingOp::RepeatedGraveyardChoice(selection) => {
            if let Some(pending) = repeated_graveyard_choice::chosen(state, res, *selection, chosen)
            {
                return next_choice(state, res, since, pending);
            }
        }
        AwaitingOp::HandToTopPlace { player } => {
            if let Some(pending) = hand_transfer::place(state, res, player, chosen) {
                return next_choice(state, res, since, pending);
            }
        }
        AwaitingOp::PrivateTopPick { looked } => {
            if let Some(pending) = private_library::picked(state, res, looked, chosen) {
                return next_choice(state, res, since, pending);
            }
        }
        AwaitingOp::HandToTopChoice { player } => {
            if let Some(pending) = private_library::chosen_hand_to_top(state, res, player, chosen) {
                return next_choice(state, res, since, pending);
            }
        }
        AwaitingOp::PrivateHandDiscard {
            inspected,
            player,
            options,
            remaining,
            selected,
        } => {
            if let Some(pending) = private_hand::discard_chosen(
                state, res, inspected, player, options, remaining, selected, chosen,
            ) {
                return next_choice(state, res, since, pending);
            }
        }
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
        AwaitingOp::ChosenGraveyard {
            destination,
            tapped,
        } => {
            if let Some(pending) = graveyard_choice::place(state, res, chosen, destination, tapped)
            {
                return next_choice(state, res, since, pending);
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
        AwaitingOp::DivideLeftRight {
            asked,
            rest,
            left,
            right,
        } => {
            return super::river::resume_divide(
                state,
                res,
                since,
                (asked, rest, left, right),
                chosen,
            );
        }
        AwaitingOp::LabelAttacker { .. } => {
            unreachable!("a label is a pile, answered via resume_pile")
        }
        AwaitingOp::BlockAttacker { blocker } => {
            // The answer names an attacker still attacking, and the blocker
            // is still on the battlefield, or nothing happens.
            if let Some(&attacker) = chosen.first()
                && state.combat.is_attacking(attacker)
                && state
                    .object(blocker)
                    .is_some_and(|o| o.zone == crate::zone::Zone::Battlefield)
                && !state.combat.is_blocking(blocker, attacker)
            {
                state.declare_block(blocker, attacker);
                // A block on one member of a band blocks the band
                // (CR 702.22h).
                for mate in state.combat.band_mates(attacker) {
                    if !state.combat.is_blocking(blocker, mate) {
                        state.declare_block(blocker, mate);
                    }
                }
            }
        }
        AwaitingOp::DiscardThenDraw => {
            // "If you do, draw that many": what was discarded, counted as it
            // happens — a card that is no longer in the hand is not.
            let you = res.controller;
            let held: Vec<(ObjectId, PlayerId)> = chosen
                .iter()
                .filter(|&&card| state.zones.list(ZoneLocation::Hand(you)).contains(&card))
                .map(|&card| (card, you))
                .collect();
            // Library of Leng, before anything moves; the answer comes back
            // here with the same cards.
            if let Some(question) = super::discard::ask_discard_destination(
                state,
                res,
                &held,
                DiscardThen::Choice {
                    op: Box::new(AwaitingOp::DiscardThenDraw),
                    chosen: chosen.to_vec(),
                },
            ) {
                return next_choice(state, res, since, question);
            }
            let mut discarded = 0;
            for &(card, _) in &held {
                state.discard_card(card, you, Cause::Effect, Some(res.controller));
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
        AwaitingOp::PutOntoBattlefield {
            tapped,
            attach_to_source_if_equipment,
        } => {
            for &card in chosen {
                if tapped {
                    state.set_tapped(card, true);
                }
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
                if attach_to_source_if_equipment
                    && state.object(card).is_some_and(|o| {
                        o.characteristics()
                            .subtypes
                            .contains(baylee_core::generated::subtypes::artifact::EQUIPMENT)
                    })
                    && let Some(equip_ref) = state.source_identity(card)
                    && let Some(source) =
                        subjects::source(state, res).filter(|s| subjects::on_battlefield(state, *s))
                {
                    super::attachments::attach_to(state, equip_ref, source.object, false);
                }
            }
        }
        AwaitingOp::DestroyAllExcept { filter } => {
            let you = res.controller;
            let doomed: Vec<ObjectId> = state
                .battlefield_seen()
                .filter(|id| {
                    !chosen.contains(id)
                        && state.object(*id).is_some_and(|o| {
                            crate::eval::matches_with_context(
                                filter,
                                state,
                                o,
                                you,
                                res.rule_context(),
                            )
                        })
                })
                .collect();
            crate::sba::destroy_all(state, &doomed, false);
        }
        AwaitingOp::MoveAura { aura } => {
            if let Some(&host) = chosen.first()
                && let Some(source) = state.source_identity(aura)
                && crate::aura_bindings::can_attach(state, aura, host)
            {
                super::attachments::attach_to(state, source, host, false);
            }
        }
        AwaitingOp::UntapPayingEach { player, each } => {
            let chosen: Vec<(ObjectId, u32)> = chosen
                .iter()
                .filter_map(|id| state.object(*id).map(|o| (*id, o.version)))
                .collect();
            if !chosen.is_empty() {
                let times = u32::try_from(chosen.len()).unwrap_or(u32::MAX);
                let cost = (0..times).fold(baylee_core::mana::ManaCost::ZERO, |total, _| {
                    total.combine(&each)
                });
                res.awaiting = Some(AwaitingOp::PlayerMayPay {
                    player,
                    cost,
                    outcome: super::ManaPaymentOutcome::Untap { chosen },
                });
                return next_choice(
                    state,
                    res,
                    since,
                    Pending::YesNo {
                        player,
                        prompt: crate::choice::YesNoPrompt::PayMana { cost },
                        source: super::resolving_ability(state, res),
                    },
                );
            }
        }
        AwaitingOp::AttackingTokens(tokens) => {
            if let Some(pending) =
                super::attacking_tokens::resume_walker(state, res, *tokens, chosen)
            {
                return next_choice(state, res, since, pending);
            }
        }
        AwaitingOp::BlockedReturns(returns) => {
            if let Some(pending) = super::blocked_returns::resume(state, res, *returns, chosen) {
                return next_choice(state, res, since, pending);
            }
        }
        AwaitingOp::HalfSpell(spells) => {
            super::half_damage::resume(state, res, &spells, chosen);
        }
        AwaitingOp::ReturnFromGraveyardOf => {
            if let Some(&card) = chosen.first() {
                super::reanimation::return_card(state, card, res.controller, true, None);
            }
        }
        AwaitingOp::RevealPut(reveal) => {
            if let Some(pending) = super::reveal_put::resume(state, res, *reveal, chosen) {
                return next_choice(state, res, since, pending);
            }
        }
        AwaitingOp::ChoosePlayerFrom => {
            unreachable!("a chosen player resumes through resume_chosen_player")
        }
        AwaitingOp::HandToBottom => {
            // In the order chosen: the owner orders cards put into a library
            // together (CR 401.4). A card no longer in the hand is not put.
            for &card in chosen {
                if state
                    .zones
                    .list(ZoneLocation::Hand(res.controller))
                    .contains(&card)
                {
                    let _ = state.move_object(
                        card,
                        ZoneLocation::Library(res.controller),
                        ZonePosition::Bottom,
                        Cause::Effect,
                    );
                }
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
        AwaitingOp::DigRest {
            found,
            rest,
            reveal,
            random,
        } => {
            if reveal && !chosen.is_empty() {
                state.journal.record(GameEvent::Revealed {
                    player: res.controller,
                    cards: chosen.to_vec(),
                });
            }
            let placed: Vec<_> = chosen
                .iter()
                .map(|&card| {
                    (
                        card,
                        search::resolve_find(state, res, res.controller, found, card),
                    )
                })
                .collect();
            let moves: Vec<_> = placed
                .iter()
                .map(|&(card, find)| {
                    let destination = match find.dest {
                        SearchDest::Graveyard => ZoneLocation::Graveyard(res.controller),
                        SearchDest::Exile => ZoneLocation::Exile(
                            state.object(card).map_or(res.controller, |o| o.owner),
                        ),
                        SearchDest::Hand => ZoneLocation::Hand(res.controller),
                        SearchDest::TopOfLibrary => ZoneLocation::Library(res.controller),
                        SearchDest::Battlefield => ZoneLocation::Battlefield,
                    };
                    (card, destination)
                })
                .collect();
            if let Some(pending) = ask_commander_replace(state, res, &moves) {
                let Some(AwaitingOp::CommanderReplace { asked, remaining }) = res.awaiting.take()
                else {
                    unreachable!("replacement question retains its owners")
                };
                res.awaiting = Some(AwaitingOp::SelectedCommander {
                    operation: Box::new(AwaitingOp::DigRest {
                        found,
                        rest,
                        reveal: false,
                        random,
                    }),
                    chosen: chosen.to_vec(),
                    asked,
                    remaining,
                });
                return Flow::Wait(pending);
            }
            for (card, find) in placed {
                if find.tapped && find.dest == SearchDest::Battlefield {
                    state.restore_tapped(card, true);
                }
                put_found(state, res.controller, card, find.dest);
                if find.dest == SearchDest::Battlefield
                    && let Some((kind, count)) = find.counter
                {
                    crate::entry::put_counters(state, res.controller, card, kind, count);
                }
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
        AwaitingOp::SearchTakeover { agent, library, .. } => {
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
            search::finish_shuffle(state, res, library);
        }
        AwaitingOp::Equalize(selection) => {
            if let Some(pending) = equalize::resume(state, res, *selection, chosen) {
                return next_choice(state, res, since, pending);
            }
        }
        AwaitingOp::ExileTokensCreatedWithThisForCounters { source, kind } => {
            let mut exiled = 0_u32;
            for &id in chosen {
                if state.object(id).is_some_and(|o| {
                    o.zone == crate::zone::Zone::Battlefield
                        && !o.status.contains(crate::object::Status::PHASED_OUT)
                        && o.card.is_none()
                        && o.riders.contains(&crate::object::Rider::CreatedBy(source))
                }) && state
                    .move_object(
                        id,
                        ZoneLocation::Exile(res.controller),
                        ZonePosition::Top,
                        Cause::Effect,
                    )
                    .is_ok()
                {
                    exiled += 1;
                }
            }
            if exiled > 0 && subjects::on_battlefield(state, source) {
                crate::replacement::put_counters_by(
                    state,
                    res.controller,
                    source.object,
                    kind,
                    u16::try_from(exiled).unwrap_or(u16::MAX),
                );
            }
        }
        AwaitingOp::Eureka {
            player,
            remaining_in_round,
            mut any_put_this_round,
            filter,
        } => {
            if let Some(&card) = chosen.first()
                && state.object(card).is_some_and(|o| {
                    o.zone == crate::zone::Zone::Hand
                        && o.zone_owner == Some(player)
                        && o.is_card()
                        && o.characteristics().types.is_permanent()
                        && eval::matches_with_context(filter, state, o, player, res.rule_context())
                })
                && !state.has_left(player)
            {
                if let Some(obj) = state.object_mut(card) {
                    obj.kind = ObjectKind::Permanent;
                    obj.set_controller(player);
                }
                state
                    .entries
                    .begin_event(state.journal.last_seq(), Some(player));
                let _ = state.move_object(
                    card,
                    ZoneLocation::Battlefield,
                    ZonePosition::Top,
                    Cause::Effect,
                );
                any_put_this_round = true;
            }
            if let Some(pending) =
                choices::advance_eureka(state, res, remaining_in_round, any_put_this_round, filter)
            {
                return next_choice(state, res, since, pending);
            }
        }
        AwaitingOp::DiscardChain {
            player,
            count,
            remaining,
            selected,
            draws,
        } => {
            return super::discard::resume_chain(
                state, res, player, count, remaining, selected, draws, chosen, since,
            );
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
                if let Some(obj) = state.object_mut(victim) {
                    obj.kind = ObjectKind::Card;
                }
                let _ = state.sacrifice_object(victim, Cause::Effect);
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
        AwaitingOp::SacrificeOneThen { if_it_was, then } => {
            if let Some(&victim) = chosen.first() {
                // Asked before it moves: what it was on the battlefield
                // (CR 608.2h), not the card it becomes in the graveyard.
                let was = state.object(victim).is_some_and(|o| {
                    eval::matches_with_context(
                        if_it_was,
                        state,
                        o,
                        res.controller,
                        res.rule_context(),
                    )
                });
                let before = state
                    .object(victim)
                    .filter(|object| {
                        object.zone == crate::zone::Zone::Battlefield
                            && object.controller == res.controller
                    })
                    .map(|object| (object.version, crate::object::SacrificedLki::of(object)));
                if let Some((version, lki)) = before {
                    let paid = state.sacrifice_object(victim, Cause::Effect).is_ok()
                        && state
                            .object(victim)
                            .is_none_or(|object| object.version != version);
                    if paid {
                        res.subject.sacrificed = Some(lki);
                    }
                    if paid
                        && was
                        && let Some(pending) = run_nested(state, res, then)
                    {
                        return next_choice(state, res, since, pending);
                    }
                }
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
        AwaitingOp::TakklemaggotAttach { aura } => {
            if let Some(&host) = chosen.first()
                && subjects::is_current(state, aura)
                && state
                    .object(aura.object)
                    .is_some_and(|o| o.zone == crate::zone::Zone::Graveyard && o.is_card())
                && crate::aura_bindings::can_attach(state, aura.object, host)
            {
                if let Some(obj) = state.object_mut(aura.object) {
                    let owner = obj.owner;
                    obj.kind = crate::object::ObjectKind::Permanent;
                    obj.set_controller(owner);
                    obj.attached_to = Some(host);
                }
                state.set_tapped(aura.object, false);
                let _ = state.move_object(
                    aura.object,
                    ZoneLocation::Battlefield,
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
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
        AwaitingOp::DiscardUpToForPlayers {
            asked,
            remaining,
            mut discards,
            max,
        } => {
            discards.extend(chosen.iter().take(usize::from(max)).map(|&c| (c, asked)));
            if let Some(pending) =
                super::immediate::discard_up_to_advance(state, res, remaining, discards, max)
            {
                return next_choice(state, res, since, pending);
            }
        }
        AwaitingOp::AnyPlayerSacrificeOrDamage {
            asked,
            remaining,
            step,
            filter,
            count,
            damage,
            effects,
        } => {
            if step == 1
                && chosen.len() == usize::from(count)
                && chosen
                    .iter()
                    .enumerate()
                    .all(|(i, id)| !chosen[..i].contains(id))
                && chosen.iter().all(|&id| {
                    state.object(id).is_some_and(|o| {
                        o.zone == crate::zone::Zone::Battlefield
                            && o.controller == asked
                            && eval::matches_with_context(
                                filter,
                                state,
                                o,
                                asked,
                                res.rule_context(),
                            )
                    })
                })
            {
                sacrifice_together(state, chosen);
                return super::answers::run_fallback(state, res, effects);
            }
            if let Some(pending) = super::immediate::any_player_sac_or_damage_advance(
                state, res, remaining, filter, count, damage, effects,
            ) {
                return next_choice(state, res, since, pending);
            }
        }
        AwaitingOp::RaidingParty {
            asked,
            remaining,
            step,
            tap_count,
            mut protected,
        } => {
            if step == 0 {
                let mut tapped_count = 0u8;
                for &id in chosen {
                    if state.object(id).is_some_and(|o| {
                        o.zone == crate::zone::Zone::Battlefield
                            && o.controller == asked
                            && !o.status.contains(crate::object::Status::TAPPED)
                            && o.characteristics()
                                .types
                                .contains(baylee_core::types::TypeSet::CREATURE)
                            && o.characteristics()
                                .colors
                                .contains(baylee_core::color::Color::White)
                    }) {
                        tap(state, id);
                        tapped_count = tapped_count.saturating_add(1);
                    }
                }
                if tapped_count > 0 {
                    let plains: Vec<ObjectId> = state
                        .battlefield_seen()
                        .filter(|&id| {
                            state.object(id).is_some_and(|o| {
                                o.characteristics()
                                    .subtypes
                                    .contains(baylee_core::generated::subtypes::land::PLAINS)
                            })
                        })
                        .collect();
                    if !plains.is_empty() {
                        let max = (tapped_count.saturating_mul(2))
                            .min(u8::try_from(plains.len()).unwrap_or(u8::MAX));
                        res.awaiting = Some(AwaitingOp::RaidingParty {
                            asked,
                            remaining,
                            step: 1,
                            tap_count: tapped_count,
                            protected,
                        });
                        return next_choice(
                            state,
                            res,
                            since,
                            Pending::ChooseCards {
                                player: asked,
                                options: plains,
                                min: 0,
                                max,
                                prompt: ChoicePrompt::KeepLands,
                                total: None,
                            },
                        );
                    }
                }
            } else {
                let max_keep = usize::from(tap_count).saturating_mul(2);
                for &id in chosen.iter().take(max_keep) {
                    if state.object(id).is_some_and(|o| {
                        o.zone == crate::zone::Zone::Battlefield
                            && o.characteristics()
                                .subtypes
                                .contains(baylee_core::generated::subtypes::land::PLAINS)
                    }) && !protected.contains(&id)
                    {
                        protected.push(id);
                    }
                }
            }
            if let Some(pending) = choices::advance_raiding_party(state, res, remaining, protected)
            {
                return next_choice(state, res, since, pending);
            }
        }
        AwaitingOp::DamageShieldSource { .. }
        | AwaitingOp::ShieldFromChosenSource { .. }
        | AwaitingOp::RedirectFromChosenSource { .. } => {
            unreachable!("source decisions resume through resume_source")
        }
        AwaitingOp::Entry { .. }
        | AwaitingOp::GraveyardOrder { .. }
        | AwaitingOp::ReorderTopLibrary { .. }
        | AwaitingOp::DigBottom
        | AwaitingOp::HandToTop { .. }
        | AwaitingOp::Scry { .. }
        | AwaitingOp::DiscardDestination { .. }
        | AwaitingOp::Surveil
        | AwaitingOp::ExploreScry { .. }
        | AwaitingOp::Explore { .. } => {
            unreachable!("arrangements resume via resume_arranged")
        }
        AwaitingOp::ChooseCardName { .. }
        | AwaitingOp::GuessNamedCardInHand { .. }
        | AwaitingOp::ChooseNumber
        | AwaitingOp::RemoveCountersSelfCreateTokens { .. }
        | AwaitingOp::DungeonPlay(_)
        | AwaitingOp::Dungeon { .. }
        | AwaitingOp::MutatePosition
        | AwaitingOp::ChooseEffect { .. }
        | AwaitingOp::LinkedPlay { .. }
        | AwaitingOp::MaskedCast
        | AwaitingOp::BattleDefeatCast { .. }
        | AwaitingOp::ControlledCard { .. }
        | AwaitingOp::LandMana { .. }
        | AwaitingOp::TextReplacement { .. }
        | AwaitingOp::ControlRotation { .. }
        | AwaitingOp::ControlRecipient { .. }
        | AwaitingOp::Damage(_)
        | AwaitingOp::ManaForDamage { .. }
        | AwaitingOp::DamagePayment { .. }
        | AwaitingOp::SacrificeOpponent { .. }
        | AwaitingOp::SourceColor { .. }
        | AwaitingOp::ManaUnlessPaidColor { .. }
        | AwaitingOp::CounterPlacementChoice
        | AwaitingOp::ShieldCounterChoice
        | AwaitingOp::DamageClearingChoice
        | AwaitingOp::RegenerationChoice
        | AwaitingOp::ManaChoice { .. }
        | AwaitingOp::ProtectionColor { .. }
        | AwaitingOp::LandwalkChoice { .. }
        | AwaitingOp::Colors { .. }
        | AwaitingOp::MoveCounter { .. }
        | AwaitingOp::Counters { .. }
        | AwaitingOp::PayLifeOrTapSelf { .. }
        | AwaitingOp::PlayerMayPayLife { .. }
        | AwaitingOp::PlayerMayGetPoison { .. }
        | AwaitingOp::PlayerMayMill { .. }
        | AwaitingOp::PayEnergy { .. }
        | AwaitingOp::MaySearchBasicLand { .. }
        | AwaitingOp::MayDo { .. }
        | AwaitingOp::RepeatCoinLifeStake(_)
        | AwaitingOp::ChooseCoinRepeatPlayer(_)
        | AwaitingOp::CascadeCast { .. }
        | AwaitingOp::CastTarget { .. }
        | AwaitingOp::TopOrBottom { .. }
        | AwaitingOp::SelectedCommander { .. }
        | AwaitingOp::GraveyardToTopOrder { .. }
        | AwaitingOp::ChosenGraveyardCommander { .. }
        | AwaitingOp::Cleansing { .. }
        | AwaitingOp::ChoosePhaseToSkipThisTurn
        | AwaitingOp::FlipCoinsUntilLossOrStopCumulative { .. }
        | AwaitingOp::CoinFlipIgnore { .. }
        | AwaitingOp::RevealTopPlayerMayPut { .. }
        | AwaitingOp::CommanderReplace { .. } => {
            unreachable!("color/yes-no choices resume via their own functions")
        }
        AwaitingOp::PlayerMayPay { .. } => {
            unreachable!("tax choices resume via resume_tax_choice")
        }
        AwaitingOp::PlayerMayPayCost {
            on_payment,
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
            let receipt = chosen
                .first()
                .and_then(|id| state.object(*id))
                .filter(|_| matches!(cost, baylee_cards_dsl::CostPart::Sacrifice(_)))
                .map(|object| {
                    (
                        (object.id, object.version),
                        crate::object::SacrificedLki::of(object),
                    )
                });
            let paid = chosen
                .first()
                .is_some_and(|&chosen| cost_wizard::pay(state, player, cost, chosen).is_ok());
            if paid
                && let Some((sacrificed, lki)) = receipt
                && let Some(stack) = state.object_mut(res.on_stack)
            {
                let record = stack.paid.get_or_insert_with(Default::default);
                record.sacrificed = Some(sacrificed);
                record.sacrificed_lki = Some(lki);
            }
            if paid == on_payment {
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
        AwaitingOp::DamageShieldSource { kind } => {
            if let Some(chosen) = chosen {
                life::batch57::answer(state, res, kind, chosen);
            }
        }
        AwaitingOp::ShieldFromChosenSource {
            all_recipients,
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
                        protects: if all_recipients {
                            crate::prevention::Shielded::Everything
                        } else {
                            crate::prevention::Shielded::Player(you)
                        },
                        kind: if all_recipients {
                            crate::prevention::ShieldKind::AllFrom { source }
                        } else {
                            crate::prevention::ShieldKind::NextFrom {
                                source,
                                all_but: u32::from(all_but),
                                gain_life,
                                combat_only,
                            }
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
