//! Suspended player decisions and exact entry payments.
#[allow(clippy::wildcard_imports)]
use super::*;

pub(super) fn ask(
    state: &mut GameState,
    entry: Attempt,
    question: Question,
    pending: Pending,
) -> Pending {
    state.entries.pending.push_front(entry);
    state.entries.question = Some(question);
    pending
}

fn before_next_entry(state: &mut GameState) -> Option<Pending> {
    if let Some(pending) = crate::graveyard_order::pending(state) {
        state.entries.question = Some(Question::GraveyardOrder);
        return Some(pending);
    }
    equilibrium::entry_order(state)
}

fn accept_sacrifice(
    state: &mut GameState,
    entry: Attempt,
    options: Vec<ObjectId>,
    count: u8,
) -> Pending {
    let player = entry.controller;
    ask(
        state,
        entry,
        Question::AcceptSacrifice {
            options,
            count: u32::from(count),
        },
        Pending::YesNo {
            player,
            prompt: YesNoPrompt::SacrificeToEnter { count },
            source: None,
        },
    )
}

/// Run entry work until all moves have completed or a player must decide.
#[allow(clippy::too_many_lines)]
#[doc(hidden)]
pub fn advance(state: &mut GameState) -> Option<Pending> {
    // Co-arrivals are one instruction. Decisions and payments resume through
    // this entry point; committing each body needs no whole-board projection.
    if !state.entries.pending.is_empty() {
        state.refresh_characteristics();
    }
    'entries: loop {
        if let Some(pending) = crate::replacement::counter_order::pending(state) {
            return Some(pending);
        }
        if completed(state) {
            return None;
        }
        if let Some(pending) = before_next_entry(state) {
            return Some(pending);
        }
        let mut entry = state.entries.pending.pop_front()?;
        if discard_stale(state, &entry) {
            continue;
        }
        if let Some((question, pending)) = initial_question(state, &mut entry) {
            return Some(ask(state, entry, question, pending));
        }
        if entry.refusal.is_none() {
            while let Some(&modifier) = entry.modifiers.get(entry.next) {
                entry.next += 1;
                let player = entry.controller;
                if let Some((question, pending)) =
                    super::tapping::apply(state, &mut entry, modifier)
                {
                    return Some(ask(state, entry, question, pending));
                }
                if let Some((question, pending)) = super::forms::question(&entry, modifier) {
                    return Some(ask(state, entry, question, pending));
                }
                match modifier {
                    EnterModifier::ChooseNumber { .. } | EnterModifier::ChooseShape(_) => {
                        unreachable!("handled entry form")
                    }
                    EnterModifier::SacrificeOrGraveyard { filter, count } => {
                        let options = options(state, &entry, filter, false);
                        if options.len() < usize::from(count) {
                            entry.refusal = Some(Refusal::Graveyard);
                            break;
                        }
                        return Some(accept_sacrifice(state, entry, options, count));
                    }
                    EnterModifier::DiscardOrGraveyard(filter) => {
                        let options = options(state, &entry, filter, true);
                        if options.is_empty() {
                            entry.refusal = Some(Refusal::Graveyard);
                            break;
                        }
                        return Some(ask(
                            state,
                            entry,
                            Question::Discard,
                            Pending::ChooseCards {
                                player,
                                options,
                                min: 0,
                                max: 1,
                                prompt: ChoicePrompt::CostDiscard,
                                total: None,
                            },
                        ));
                    }
                    EnterModifier::SacrificeForPowerToughness(filter) => {
                        let options = options(state, &entry, filter, false);
                        entry.sacrifices = Some(0);
                        if options.is_empty() {
                            continue;
                        }
                        let max = u32::try_from(options.len()).expect("arena fits u32");
                        return Some(ask(
                            state,
                            entry,
                            Question::SacrificeCount { options },
                            Pending::ChooseNumber {
                                player,
                                min: 0,
                                max,
                                reason: crate::choice::NumberPrompt::Sacrifices,
                            },
                        ));
                    }
                    EnterModifier::SacrificeOthers(filter) => {
                        let victims = options(state, &entry, filter, false);
                        sacrifice(state, &victims);
                        state.refresh_characteristics();
                        state.entries.pending.push_front(entry);
                        continue 'entries;
                    }
                    EnterModifier::ExileFromGraveyardXOrGraveyard(filter) => {
                        let x = state
                            .object(entry.object)
                            .filter(|o| o.zone == Zone::Stack || entry.copy == CopyStatus::Handled)
                            .map_or(0, |o| o.x_value);
                        entry.sacrifices = Some(0);
                        if x == 0 {
                            continue;
                        }
                        let unavailable = unavailable(state);
                        let options: Vec<ObjectId> = state
                            .zones
                            .list(ZoneLocation::Graveyard(player))
                            .iter()
                            .copied()
                            .filter(|id| {
                                *id != entry.object
                                    && !unavailable.contains(id)
                                    && state.object(*id).is_some_and(|o| {
                                        crate::eval::matches(filter, state, o, player, entry.object)
                                    })
                            })
                            .collect();
                        if options.len() < x as usize {
                            entry.refusal = Some(Refusal::Graveyard);
                            break;
                        }
                        let count = u8::try_from(x.min(255)).expect("bounded page");
                        return Some(ask(
                            state,
                            entry,
                            Question::ExileFromGraveyard { count: x },
                            Pending::ChooseCards {
                                player,
                                options,
                                min: count,
                                max: count,
                                prompt: ChoicePrompt::CostExile,
                                total: None,
                            },
                        ));
                    }
                    EnterModifier::ChooseCounterPerEntryCount(kinds) => {
                        let count = entry.sacrifices.unwrap_or(0);
                        if count == 0 || kinds.is_empty() {
                            continue;
                        }
                        let max = u32::try_from(kinds.len() - 1).expect("bounded slice");
                        return Some(ask(
                            state,
                            entry,
                            Question::ChooseCounter {
                                kinds,
                                remaining: count,
                            },
                            Pending::ChooseNumber {
                                player,
                                min: 0,
                                max,
                                reason: crate::choice::NumberPrompt::Choice,
                            },
                        ));
                    }
                    EnterModifier::PayLifeUpTo(amount) => {
                        let x = state
                            .object(entry.object)
                            .filter(|o| o.zone == Zone::Stack || entry.copy == CopyStatus::Handled)
                            .map(|o| o.x_value);
                        let cap = crate::eval::amount(&amount, state, player, entry.object, x);
                        let life = state
                            .players
                            .get(usize::from(player.get()))
                            .map_or(0, |p| u32::try_from(p.life.max(0)).unwrap_or(0));
                        let max = if state.can_pay_life(player, 1) {
                            cap.min(life)
                        } else {
                            0
                        };
                        entry.sacrifices = Some(0);
                        if max == 0 {
                            continue;
                        }
                        return Some(ask(
                            state,
                            entry,
                            Question::PayLifeUpTo,
                            Pending::ChooseNumber {
                                player,
                                min: 0,
                                max,
                                reason: crate::choice::NumberPrompt::Choice,
                            },
                        ));
                    }
                    EnterModifier::Amplify(factor) => {
                        let options = amplify_options(state, &entry);
                        if options.is_empty() {
                            continue;
                        }
                        let max = u8::try_from(options.len().min(255)).expect("bounded page");
                        return Some(ask(
                            state,
                            entry,
                            Question::Amplify { factor },
                            Pending::ChooseCards {
                                player,
                                options,
                                min: 0,
                                max,
                                prompt: ChoicePrompt::Amplify,
                                total: None,
                            },
                        ));
                    }
                    _ => {}
                }
                state.entries.pending.push_front(entry);
                continue 'entries;
            }
        }
        if let Some(pending) = finish_or_suspend(state, entry) {
            return Some(pending);
        }
    }
}

