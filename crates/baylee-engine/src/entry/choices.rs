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
        if let super::TributeState::Ask(opp, counters) = entry.tribute {
            entry.tribute = super::TributeState::None;
            let permanent = entry.object;
            return Some(ask(
                state,
                entry,
                Question::Tribute { counters },
                Pending::YesNo {
                    player: opp,
                    prompt: YesNoPrompt::Tribute {
                        permanent,
                        counters,
                    },
                    source: None,
                },
            ));
        }
        if !entry.is_cancelled() {
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
                    EnterModifier::Devour { filter, factor } => {
                        let options = options(state, &entry, filter, false);
                        entry.sacrifices = Some(entry.sacrifices.unwrap_or(0));
                        if options.is_empty() {
                            continue;
                        }
                        let max = u8::try_from(options.len().min(255)).expect("bounded page");
                        return Some(ask(
                            state,
                            entry,
                            Question::Devour { factor },
                            Pending::ChooseCards {
                                player,
                                options,
                                min: 0,
                                max,
                                prompt: ChoicePrompt::Devour { factor },
                                total: None,
                            },
                        ));
                    }
                    EnterModifier::Tribute(counters) => {
                        let opponents = crate::eval::players(
                            baylee_cards_dsl::PlayerRel::Opponent,
                            state,
                            player,
                        )
                        .unwrap_or_default();
                        match opponents.as_slice() {
                            [] => continue,
                            [opp] => {
                                let permanent = entry.object;
                                return Some(ask(
                                    state,
                                    entry,
                                    Question::Tribute { counters },
                                    Pending::YesNo {
                                        player: *opp,
                                        prompt: YesNoPrompt::Tribute {
                                            permanent,
                                            counters,
                                        },
                                        source: None,
                                    },
                                ));
                            }
                            _ => {
                                return Some(ask(
                                    state,
                                    entry,
                                    Question::TributeOpponent { counters },
                                    Pending::ChoosePlayer {
                                        player,
                                        options: opponents,
                                    },
                                ));
                            }
                        }
                    }
                    EnterModifier::ChooseSubtype
                    | EnterModifier::ChooseSubtypeFrom(_)
                    | EnterModifier::ChooseBasicLandType
                        if super::has_pre_counter_choice(entry.modifiers) =>
                    {
                        let options = match modifier {
                            EnterModifier::ChooseBasicLandType => {
                                use baylee_core::generated::subtypes::land::{
                                    FOREST, ISLAND, MOUNTAIN, PLAINS, SWAMP,
                                };
                                vec![PLAINS, ISLAND, SWAMP, MOUNTAIN, FOREST]
                            }
                            EnterModifier::ChooseSubtypeFrom(types) => types.to_vec(),
                            _ => baylee_core::generated::subtypes::ALL_CREATURE_TYPES
                                .iter()
                                .collect(),
                        };
                        return Some(ask(
                            state,
                            entry,
                            Question::EntrySubtype,
                            Pending::ChooseSubtype { player, options },
                        ));
                    }
                    EnterModifier::ChooseMode(labels) => {
                        if labels.is_empty() {
                            continue;
                        }
                        let object = entry.object;
                        let options = (0..labels.len())
                            .map(|index| crate::choice::CastModeDesc {
                                index: u32::try_from(index).expect("mode fits u32"),
                                kind: crate::choice::CastModeKind::Mode(index),
                                cost: baylee_core::mana::ManaCost::ZERO,
                            })
                            .collect();
                        return Some(ask(
                            state,
                            entry,
                            Question::ChooseMode {
                                count: u8::try_from(labels.len()).expect("mode fits u8"),
                            },
                            Pending::ChooseCastMode {
                                player,
                                object,
                                options,
                            },
                        ));
                    }
                    EnterModifier::EachPlayerChooseMode(labels) => {
                        if labels.is_empty() {
                            continue;
                        }
                        let object = entry.object;
                        let mut seats = crate::eval::players(
                            baylee_cards_dsl::PlayerRel::EachPlayer,
                            state,
                            player,
                        )
                        .unwrap_or_default();
                        seats.sort_by_key(|&p| crate::battle::sba::apnap_distance(state, p));
                        if seats.is_empty() {
                            continue;
                        }
                        let first = seats.remove(0);
                        let options = (0..labels.len())
                            .map(|index| crate::choice::CastModeDesc {
                                index: u32::try_from(index).expect("mode fits u32"),
                                kind: crate::choice::CastModeKind::Mode(index),
                                cost: baylee_core::mana::ManaCost::ZERO,
                            })
                            .collect();
                        return Some(ask(
                            state,
                            entry,
                            Question::EachPlayerChooseMode {
                                player: first,
                                remaining: seats,
                                count: u8::try_from(labels.len()).expect("mode fits u8"),
                            },
                            Pending::ChooseCastMode {
                                player: first,
                                object,
                                options,
                            },
                        ));
                    }
                    EnterModifier::ChooseOpponent | EnterModifier::ChooseAnyPlayer
                        if super::has_pre_counter_choice(entry.modifiers) =>
                    {
                        let rel = if matches!(modifier, EnterModifier::ChooseOpponent) {
                            baylee_cards_dsl::PlayerRel::Opponent
                        } else {
                            baylee_cards_dsl::PlayerRel::EachPlayer
                        };
                        let options = crate::eval::players(rel, state, player).unwrap_or_default();
                        if options.is_empty() {
                            continue;
                        }
                        return Some(ask(
                            state,
                            entry,
                            Question::EntryPlayer,
                            Pending::ChoosePlayer { player, options },
                        ));
                    }
                    EnterModifier::EntersUnderChosenOpponentControl => {
                        if entry.applied_own_control {
                            continue;
                        }
                        if let Some((question, pending)) =
                            equilibrium::apply_own_control(state, &mut entry)
                        {
                            return Some(ask(state, entry, question, pending));
                        }
                    }
                    EnterModifier::ChooseNumberAtRandom { min, max } => {
                        if min <= max {
                            let span = u64::from(max - min) + 1;
                            let picked = min + u8::try_from(state.rng.below(span)).unwrap_or(0);
                            entry.chosen_number = Some(picked);
                        }
                        continue;
                    }
                    EnterModifier::SecretlyChooseNumber { min, max } => {
                        if min > max {
                            continue;
                        }
                        return Some(ask(
                            state,
                            entry,
                            Question::SecretNumber,
                            Pending::ChooseNumber {
                                player,
                                min: u32::from(min),
                                max: u32::from(max),
                                reason: crate::choice::NumberPrompt::Choice,
                            },
                        ));
                    }
                    EnterModifier::SecretlyChooseOpponent => {
                        let options = crate::eval::players(
                            baylee_cards_dsl::PlayerRel::Opponent,
                            state,
                            player,
                        )
                        .unwrap_or_default();
                        if options.is_empty() {
                            continue;
                        }
                        return Some(ask(
                            state,
                            entry,
                            Question::SecretOpponent,
                            Pending::ChoosePlayer { player, options },
                        ));
                    }
                    EnterModifier::ChooseTwoPlayers => {
                        let options = crate::eval::players(
                            baylee_cards_dsl::PlayerRel::EachPlayer,
                            state,
                            player,
                        )
                        .unwrap_or_default();
                        match options.as_slice() {
                            [] => continue,
                            [only] => {
                                entry.chosen_player = Some(*only);
                                continue;
                            }
                            _ => {
                                return Some(ask(
                                    state,
                                    entry,
                                    Question::EntryFirstOfTwoPlayers,
                                    Pending::ChoosePlayer { player, options },
                                ));
                            }
                        }
                    }
                    EnterModifier::OpponentChoosesCreatureType => {
                        let opponents = crate::eval::players(
                            baylee_cards_dsl::PlayerRel::Opponent,
                            state,
                            player,
                        )
                        .unwrap_or_default();
                        match opponents.as_slice() {
                            [] => continue,
                            [opp] => {
                                let options: Vec<_> = baylee_core::types::SubtypeSet::ALL_CREATURE
                                    .iter()
                                    .collect();
                                return Some(ask(
                                    state,
                                    entry,
                                    Question::EntrySubtypeBy(*opp),
                                    Pending::ChooseSubtype {
                                        player: *opp,
                                        options,
                                    },
                                ));
                            }
                            _ => {
                                return Some(ask(
                                    state,
                                    entry,
                                    Question::EntryOpponentForSubtype,
                                    Pending::ChoosePlayer {
                                        player,
                                        options: opponents,
                                    },
                                ));
                            }
                        }
                    }
                    EnterModifier::NoteMostPrevalentCreatureTypeInOpponentLibrary => {
                        let opponents = crate::eval::players(
                            baylee_cards_dsl::PlayerRel::Opponent,
                            state,
                            player,
                        )
                        .unwrap_or_default();
                        match opponents.as_slice() {
                            [] => continue,
                            [opp] => {
                                if let Some((q, p)) =
                                    note_most_prevalent_creature_type(state, &mut entry, *opp)
                                {
                                    return Some(ask(state, entry, q, p));
                                }
                                continue;
                            }
                            _ => {
                                return Some(ask(
                                    state,
                                    entry,
                                    Question::EntryOpponentLibraryType,
                                    Pending::ChoosePlayer {
                                        player,
                                        options: opponents,
                                    },
                                ));
                            }
                        }
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
    if let Some((question, pending)) = keyword_entry_question(state, &mut entry) {
        return Some(ask(state, entry, question, pending));
    }
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
        (Question::Riot, PlayerAction::YesNo(counter)) => {
            if *counter {
                entry
                    .counters
                    .push((Some(entry.controller), CounterKind::P1P1, 1));
            } else {
                entry.kw_choices.riot_haste = true;
            }
        }
        (Question::Unleash, PlayerAction::YesNo(counter)) => {
            if *counter {
                entry
                    .counters
                    .push((Some(entry.controller), CounterKind::P1P1, 1));
            }
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
        (Question::ChooseMode { .. }, PlayerAction::ChooseMode(index)) => {
            entry.chosen_mode = Some(u8::try_from(*index).expect("mode fits u8"));
        }
        (
            Question::EachPlayerChooseMode {
                player: chooser,
                mut remaining,
                count,
            },
            PlayerAction::ChooseMode(index),
        ) => {
            entry
                .player_chosen_modes
                .push((chooser, u8::try_from(*index).expect("mode fits u8")));
            remaining.retain(|&p| !state.has_left(p));
            if !remaining.is_empty() {
                let next_player = remaining.remove(0);
                state.entries.question = Some(Question::EachPlayerChooseMode {
                    player: next_player,
                    remaining,
                    count,
                });
                state.entries.pending.push_front(entry);
                return;
            }
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
        (Question::Devour { factor }, PlayerAction::ChooseObjects { objects }) => {
            let sacrificed = u32::try_from(objects.len()).unwrap_or(u32::MAX);
            entry.sacrifices = Some(entry.sacrifices.unwrap_or(0).saturating_add(sacrificed));
            if !objects.is_empty() {
                sacrifice(state, objects);
                let raw = if factor == 0 {
                    sacrificed.saturating_mul(sacrificed)
                } else {
                    u32::from(factor).saturating_mul(sacrificed)
                };
                let counters = u16::try_from(raw).unwrap_or(u16::MAX);
                if counters > 0 {
                    entry
                        .counters
                        .push((Some(entry.controller), CounterKind::P1P1, counters));
                }
                state.refresh_characteristics();
            }
        }
        (Question::TributeOpponent { counters }, PlayerAction::ChoosePlayer(opp)) => {
            entry.tribute = super::TributeState::Ask(*opp, counters);
        }
        (Question::Tribute { counters }, PlayerAction::YesNo(paid)) => {
            if *paid {
                entry.tribute = super::TributeState::Paid;
                if counters > 0 {
                    entry.counters.push((None, CounterKind::P1P1, counters));
                }
            }
        }
        (
            Question::EntrySubtype | Question::EntrySubtypeBy(_) | Question::EntrySubtypeTie { .. },
            PlayerAction::ChooseSubtype(sub),
        ) => {
            entry.chosen_subtype = Some(*sub);
            if let Some(obj) = state.object_mut(entry.object) {
                obj.chosen_subtype = Some(*sub);
            }
        }
        (Question::EntryPlayer, PlayerAction::ChoosePlayer(chosen)) => {
            entry.chosen_player = Some(*chosen);
            if let Some(obj) = state.object_mut(entry.object) {
                obj.set_chosen_opponent(Some(*chosen));
            }
        }
        (Question::EntryControllerOpponent, PlayerAction::ChoosePlayer(opp)) => {
            entry.controller_override = Some(*opp);
            entry.controller = *opp;
        }
        (Question::SecretNumber, PlayerAction::ChooseNumber(n)) => {
            entry.secret_chosen_number = Some(u8::try_from(*n).unwrap_or(u8::MAX));
        }
        (Question::SecretOpponent, PlayerAction::ChoosePlayer(chosen)) => {
            entry.secret_chosen_player = Some(*chosen);
        }
        (Question::EntryFirstOfTwoPlayers, PlayerAction::ChoosePlayer(first)) => {
            entry.chosen_player = Some(*first);
            let remaining: Vec<PlayerId> = crate::eval::players(
                baylee_cards_dsl::PlayerRel::EachPlayer,
                state,
                entry.controller,
            )
            .unwrap_or_default()
            .into_iter()
            .filter(|&p| p != *first)
            .collect();
            match remaining.as_slice() {
                [] => {}
                [only] => {
                    entry.second_chosen_player = Some(*only);
                }
                _ => {
                    state.entries.question =
                        Some(Question::EntrySecondOfTwoPlayers { first: *first });
                }
            }
        }
        (Question::EntrySecondOfTwoPlayers { .. }, PlayerAction::ChoosePlayer(second)) => {
            entry.second_chosen_player = Some(*second);
        }
        (Question::EntryOpponentForSubtype, PlayerAction::ChoosePlayer(opp)) => {
            state.entries.question = Some(Question::EntrySubtypeBy(*opp));
        }
        (Question::EntryOpponentLibraryType, PlayerAction::ChoosePlayer(opp)) => {
            if let Some((q, _pending)) = note_most_prevalent_creature_type(state, &mut entry, *opp)
            {
                state.entries.question = Some(q);
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
    let prospective = super::prospective_battlefield(object);
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
        Question::EachPlayerChooseMode {
            player: chooser,
            count,
            ..
        } => Some(Pending::ChooseCastMode {
            player: *chooser,
            object: entry.object,
            options: (0..usize::from(*count))
                .map(|index| crate::choice::CastModeDesc {
                    index: u32::try_from(index).expect("mode fits u32"),
                    kind: crate::choice::CastModeKind::Mode(index),
                    cost: baylee_core::mana::ManaCost::ZERO,
                })
                .collect(),
        }),
        Question::EntrySecondOfTwoPlayers { first } => {
            let options: Vec<PlayerId> =
                crate::eval::players(baylee_cards_dsl::PlayerRel::EachPlayer, state, player)
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|&p| p != *first)
                    .collect();
            (!options.is_empty()).then_some(Pending::ChoosePlayer { player, options })
        }
        Question::EntrySubtypeBy(opp) => Some(Pending::ChooseSubtype {
            player: *opp,
            options: baylee_core::types::SubtypeSet::ALL_CREATURE
                .iter()
                .collect(),
        }),
        Question::EntrySubtypeTie { options } => Some(Pending::ChooseSubtype {
            player,
            options: options.clone(),
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
        if entry.is_cancelled() {
            return None;
        }
        equilibrium::replacement_order(state, entry)
    })
}

fn keyword_entry_question(
    state: &mut GameState,
    entry: &mut Attempt,
) -> Option<(Question, Pending)> {
    if entry.is_cancelled() {
        return None;
    }
    if !entry.kw_choices.initialized {
        entry.kw_choices.initialized = true;
        if super::may_have_keyword_entry(state, entry.object)
            && let Some(object) = state.object(entry.object)
        {
            let prospective = super::prospective_battlefield(object);
            let plan = crate::layers::LayerPlan::before_entries(&state.effects, unavailable(state));
            let riot = crate::layers::keyword_instances_with_plan(
                state,
                &prospective,
                baylee_cards_dsl::KeywordSet::RIOT,
                &plan,
            );
            let unleash = crate::layers::keyword_instances_with_plan(
                state,
                &prospective,
                baylee_cards_dsl::KeywordSet::UNLEASH,
                &plan,
            );
            entry.kw_choices.riot_remaining = u8::try_from(riot).unwrap_or(u8::MAX);
            entry.kw_choices.unleash_remaining = u8::try_from(unleash).unwrap_or(u8::MAX);
        }
    }
    if entry.kw_choices.riot_remaining > 0 {
        entry.kw_choices.riot_remaining -= 1;
        state.entries.counter_transaction = true;
        return Some((
            Question::Riot,
            Pending::YesNo {
                player: entry.controller,
                prompt: YesNoPrompt::Riot {
                    permanent: entry.object,
                },
                source: None,
            },
        ));
    }
    if entry.kw_choices.unleash_remaining > 0 {
        entry.kw_choices.unleash_remaining -= 1;
        state.entries.counter_transaction = true;
        return Some((
            Question::Unleash,
            Pending::YesNo {
                player: entry.controller,
                prompt: YesNoPrompt::Unleash {
                    permanent: entry.object,
                },
                source: None,
            },
        ));
    }
    None
}

/// Inspects `opponent`'s library for the creature type(s) appearing on the most
/// creature cards (*Favored Enemy*). Notes the unique winner immediately, or
/// returns a tie-breaker question for the permanent's controller.
fn note_most_prevalent_creature_type(
    state: &GameState,
    entry: &mut Attempt,
    opponent: PlayerId,
) -> Option<(Question, Pending)> {
    let mut counts = std::collections::BTreeMap::<baylee_core::ids::SubtypeId, u32>::new();
    for &card_id in state.zones.list(ZoneLocation::Library(opponent)) {
        let Some(obj) = state.object(card_id) else {
            continue;
        };
        let chars = obj.characteristics();
        if !chars.types.contains(baylee_core::types::TypeSet::CREATURE) {
            continue;
        }
        for sub in chars
            .subtypes
            .intersection(baylee_core::types::SubtypeSet::ALL_CREATURE)
            .iter()
        {
            *counts.entry(sub).or_default() += 1;
        }
    }
    let max_count = counts.values().copied().max().unwrap_or(0);
    if max_count == 0 {
        return None;
    }
    let winners: Vec<baylee_core::ids::SubtypeId> = counts
        .into_iter()
        .filter_map(|(sub, n)| (n == max_count).then_some(sub))
        .collect();
    match winners.as_slice() {
        [] => None,
        [only] => {
            entry.chosen_subtype = Some(*only);
            None
        }
        _ => Some((
            Question::EntrySubtypeTie {
                options: winners.clone(),
            },
            Pending::ChooseSubtype {
                player: entry.controller,
                options: winners,
            },
        )),
    }
}
