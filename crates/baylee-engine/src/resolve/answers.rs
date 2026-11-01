//! Resuming a resolution with a player's answer: a number, a colour, yes or no, an arrangement, a tax, targets.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary
use super::*;

/// Resumes a bounded counter placement's numeric choice.
pub fn resume_with_number(state: &mut GameState, res: &mut Resolution, number: u32) -> Flow {
    subjects::begin_resume(state, res);
    let flow = resume_with_number_inner(state, res, number);
    subjects::flush(state, res);
    flow
}

pub(super) fn resume_with_number_inner(
    state: &mut GameState,
    res: &mut Resolution,
    number: u32,
) -> Flow {
    if matches!(res.awaiting, Some(AwaitingOp::ChoosePhaseToSkipThisTurn)) {
        match number {
            0 => state.per_turn.skip_draw_step = true,
            1 => state.per_turn.skip_main_phases = true,
            _ => state.per_turn.skip_combat_phases = true,
        }
        res.awaiting = None;
        res.pc += 1;
        return run(state, res);
    }
    if matches!(res.awaiting, Some(AwaitingOp::ChooseNumber)) {
        if let Some(source) =
            subjects::source(state, res).filter(|source| subjects::on_battlefield(state, *source))
        {
            let acquisition = super::subjects::acquisition(state, res);
            crate::entry::set_chosen_number(
                state,
                source.object,
                u8::try_from(number).expect("bounded choice"),
                acquisition,
            );
        }
        res.awaiting = None;
        advance_program(res);
        return run(state, res);
    }
    if matches!(res.awaiting, Some(AwaitingOp::TapEnergy(_))) {
        let Some(AwaitingOp::TapEnergy(selection)) = res.awaiting.take() else {
            unreachable!("energy payment continuation")
        };
        if let Some(pending) = super::tap_energy::pay(state, res, *selection, number) {
            return Flow::Wait(pending);
        }
        res.pc += 1;
        return run(state, res);
    }
    if let Some(AwaitingOp::TextReplacement { target, kind }) = res.awaiting {
        if let Some(replacement) = crate::text_changes::TextReplacement::from_choice(kind, number)
            && state
                .object(target.object)
                .is_some_and(|object| object.version == target.version)
        {
            state.text_changes.replace(target, replacement);
            state.invalidate_projections();
        }
        res.awaiting = None;
        res.pc += 1;
        return run(state, res);
    }

    if let Some(AwaitingOp::DamagePayment { player, amount }) = res.awaiting {
        res.awaiting = None;
        let cost = baylee_core::mana::ManaCost::from_symbol_generic(number);
        let what = super::payment_for(state, res);
        let paid = crate::casting::pay_mana_for(state, player, what, &cost).is_some();
        debug_assert!(paid, "the numeric payment is bounded by spendable mana");
        if let Some(pending) =
            life::damage_with_payment(state, res, player, amount, if paid { number } else { 0 })
        {
            return Flow::Wait(pending);
        }
        res.pc += 1;
        return run(state, res);
    }
    if let Some(AwaitingOp::RemoveCountersSelfCreateTokens {
        source,
        kind,
        token,
    }) = res.awaiting
    {
        res.awaiting = None;
        remove_counters_self_create_tokens(state, res, source, kind, token, number);
        res.pc += 1;
        return run(state, res);
    }
    let Some(AwaitingOp::Counters {
        target,
        version,
        kind,
        maximum,
    }) = res.awaiting.take()
    else {
        unreachable!("numeric resolution choice has a counter continuation");
    };
    if state
        .object(target)
        .is_some_and(|object| object.version == version)
    {
        crate::replacement::put_counters_bounded_by(
            state,
            res.controller,
            target,
            kind,
            u16::try_from(number).unwrap_or(u16::MAX),
            maximum,
        );
    }
    res.pc += 1;
    run(state, res)
}

fn remove_counters_self_create_tokens(
    state: &mut GameState,
    res: &mut Resolution,
    source: baylee_core::ids::DamageSourceRef,
    kind: baylee_cards_dsl::CounterKind,
    token: &'static baylee_cards_dsl::TokenDef,
    number: u32,
) {
    let want = u16::try_from(number).unwrap_or(u16::MAX);
    if want > 0 && subjects::on_battlefield(state, source) {
        let removed = crate::replacement::remove_counters_by(
            state,
            source.object,
            kind,
            want,
            Some(res.controller),
        );
        if removed > 0 {
            let created = tokens::create_tokens_with_text(
                state,
                res.controller,
                token,
                None,
                u32::from(removed),
                res.text,
            );
            for id in created {
                if let Some(tok) = state.object_mut(id) {
                    tok.riders.push(crate::object::Rider::CreatedBy(source));
                }
            }
        }
    }
}

/// Resumes a color choice suspended on [`AwaitingOp::ManaChoice`].
///
/// # Panics
/// When the suspended operation is not a mana choice.
#[must_use]
pub fn resume_with_color(state: &mut GameState, res: &mut Resolution, color: ManaColor) -> Flow {
    subjects::begin_resume(state, res);
    let flow = resume_with_color_inner(state, res, color);
    subjects::flush(state, res);
    flow
}