fn finish_or_suspend(state: &mut GameState, mut entry: Attempt) -> Option<Pending> {
    if let Some((question, pending)) = super::auras::question(state, &mut entry) {
        return Some(ask(state, entry, question, pending));
    }
    if let Some((question, pending)) = super::battles::question(state, &mut entry) {
        state.entries.counter_transaction = true;
        return Some(ask(state, entry, question, pending));
    }
    state.entries.counter_transaction |= entry.battle_protector.is_some();
    if let Some((question, pending)) = super::sagas::question(state, &mut entry) {
        state.entries.counter_transaction = true;
        return Some(ask(state, entry, question, pending));
    }
    state.entries.counter_transaction |= entry.saga_chapter.is_some();
    if let Some(pending) = super::counters::question(state, &mut entry) {
        return Some(ask(state, entry, Question::Counters, pending));
    }
    state.entries.chosen_entry = None;
    if state.entries.counter_transaction {
        state.entries.ready.push_back(entry);
    } else {
        finish(state, entry);
    }
    None
}

/// The engine validates the offered seat, cardinality and membership before entering here.
#[allow(clippy::too_many_lines)]
#[doc(hidden)]
pub fn answer(state: &mut GameState, action: &PlayerAction) {
    let question = state.entries.question.take().expect("entry question");
    if equilibrium::answer_special(state, &question, action) {
        return;
    }
    if answer_graveyard(state, &question, action) {
        return;
    }
    if super::tapping::answer(state, &question, action) {
        return;
    }
    if super::forms::answer(state, &question, action) {
        return;
    }
    if let Question::Copy {
        index,
        mods,
        until_eot,
    } = question
    {
        answer_copy(state, action, index, mods, until_eot);
        return;
    }
    let mut entry = state
        .entries
        .pending
        .pop_front()
        .expect("entry continuation");
    match (question, action) {
        (Question::BattleProtector, PlayerAction::ChoosePlayer(player)) => {
            entry.battle_protector = Some(*player);
        }
        (Question::ReadAhead, PlayerAction::ChooseNumber(chapter)) => {
            entry.saga_chapter = Some(u8::try_from(*chapter).expect("bounded saga chapter"));
        }
        (Question::Counters, PlayerAction::YesNo(apply)) => {
            entry.counter_plans[entry.next_counter].answer(*apply);
        }
        (Question::AuraHost, PlayerAction::ChooseObjects { objects }) => {
            state.attach(entry.object, objects[0]);
        }
        (Question::AuraHost, PlayerAction::ChoosePlayer(player)) => {
            state.attach_to_player(entry.object, *player);
        }
        (Question::AuraHost, PlayerAction::ChooseTargets { objects, players }) => {
            if let Some(&host) = objects.first() {
                state.attach(entry.object, host);
            } else if let Some(&player) = players.first() {
                state.attach_to_player(entry.object, player);
            }
        }
        (Question::Number, PlayerAction::ChooseNumber(number)) => {
            entry.chosen_number = Some(u8::try_from(*number).expect("bounded choice"));
        }
        (Question::AcceptSacrifice { options, count }, PlayerAction::YesNo(accept)) => {
            if *accept {
                select(state, entry, options, count, Vec::new());
                return;
            }
            entry.refusal = Some(Refusal::Graveyard);
        }
        (Question::SacrificeCount { options }, PlayerAction::ChooseNumber(count)) => {
            entry.sacrifices = Some(*count);
            if *count > 0 {
                select(state, entry, options, *count, Vec::new());
                return;
            }
        }
        (
            Question::Sacrifice {
                mut options,
                remaining,
                mut chosen,
            },
            PlayerAction::ChooseObjects { objects },
        ) => {
            chosen.extend_from_slice(objects);
            let remaining = remaining - u32::try_from(objects.len()).expect("page fits u32");
            if remaining > 0 {
                let mut used = objects.clone();
                used.sort_unstable();
                options.retain(|id| used.binary_search(id).is_err());
                select(state, entry, options, remaining, chosen);
                return;
            }
            sacrifice(state, &chosen);
        }
        (Question::Discard, PlayerAction::ChooseObjects { objects }) => {
            if let Some(&card) = objects.first() {
                if state.discard_top_source(entry.controller).is_some()
                    && !state.discard_answered(card)
                {
                    state.entries.question = Some(Question::DiscardDestination { card });
                    state.entries.pending.push_front(entry);
                    return;
                }
                let player = entry.controller;
                state.discard_card(card, player, Cause::Effect, Some(player));
            } else {
                entry.refusal = Some(Refusal::Graveyard);
            }
        }
        (Question::DiscardDestination { card }, PlayerAction::Arrange { piles }) => {
            discard_to_destination(state, &entry, card, piles);
        }
        (Question::ExileFromGraveyard { count }, PlayerAction::ChooseObjects { objects }) => {
            for &card in objects {
                let owner = state.object(card).map_or(entry.controller, |o| o.owner);
                let _ = state.move_object(
                    card,
                    ZoneLocation::Exile(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            entry.sacrifices = Some(count);
        }
        (Question::ChooseCounter { kinds, remaining }, PlayerAction::ChooseNumber(choice)) => {
            let kind = kinds
                .get(*choice as usize)
                .copied()
                .unwrap_or_else(|| kinds[kinds.len() - 1]);
            entry.counters.push((Some(entry.controller), kind, 1));
            if remaining > 1 {
                state.entries.question = Some(Question::ChooseCounter {
                    kinds,
                    remaining: remaining - 1,
                });
                state.entries.pending.push_front(entry);
                return;
            }
        }
        (Question::PayLifeUpTo, PlayerAction::ChooseNumber(amount)) => {
            entry.sacrifices = Some(*amount);
            if *amount > 0 {
                let delta = -i32::try_from(*amount).unwrap_or(i32::MAX);
                state.change_life(entry.controller, delta, Cause::Effect);
            }
        }
        (Question::Amplify { factor }, PlayerAction::ChooseObjects { objects }) => {
            if !objects.is_empty() {
                state.journal.record(GameEvent::Revealed {
                    player: entry.controller,
                    cards: objects.clone(),
                });
                let revealed = u16::try_from(objects.len()).unwrap_or(u16::MAX);
                let count = u16::from(factor).saturating_mul(revealed);
                if count > 0 {
                    entry
                        .counters
                        .push((Some(entry.controller), CounterKind::P1P1, count));
                }
            }
        }
        _ => unreachable!("entry answer shape was validated"),
    }
    state.entries.pending.push_front(entry);
}

/// Hand cards eligible to be revealed for `EnterModifier::Amplify` (CR 702.38a,
/// CR 614.12): share at least one creature type with the prospective permanent
/// (including Changeling, CR 702.73a), excluding the entering object itself and
/// any co-entering cards in `unavailable(state)`.
fn amplify_options(state: &GameState, entry: &Attempt) -> Vec<ObjectId> {
    let Some(object) = state.object(entry.object) else {
        return Vec::new();
    };
    let excluded = unavailable(state);
    let mut prospective = object.clone();
    prospective.zone = Zone::Battlefield;
    prospective.version = prospective.version.wrapping_add(1);
    let plan = crate::layers::LayerPlan::before_entries(&state.effects, excluded);
    let projection = crate::layers::recompute_with(state, &prospective, &plan);
    let entering_types = projection
        .characteristics
        .subtypes
        .intersection(baylee_core::types::SubtypeSet::ALL_CREATURE);
    if entering_types.is_empty() {
        return Vec::new();
    }
    state
        .zones
        .list(ZoneLocation::Hand(entry.controller))
        .iter()
        .copied()
        .filter(|id| *id != entry.object && !excluded.contains(id))
        .filter(|id| {
            state.object(*id).is_some_and(|card| {
                let hand_proj = crate::layers::recompute_with(state, card, &plan);
                hand_proj
                    .characteristics
                    .subtypes
                    .intersects(entering_types)
            })
        })
        .collect()
}

fn discard_to_destination(
    state: &mut GameState,
    entry: &Attempt,
    card: ObjectId,
    piles: &[Vec<ObjectId>],
) {
    let top = piles.get(1).map_or(&[][..], Vec::as_slice);
    let graveyard = if top.contains(&card) {
        &[][..]
    } else {
        std::slice::from_ref(&card)
    };
    state.record_discard_answers(graveyard, top, &BTreeSet::new());
    state.discard_card(
        card,
        entry.controller,
        Cause::Effect,
        Some(entry.controller),
    );
}

pub(super) fn select(
    state: &mut GameState,
    entry: Attempt,
    options: Vec<ObjectId>,
    remaining: u32,
    chosen: Vec<ObjectId>,
) {
    state.entries.question = Some(Question::Sacrifice {
        options,
        remaining,
        chosen,
    });
    state.entries.pending.push_front(entry);
}

/// Re-publish a second question of the same entry after its previous answer.
#[doc(hidden)]
pub fn next_question(state: &GameState) -> Option<Pending> {
    let entry = state.entries.pending.front()?;
    let player = entry.controller;
    match state.entries.question.as_ref()? {
        Question::Sacrifice {
            options, remaining, ..
        } => {
            let count = u8::try_from((*remaining).min(255)).expect("bounded page");
            Some(Pending::ChooseCards {
                player,
                options: options.clone(),
                min: count,
                max: count,
                prompt: ChoicePrompt::CostSacrifice,
                total: None,
            })
        }
        Question::DiscardDestination { card } => Some(Pending::Arrange {
            player,
            cards: vec![*card],
            piles: vec![
                ArrangePile::up_to(ArrangePlace::Graveyard, 1),
                ArrangePile::up_to(ArrangePlace::LibraryTop, 1),
            ],
            prompt: ArrangePrompt::DiscardToLibrary,
        }),
        Question::ChooseCounter { kinds, .. } => Some(Pending::ChooseNumber {
            player,
            min: 0,
            max: u32::try_from(kinds.len().saturating_sub(1)).expect("bounded slice"),
            reason: crate::choice::NumberPrompt::Choice,
        }),
        _ => None,
    }
}

fn answer_graveyard(state: &mut GameState, question: &Question, action: &PlayerAction) -> bool {
    if let Question::GraveyardOrder = *question {
        let PlayerAction::Arrange { piles } = action else {
            unreachable!("graveyard arrangement")
        };
        crate::graveyard_order::answer(state, piles.first().map_or(&[], Vec::as_slice));
        return true;
    }
    false
}

fn discard_stale(state: &mut GameState, entry: &Attempt) -> bool {
    if state
        .object(entry.object)
        .is_none_or(|o| o.version != entry.version)
    {
        state.entries.pending_objects.remove(&entry.object);
        true
    } else {
        false
    }
}

fn completed(state: &mut GameState) -> bool {
    if !state.entries.pending.is_empty() {
        return false;
    }
    while let Some(entry) = state.entries.ready.pop_front() {
        finish(state, entry);
    }
    state.entries.counter_transaction = false;
    state.entries.pending_objects.clear();
    state.entries.order_prepared = false;
    state.entries.putter = None;
    state.entries.chosen_entry = None;
    true
}

fn initial_question(state: &mut GameState, entry: &mut Attempt) -> Option<(Question, Pending)> {
    copy_question(state, entry).or_else(|| {
        if entry.refusal.is_some() {
            return None;
        }
        equilibrium::replacement_order(state, entry).map(|pending| (Question::Replacement, pending))
    })
}