pub(super) fn resume_with_color_inner(
    state: &mut GameState,
    res: &mut Resolution,
    color: ManaColor,
) -> Flow {
    let awaiting = res.awaiting.take().expect("resume without awaiting op");
    if let AwaitingOp::SourceColor { source } = awaiting {
        super::source_color::remember(state, source, color);
        res.pc += 1;
        return run(state, res);
    }
    if let AwaitingOp::ManaUnlessPaidColor { amount, cost } = awaiting {
        if let Some(pending) = mana::offer_prevention(state, res, color, amount, cost, 0) {
            return Flow::Wait(pending);
        }
        res.pc += 1;
        return run(state, res);
    }
    if let AwaitingOp::ProtectionColor { target, duration } = awaiting {
        grant_protection_from(state, res, target, color, duration);
        res.pc += 1;
        return run(state, res);
    }
    let AwaitingOp::ManaChoice {
        recipient,
        mut colors,
        distinct,
        remaining,
        per_pick,
        by_color,
        restriction,
    } = awaiting
    else {
        panic!("resume_with_color on a question that is not about a color");
    };
    debug_assert!(colors.contains(&color));
    mana::add_to(
        state,
        res,
        recipient,
        color,
        by_color.map_or(per_pick, |amounts| amounts[color.index()]),
        restriction,
    );
    if distinct {
        colors.retain(|available| *available != color);
    }
    if remaining > 1 && !colors.is_empty() {
        res.awaiting = Some(AwaitingOp::ManaChoice {
            recipient,
            colors: colors.clone(),
            distinct,
            remaining: remaining - 1,
            per_pick,
            by_color,
            restriction,
        });
        return Flow::Wait(Pending::ChooseColor {
            player: recipient,
            options: colors,
        });
    }
    res.pc += 1;
    run(state, res)
}

/// "Protection from [color]", one filter per color, so a grant made from a
/// choice names a filter that lives as long as the effect does.
pub(super) static PROTECTION_COLORS: [baylee_cards_dsl::Filter; 5] = [
    baylee_cards_dsl::Filter::HasColor(ColorSet::of(baylee_core::color::Color::White)),
    baylee_cards_dsl::Filter::HasColor(ColorSet::of(baylee_core::color::Color::Blue)),
    baylee_cards_dsl::Filter::HasColor(ColorSet::of(baylee_core::color::Color::Black)),
    baylee_cards_dsl::Filter::HasColor(ColorSet::of(baylee_core::color::Color::Red)),
    baylee_cards_dsl::Filter::HasColor(ColorSet::of(baylee_core::color::Color::Green)),
];

/// Registers "protection from `color`" on `target` for `duration`, if it is
/// still on the battlefield — the answer to [`AwaitingOp::ProtectionColor`].
pub(super) fn grant_protection_from(
    state: &mut GameState,
    res: &Resolution,
    target: ObjectId,
    color: ManaColor,
    duration: baylee_cards_dsl::Duration,
) {
    let Some(filter) = PROTECTION_COLORS.get(color as usize) else {
        return;
    };
    if !state
        .object(target)
        .is_some_and(|o| o.zone == crate::zone::Zone::Battlefield)
    {
        return;
    }
    let timestamp = state.next_timestamp();
    let filter_on = crate::effects::EffectFilter::object(state, target);
    state.effects.register(crate::effects::ContinuousEffect {
        id: baylee_core::ids::EffectId::new(0),
        source: Some(res.source),
        controller: res.controller,
        origin: crate::effects::EffectOrigin::Resolution,
        layer: baylee_cards_dsl::Layer::Ability,
        timestamp,
        duration,
        filter: filter_on,
        modifier: baylee_cards_dsl::Modifier::ProtectionFrom(filter),
    });
}

/// Puts CR 903.9b's question before an operation that is about to move
/// cards into hands or libraries, and suspends if anyone has to answer it.
///
/// `moves` is what the operation is about to hand to
/// [`GameState::move_object`], destination included — the same pairs, not a
/// summary of them, so what the player is asked about cannot drift from
/// what actually moves.
///
/// **Call this before the operation mutates anything.** The last answer
/// re-enters the operation at the same program counter, so an effect that
/// had already flipped a card's kind or dealt its damage would do it twice.
pub(super) fn ask_commander_replace(
    state: &GameState,
    res: &mut Resolution,
    moves: &[(ObjectId, ZoneLocation)],
) -> Option<Pending> {
    let mut remaining: Vec<(PlayerId, ObjectId, bool)> = Vec::new();
    for &(root, to) in moves {
        for id in state.merged_components(root) {
            // Each physical commander chooses independently; the other cards
            // still go to the destination of the merged permanent.
            if state.commander_redirect.iter().any(|(o, _)| *o == id)
                || remaining.iter().any(|(_, other, _)| *other == id)
            {
                continue;
            }
            if let Some(owner) = state.commander_owner(id, to) {
                remaining.push((owner, id, matches!(to, ZoneLocation::Library(_))));
            }
        }
    }
    let (asked, pending) = next_commander_ask(state, &mut remaining)?;
    res.awaiting = Some(AwaitingOp::CommanderReplace { asked, remaining });
    Some(pending)
}

/// Takes the next owner off the list and builds their question.
pub(super) fn next_commander_ask(
    state: &GameState,
    remaining: &mut Vec<(PlayerId, ObjectId, bool)>,
) -> Option<(ObjectId, Pending)> {
    if remaining.is_empty() {
        return None;
    }
    let (player, card, to_library) = remaining.remove(0);
    let pending = Pending::YesNo {
        player,
        prompt: YesNoPrompt::CommanderReplace { card, to_library },
        // A card-scoped handle, so "always send Katara home" is an answer a
        // seat can keep — and its own reserved index, because agreeing to
        // that for a graveyard is not agreeing to it for a bounce.
        source: state.object(card).and_then(|o| o.card).map(|c| {
            baylee_core::ids::AbilityRef::new(
                c.index,
                baylee_core::ids::AbilityRef::COMMANDER_REPLACE,
            )
        }),
    };
    Some((card, pending))
}

/// The card types a "for each card type" asks about, in CR 205.2a's order:
/// the ones a card in a library can have.
pub(super) const CARD_TYPES: [baylee_core::types::TypeSet; 9] = [
    baylee_core::types::TypeSet::ARTIFACT,
    baylee_core::types::TypeSet::BATTLE,
    baylee_core::types::TypeSet::CREATURE,
    baylee_core::types::TypeSet::ENCHANTMENT,
    baylee_core::types::TypeSet::INSTANT,
    baylee_core::types::TypeSet::KINDRED,
    baylee_core::types::TypeSet::LAND,
    baylee_core::types::TypeSet::PLANESWALKER,
    baylee_core::types::TypeSet::SORCERY,
];

/// Asks the next of `RevealTopOnePerType`'s questions, from the card type at
/// `from` on: the first type one of the `revealed` cards still in the
/// library has. When no type is left to ask about, the cards still there go
/// to the bottom of the library in a random order and nothing is asked.
pub(super) fn one_per_type(
    state: &mut GameState,
    res: &mut Resolution,
    revealed: Vec<ObjectId>,
    from: usize,
) -> Option<Pending> {
    let still_there = |state: &GameState, card: ObjectId| {
        state
            .object(card)
            .is_some_and(|o| o.zone == crate::zone::Zone::Library)
    };
    for (i, &card_type) in CARD_TYPES.iter().enumerate().skip(from) {
        let options: Vec<ObjectId> = revealed
            .iter()
            .copied()
            .filter(|&card| {
                still_there(state, card)
                    && state
                        .object(card)
                        .is_some_and(|o| o.characteristics().types.contains(card_type))
            })
            .collect();
        if options.is_empty() {
            continue;
        }
        res.awaiting = Some(AwaitingOp::OnePerType {
            revealed,
            next: i + 1,
        });
        return Some(Pending::ChooseCards {
            player: res.controller,
            options,
            min: 0,
            max: 1,
            prompt: ChoicePrompt::OneOfType { card_type },
            total: None,
        });
    }
    let mut rest: Vec<ObjectId> = revealed
        .into_iter()
        .filter(|&card| still_there(state, card))
        .collect();
    state.rng.shuffle(&mut rest);
    for card in rest {
        let _ = state.move_object(
            card,
            ZoneLocation::Library(res.controller),
            ZonePosition::Bottom,
            Cause::Effect,
        );
    }
    None
}

/// The two yes/no questions that offer a cast (`MayCastTarget`, cascade),
/// resumed; `None` when the question out is another one.
pub(super) fn resume_cast_question(
    state: &mut GameState,
    res: &mut Resolution,
    answer: bool,
) -> Option<Flow> {
    // "You may cast that card": a yes is a cast the engine makes as this
    // resolution ends, after a payment window (CR 608.2g).
    if let Some(AwaitingOp::CastTarget {
        card,
        version,
        then_no_more_spells,
        any_mana,
    }) = res.awaiting
    {
        res.awaiting = None;
        if answer {
            state.delayed.push(crate::state::DelayedTrigger {
                controller: res.controller,
                when: crate::state::DelayedWhen::AsResolutionEnds,
                action: crate::state::DelayedAction::CastPaying {
                    card,
                    version,
                    then_no_more_spells,
                    any_mana,
                },
            });
        }
        res.pc += 1;
        return Some(run(state, res));
    }
    // Cascade: the cards not cast go to the bottom now, and a yes is a cast
    // the engine makes as this resolution ends (CR 702.85a).
    if matches!(res.awaiting, Some(AwaitingOp::CascadeCast { .. })) {
        let Some(AwaitingOp::CascadeCast { hit, mut rest }) = res.awaiting.take() else {
            unreachable!("just matched")
        };
        let you = res.controller;
        let hit_version = state
            .object(hit)
            .filter(|o| o.zone == crate::zone::Zone::Exile)
            .map(|o| o.version);
        match hit_version {
            Some(version) if answer => state.delayed.push(crate::state::DelayedTrigger {
                controller: you,
                when: crate::state::DelayedWhen::AsResolutionEnds,
                action: crate::state::DelayedAction::CastFreeOrBottom { card: hit, version },
            }),
            Some(_) => rest.push(hit),
            None => {}
        }
        bottom_in_random_order(state, you, rest);
        res.pc += 1;
        return Some(run(state, res));
    }
    None
}

/// Resumes a yes/no choice (shockland payment and friends).
///
/// # Panics
/// When the suspended operation is not a yes/no choice.
#[must_use]
pub fn resume_yes_no(state: &mut GameState, res: &mut Resolution, answer: bool) -> Flow {
    subjects::begin_resume(state, res);
    let flow = resume_yes_no_inner(state, res, answer);
    subjects::flush(state, res);
    flow
}

fn resume_may_exile_graveyard(
    state: &mut GameState,
    res: &mut Resolution,
    answer: bool,
) -> Option<Flow> {
    if let Some(AwaitingOp::PlayerMayGetPoison {
        player,
        amount,
        effect,
    }) = res.awaiting
    {
        res.awaiting = None;
        if answer {
            state.put_player_counters_by(
                res.controller,
                player,
                baylee_cards_dsl::CounterKind::Poison,
                amount,
            );
            res.pc += 1;
            return Some(run(state, res));
        }
        return Some(run_fallback(state, res, std::slice::from_ref(effect)));
    }
    if let Some(AwaitingOp::PlayerMayMill {
        player,
        count,
        effect,
    }) = res.awaiting
    {
        res.awaiting = None;
        if answer && state.zones.list(ZoneLocation::Library(player)).len() >= usize::from(count) {
            let _ = state.mill_top(player, usize::from(count));
            res.pc += 1;
            return Some(run(state, res));
        }
        return Some(run_fallback(state, res, std::slice::from_ref(effect)));
    }
    if !matches!(
        res.awaiting,
        Some(AwaitingOp::MayExileFromGraveyardThen { .. })
    ) {
        return None;
    }
    let Some(AwaitingOp::MayExileFromGraveyardThen {
        options,
        count,
        then,
    }) = res.awaiting.take()
    else {
        unreachable!("just matched")
    };
    if !answer {
        res.pc += 1;
        return Some(run(state, res));
    }
    if count == 0 {
        return Some(run_fallback(state, res, then));
    }
    let pending = Pending::ChooseCards {
        player: res.controller,
        options: options.clone(),
        min: count,
        max: count,
        prompt: ChoicePrompt::CostExile,
        total: None,
    };
    res.awaiting = Some(AwaitingOp::MayExileFromGraveyardThen {
        options,
        count,
        then,
    });
    Some(Flow::Wait(pending))
}

#[allow(clippy::too_many_lines)]
pub(super) fn resume_yes_no_inner(
    state: &mut GameState,
    res: &mut Resolution,
    answer: bool,
) -> Flow {
    if let Some(flow) = super::coin_stakes::answer(state, res, answer)
        .or_else(|| super::search::resume_consent(state, res, answer))
        .or_else(|| resume_selected_commander(state, res, answer))
        .or_else(|| graveyard_choice::resume_commander(state, res, answer))
        .or_else(|| resume_cast_question(state, res, answer))
        .or_else(|| super::proliferate::resume_player(state, res, answer))
        .or_else(|| resume_may_exile_graveyard(state, res, answer))
    {
        return flow;
    }
    if let Some(AwaitingOp::PayEnergy { amount, effect }) = res.awaiting {
        res.awaiting = None;
        if answer && state.pay_energy(res.controller, u32::from(amount)) {
            res.pc += 1;
            return run(state, res);
        }
        return run_fallback(state, res, std::slice::from_ref(effect));
    }
    if let Some(AwaitingOp::PlayerMayPayLife {
        player,
        amount,
        effect,
    }) = res.awaiting
    {
        res.awaiting = None;
        if answer
            && let Ok(amount) = i32::try_from(amount)
            && state.can_pay_life(player, amount)
        {
            state.change_life(player, -amount, Cause::Cost);
            res.pc += 1;
            return run(state, res);
        }
        return run_fallback(state, res, std::slice::from_ref(effect));
    }
    if let Some(AwaitingOp::TopOrBottom { card, owner }) = res.awaiting {
        res.awaiting = None;
        if let Some(obj) = state.object_mut(card) {
            obj.kind = ObjectKind::Card;
        }
        let _ = state.move_object(
            card,
            ZoneLocation::Library(owner),
            if answer {
                ZonePosition::Top
            } else {
                ZonePosition::Bottom
            },
            Cause::Effect,
        );
        res.pc += 1;
        return run(state, res);
    }
    // Island Sanctuary's skip for one waiting draw: the answer settles that
    // card, and the next waiting draw may be asked in turn. The
    // instruction that drew has run, so nothing advances here.
    if matches!(res.awaiting, Some(AwaitingOp::SkipDraw { .. })) {
        let Some(AwaitingOp::SkipDraw { source }) = res.awaiting.take() else {
            unreachable!("just matched")
        };
        if let Some((player, next)) = state.draw_offer_answered(source, answer) {
            res.awaiting = Some(AwaitingOp::SkipDraw { source: next });
            return Flow::Wait(state.draw_offer_question(player, next));
        }
        return run(state, res);
    }
    // CR 903.9b: record what this owner said, then either ask the next one
    // or run the operation that has been waiting for all of them.
    if matches!(res.awaiting, Some(AwaitingOp::CommanderReplace { .. })) {
        let Some(AwaitingOp::CommanderReplace {
            asked,
            mut remaining,
        }) = res.awaiting.take()
        else {
            unreachable!("just matched")
        };
        state.commander_redirect.push((asked, answer));
        if let Some((asked, pending)) = next_commander_ask(state, &mut remaining) {
            res.awaiting = Some(AwaitingOp::CommanderReplace { asked, remaining });
            return Flow::Wait(pending);
        }
        // `take` above already cleared `awaiting`, and no `pc += 1` here:
        // the operation this was asked for has not run yet.
        return run(state, res);
    }
    if matches!(res.awaiting, Some(AwaitingOp::SacrificePayment { .. })) {
        return super::sacrifices::answer_payment(state, res, answer);
    }
    if matches!(res.awaiting, Some(AwaitingOp::Cleansing { .. })) {
        let Some(AwaitingOp::Cleansing {
            asked,
            permanents,
            mut current_idx,
            mut remaining_players,
            to_destroy,
            life,
        }) = res.awaiting.take()
        else {
            unreachable!("just matched")
        };
        if answer && state.can_pay_life(asked, i32::from(life)) {
            state.change_life(asked, -i32::from(life), Cause::Effect);
            current_idx += 1;
            remaining_players =
                eval::players(PlayerRel::EachPlayer, state, state.turn.active).unwrap_or_default();
        }
        if let Some(pending) = super::immediate::cleansing_advance(
            state,
            res,
            permanents,
            current_idx,
            remaining_players,
            to_destroy,
            life,
        ) {
            return Flow::Wait(pending);
        }
        res.pc += 1;
        return run(state, res);
    }
    if matches!(
        res.awaiting,
        Some(AwaitingOp::AnyPlayerSacrificeOrDamage { .. })
    ) {
        let Some(AwaitingOp::AnyPlayerSacrificeOrDamage {
            asked,
            remaining,
            step,
            filter,
            count,
            damage,
            effects,
        }) = res.awaiting.take()
        else {
            unreachable!("just matched")
        };
        if step == 0 {
            if answer {
                let options = chosen::options(state, asked, filter, asked, res.source);
                if options.len() >= usize::from(count) {
                    res.awaiting = Some(AwaitingOp::AnyPlayerSacrificeOrDamage {
                        asked,
                        remaining,
                        step: 1,
                        filter,
                        count,
                        damage,
                        effects,
                    });
                    return Flow::Wait(Pending::ChooseCards {
                        player: asked,
                        options,
                        min: count,
                        max: count,
                        prompt: ChoicePrompt::Generic,
                        total: None,
                    });
                }
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
            return Flow::Wait(Pending::YesNo {
                player: asked,
                prompt: YesNoPrompt::Generic,
                source: resolving_ability(state, res),
            });
        }
        if answer {
            let next = res.pc + 1;
            res.effects.splice(next..next, effects.iter().copied());
            if let Some(pending) = life::deal_to_players(state, res, &[(asked, u32::from(damage))])
            {
                return Flow::Wait(pending);
            }
            res.pc += 1;
            return run(state, res);
        }
        if let Some(pending) = super::immediate::any_player_sac_or_damage_advance(
            state, res, remaining, filter, count, damage, effects,
        ) {
            return Flow::Wait(pending);
        }
        res.pc += 1;
        return run(state, res);
    }
    if let Some(AwaitingOp::RevealTopPlayerMayPut { player, card }) = res.awaiting {
        res.awaiting = None;
        let since = state.journal.last_seq();
        if answer
            && state
                .zones
                .list(ZoneLocation::Library(player))
                .contains(&card)
        {
            super::search::put_found(state, player, card, SearchDest::Battlefield);
        }
        return finish_choice(state, res, since);
    }
    if let Some(AwaitingOp::GuessNamedCardInHand { actual }) = res.awaiting {
        res.awaiting = None;
        res.subject.guess = Some(super::subjects::GuessOutcome {
            actual,
            claimed: answer,
            revealed: false,
        });
        res.pc += 1;
        return run(state, res);
    }
    if matches!(res.awaiting, Some(AwaitingOp::CoinFlipIgnore { .. })) {
        return super::coin::resume(state, res, answer);
    }
    if let Some(AwaitingOp::FlipCoinsUntilLossOrStopCumulative { wins }) = res.awaiting {
        const WIN_1: &[Effect] = &[Effect::DealDamage {
            target: TargetSpec::Object(&baylee_cards_dsl::Filter::CREATURE),
            amount: Amount::Fixed(3),
        }];
        const WIN_2: &[Effect] = &[
            Effect::DealDamage {
                target: TargetSpec::Object(&baylee_cards_dsl::Filter::CREATURE),
                amount: Amount::Fixed(3),
            },
            Effect::DealDamage {
                target: TargetSpec::Player(PlayerRel::EachOpponent),
                amount: Amount::Fixed(6),
            },
        ];
        const WIN_3: &[Effect] = &[
            Effect::DealDamage {
                target: TargetSpec::Object(&baylee_cards_dsl::Filter::CREATURE),
                amount: Amount::Fixed(3),
            },
            Effect::DealDamage {
                target: TargetSpec::Player(PlayerRel::EachOpponent),
                amount: Amount::Fixed(6),
            },
            Effect::DrawCards {
                amount: Amount::Fixed(9),
            },
            Effect::UntapAll {
                filter: &baylee_cards_dsl::Filter::YOUR_LAND,
            },
        ];
        res.awaiting = None;
        let you = res.controller;
        if answer {
            match super::coin::flip(
                state,
                res,
                you,
                super::CoinContinuation::UntilLossOrStop {
                    wins: wins.saturating_add(1),
                },
            ) {
                super::coin::Flip::Outcome(won) => {
                    if !won {
                        res.pc += 1;
                        return run(state, res);
                    }
                    res.awaiting = Some(AwaitingOp::FlipCoinsUntilLossOrStopCumulative {
                        wins: wins.saturating_add(1),
                    });
                    return Flow::Wait(Pending::YesNo {
                        player: you,
                        prompt: YesNoPrompt::MayDo,
                        source: resolving_ability(state, res),
                    });
                }
                super::coin::Flip::Wait(pending) => return Flow::Wait(*pending),
            }
        }
        let effects = match wins {
            0 => &[][..],
            1 => WIN_1,
            2 => WIN_2,
            _ => WIN_3,
        };
        return run_fallback(state, res, effects);
    }
    let AwaitingOp::PayLifeOrTapSelf { amount } =
        res.awaiting.take().expect("resume without awaiting op")
    else {
        panic!("resume_yes_no on non-yes/no choice");
    };
    if answer {
        state.change_life(res.controller, -i32::from(amount), Cause::Effect);
    } else {
        state.set_tapped(res.source, true);
    }
    res.pc += 1;
    run(state, res)
}

/// Resumes an optional clause ([`Effect::MayDo`]): `yes` means the
/// controller takes it.
///
/// A no is not a failure and runs no fallback — the clause simply does not
/// happen and the rest of the ability carries on, which is what makes this
/// different from [`resume_tax_choice`], where declining *is* an outcome
/// the card prints.
///
/// # Panics
/// When the suspended operation is not an optional clause.
#[must_use]
pub fn resume_may_do(state: &mut GameState, res: &mut Resolution, yes: bool) -> Flow {
    subjects::begin_resume(state, res);
    let flow = resume_may_do_inner(state, res, yes);
    subjects::flush(state, res);
    flow
}

pub(super) fn resume_may_do_inner(state: &mut GameState, res: &mut Resolution, yes: bool) -> Flow {
    let AwaitingOp::MayDo {
        effects,
        once_each_turn,
    } = res.awaiting.take().expect("resume without awaiting op")
    else {
        panic!("resume_may_do on a choice that is not an optional clause");
    };
    // The yes uses the turn's one go, before the body runs: the body may
    // suspend, and the go is spent by choosing to do it.
    if yes && let Some(key) = once_each_turn {
        state.ability_fires.insert(key, 1);
    }
    if yes && let Some(pending) = run_nested(state, res, effects) {
        return Flow::Wait(pending);
    }
    res.pc += 1;
    run(state, res)
}

/// A scry's or a surveil's question: the looked-at cards, a top pile that
/// may take any of them in an order, and `away` — the bottom or a graveyard
/// — that may take the rest. Top first, so the default answer, which fills
/// the first pile with room, keeps every card where it was.
pub(super) fn look_question(
    player: PlayerId,
    looked: Vec<ObjectId>,
    away: ArrangePlace,
) -> Pending {
    let n = u32::try_from(looked.len()).unwrap_or(u32::MAX);
    Pending::Arrange {
        player,
        cards: looked,
        piles: vec![
            ArrangePile::up_to(ArrangePlace::LibraryTop, n),
            ArrangePile::up_to(away, n),
        ],
        prompt: match away {
            ArrangePlace::Graveyard => ArrangePrompt::Surveil,
            ArrangePlace::LibraryTop | ArrangePlace::LibraryBottom => ArrangePrompt::Scry,
        },
    }
}

/// Resumes a [`Pending::Arrange`] with the cards the player put in each
/// pile, library piles listed top to bottom.
///
/// # Panics
/// When the suspended operation is not an arrangement.
#[must_use]
pub fn resume_arranged(
    state: &mut GameState,
    res: &mut Resolution,
    piles: &[Vec<ObjectId>],
) -> Flow {
    subjects::begin_resume(state, res);
    let flow = resume_arranged_inner(state, res, piles);
    subjects::flush(state, res);
    flow
}

#[allow(
    clippy::too_many_lines,
    reason = "dispatch each suspended arrangement without hiding variants"
)]
pub(super) fn resume_arranged_inner(
    state: &mut GameState,
    res: &mut Resolution,
    piles: &[Vec<ObjectId>],
) -> Flow {
    if let Some(AwaitingOp::GraveyardOrder { next }) = res
        .awaiting
        .take_if(|op| matches!(op, AwaitingOp::GraveyardOrder { .. }))
    {
        crate::graveyard_order::answer(state, &piles[0]);
        let pending = next.map(|next| {
            let (awaiting, pending) = *next;
            res.awaiting = Some(awaiting);
            pending
        });
        return match order_before_continuing(state, res, pending) {
            Some(flow) => flow,
            None => run(state, res),
        };
    }
    let since = state.journal.last_seq();
    let awaiting = res.awaiting.take().expect("resume without awaiting op");
    // Library of Leng's arrangement: answered before the discard is made,
    // which the instruction then makes itself.
    if let AwaitingOp::DiscardDestination { discards, then } = awaiting {
        return resume_discard_destination(state, res, &discards, then, piles);
    }
    let library = ZoneLocation::Library(res.controller);
    match (awaiting, piles) {
        (AwaitingOp::GraveyardToTopOrder { player }, [top]) => {
            if let Some(question) = graveyard_top::place(state, res, player, top.clone()) {
                return next_choice(state, res, since, question);
            }
        }
        (AwaitingOp::HandToTop { player }, [top]) => {
            if let Some(pending) = hand_transfer::place(state, res, player, top) {
                return next_choice(state, res, since, pending);
            }
        }
        (AwaitingOp::ReorderTopLibrary { player }, [top]) => {
            // The first card listed is the new top card, so the list goes on
            // from its bottom end: each card put on top covers the one
            // listed after it. The library is the one looked at, which for
            // Natural Selection is not the controller's.
            let library = ZoneLocation::Library(player);
            for &card in top.iter().rev() {
                let _ = state.move_object(card, library, ZonePosition::Top, Cause::Effect);
            }
        }
        (AwaitingOp::DigBottom, [bottom]) => {
            // The last card listed is the bottom card, so the list goes in
            // from its top end: each card put on the bottom goes under the
            // one listed before it.
            for &card in bottom {
                let _ = state.move_object(card, library, ZonePosition::Bottom, Cause::Effect);
            }
        }
        // CR 701.22a: any number on the bottom in any order, the rest on top
        // in any order. The library is the one the cards were looked at in,
        // which for Jace's "look at the top card of target player's library"
        // is not the controller's — see the variant's own doc.
        (AwaitingOp::Scry { player }, [top, bottom]) => {
            let library = ZoneLocation::Library(player);
            for &card in bottom {
                let _ = state.move_object(card, library, ZonePosition::Bottom, Cause::Effect);
            }
            for &card in top.iter().rev() {
                let _ = state.move_object(card, library, ZonePosition::Top, Cause::Effect);
            }
        }
        (
            AwaitingOp::ExploreScry {
                player,
                target,
                remaining,
            },
            [top, bottom],
        ) => {
            if let Some(question) =
                super::explore::resume_scry(state, res, player, target, remaining, top, bottom)
            {
                return next_choice(state, res, since, question);
            }
        }
        // CR 701.25a: any number into the graveyard, the rest on top in any
        // order. The owner's graveyard and not the controller's: a card only
        // ever goes to its owner's graveyard, and a surveil that met a stolen
        // card would still send it home.
        (AwaitingOp::Explore { player }, [top, graveyard]) => {
            for &card in graveyard {
                let owner = state.object(card).map_or(player, |object| object.owner);
                let _ = state.move_object(
                    card,
                    ZoneLocation::Graveyard(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            for &card in top.iter().rev() {
                let _ = state.move_object(
                    card,
                    ZoneLocation::Library(player),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
        }
        (AwaitingOp::Surveil, [top, graveyard]) => {
            for &card in graveyard {
                let owner = state.object(card).map_or(res.controller, |o| o.owner);
                let _ = state.move_object(
                    card,
                    ZoneLocation::Graveyard(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            for &card in top.iter().rev() {
                let _ = state.move_object(card, library, ZonePosition::Top, Cause::Effect);
            }
        }
        (other, _) => panic!("resume_arranged on {other:?} with {} piles", piles.len()),
    }
    finish_choice(state, res, since)
}

/// Resumes a tax choice (Rhystic Study & co.) or a price (Crystal Rod):
/// `paid` means the player chose to pay the mana.
///
/// # Panics
/// When the suspended operation is not a tax choice.
#[must_use]
pub fn resume_tax_choice(state: &mut GameState, res: &mut Resolution, paid: bool) -> Flow {
    subjects::begin_resume(state, res);
    let flow = resume_tax_choice_inner(state, res, paid);
    subjects::flush(state, res);
    flow
}

pub(super) fn resume_tax_choice_inner(
    state: &mut GameState,
    res: &mut Resolution,
    paid: bool,
) -> Flow {
    let AwaitingOp::PlayerMayPay {
        player,
        cost,
        outcome,
    } = res.awaiting.take().expect("resume without awaiting op")
    else {
        panic!("resume_tax_choice on non-tax choice");
    };
    // `pay` mutates the pool — never hide the call behind `debug_assert!`,
    // which is not evaluated in release. A failed payment takes the
    // not-paid fallback, exactly as if the player had declined.
    let what = super::payment_for(state, res);
    let life = match &outcome {
        ManaPaymentOutcome::ManaAndLife { life, .. } => *life,
        _ => 0,
    };
    let actually_paid = paid
        && super::mana_life_payment::life_payable(state, player, life)
        && crate::casting::pay_mana_for(state, player, what, &cost).is_some();
    if actually_paid && life != 0 {
        state.change_life(
            player,
            -i32::try_from(life).expect("payable life"),
            Cause::Cost,
        );
    }
    debug_assert!(!paid || actually_paid, "tax was offered as payable");
    let (effects, on_payment) = match outcome {
        ManaPaymentOutcome::AnyPlayer { unpaid, next } => {
            if !actually_paid {
                let since = state.journal.last_seq();
                if let Some(pending) =
                    super::any_player_payment::offer(state, res, cost, unpaid, next)
                {
                    return next_choice(state, res, since, pending);
                }
            }
            res.pc += 1;
            return run(state, res);
        }
        ManaPaymentOutcome::RepeatGraveyardChoice(selection) => {
            let since = state.journal.last_seq();
            if let Some(pending) =
                repeated_graveyard_choice::paid(state, res, *selection, actually_paid)
            {
                return next_choice(state, res, since, pending);
            }
            return finish_choice(state, res, since);
        }
        ManaPaymentOutcome::ManaAndLife { paid, unpaid, .. }
        | ManaPaymentOutcome::Branches { paid, unpaid } => {
            return run_fallback(state, res, if actually_paid { paid } else { unpaid });
        }
        ManaPaymentOutcome::Effects {
            effects,
            on_payment,
        } => (effects, on_payment),
        ManaPaymentOutcome::PreventMana {
            color,
            amount,
            next,
        } => {
            if !actually_paid
                && let Some(pending) = mana::offer_prevention(state, res, color, amount, cost, next)
            {
                return Flow::Wait(pending);
            }
            res.pc += 1;
            return run(state, res);
        }
        ManaPaymentOutcome::Untap { chosen } => {
            if actually_paid {
                for (id, version) in chosen {
                    if state.object(id).is_some_and(|o| {
                        o.version == version && o.zone == crate::zone::Zone::Battlefield
                    }) {
                        state.set_tapped(id, false);
                        state
                            .journal
                            .record(crate::event::GameEvent::ObjectUntapped {
                                object: id,
                                cause: crate::event::Cause::Effect,
                            });
                    }
                }
            }
            res.pc += 1;
            return run(state, res);
        }
        ManaPaymentOutcome::Upgrade { card, player } => {
            let since = state.journal.last_seq();
            upgrade::place(state, res, card, player, actually_paid);
            return finish_choice(state, res, since);
        }
    };
    // A tax runs its effect on a refusal and a price on a payment; the
    // other answer is the ability doing nothing more.
    if actually_paid != on_payment {
        res.pc += 1;
        return run(state, res);
    }
    run_fallback(state, res, effects)
}

/// Runs the branch an "unless" effect takes when the player does not pay.
///
/// Shared by the two shapes of that effect rather than written twice: a
/// fallback that suspended on a choice has to splice its own remaining
/// program into the resolution that called it, and a second copy of that
/// splice would be a second place for the program counter to be wrong.
pub(super) fn run_fallback(
    state: &mut GameState,
    res: &mut Resolution,
    effects: &'static [Effect],
) -> Flow {
    if let Some(pending) = run_nested(state, res, effects) {
        return Flow::Wait(pending);
    }
    res.pc += 1;
    run(state, res)
}

/// Continues a resolution suspended on a target question: [`resume`], except
/// that a change of targets (CR 115.7) reads the players too, since a spell
/// aimed at a player may be turned onto another.
///
/// # Panics
/// When called without a suspended operation (engine invariant).
#[must_use]
pub fn resume_targets(
    state: &mut GameState,
    res: &mut Resolution,
    objects: &[ObjectId],
    players: &[PlayerId],
) -> Flow {
    subjects::begin_resume(state, res);
    let flow = resume_targets_inner(state, res, objects, players);
    subjects::flush(state, res);
    flow
}

pub(super) fn resume_targets_inner(
    state: &mut GameState,
    res: &mut Resolution,
    objects: &[ObjectId],
    players: &[PlayerId],
) -> Flow {
    if !matches!(res.awaiting, Some(AwaitingOp::NewTargets(_))) {
        return resume(state, res, objects);
    }
    let Some(AwaitingOp::NewTargets(retarget)) = res.awaiting.take() else {
        unreachable!("matched just above")
    };
    if let Some(pending) = retarget::answer(state, res, *retarget, objects, players) {
        return Flow::Wait(pending);
    }
    advance_program(res);
    run(state, res)
}

/// Resume a non-modal choice made as an instruction resolves.
///
/// # Panics
/// The engine must validate the chosen option against the pending question.
pub fn resume_effect_choice(state: &mut GameState, res: &mut Resolution, index: usize) -> Flow {
    subjects::begin_resume(state, res);
    if matches!(res.awaiting, Some(AwaitingOp::Dungeon { .. })) {
        return dungeon::resume(state, res, index);
    }
    if matches!(res.awaiting, Some(AwaitingOp::MutatePosition)) {
        return mutate::resume(state, res, index);
    }
    if matches!(res.awaiting, Some(AwaitingOp::MoveCounter { .. })) {
        let flow = counter_move::resume(state, res, index);
        subjects::flush(state, res);
        return flow;
    }
    if matches!(res.awaiting, Some(AwaitingOp::Colors { .. })) {
        let flow = colors::resume(state, res, index);
        subjects::flush(state, res);
        return flow;
    }
    let Some(AwaitingOp::ChooseEffect { choices }) = res.awaiting.take() else {
        panic!("effect choice suspended");
    };
    let selected = *choices.get(index).expect("offered choice");
    let flow = run_fallback(state, res, std::slice::from_ref(selected));
    subjects::flush(state, res);
    flow
}

/// Resume an already-made selection after all CR 903.9b answers are known.
fn resume_selected_commander(
    state: &mut GameState,
    res: &mut Resolution,
    answer: bool,
) -> Option<Flow> {
    if !matches!(res.awaiting, Some(AwaitingOp::SelectedCommander { .. })) {
        return None;
    }
    let Some(AwaitingOp::SelectedCommander {
        operation,
        chosen,
        asked,
        mut remaining,
    }) = res.awaiting.take()
    else {
        unreachable!("checked selected replacement continuation")
    };
    state.commander_redirect.push((asked, answer));
    if let Some((asked, pending)) = next_commander_ask(state, &mut remaining) {
        res.awaiting = Some(AwaitingOp::SelectedCommander {
            operation,
            chosen,
            asked,
            remaining,
        });
        return Some(Flow::Wait(pending));
    }
    res.awaiting = Some(*operation);
    Some(super::resume::resume_inner(state, res, &chosen))
}
