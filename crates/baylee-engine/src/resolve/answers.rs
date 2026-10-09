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
        let paid = crate::casting::pay_mana(state, player, &cost);
        debug_assert!(paid, "the numeric payment is bounded by spendable mana");
        if let Some(pending) =
            life::damage_with_payment(state, res, player, amount, if paid { number } else { 0 })
        {
            return Flow::Wait(pending);
        }
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
    if let Some(obj) = state.object(target).filter(|o| o.version == version) {
        let room = maximum.saturating_sub(obj.counters.get(kind));
        let count = u16::try_from(number)
            .unwrap_or(u16::MAX)
            .saturating_mul(crate::replacement::counter_multiplier(state, target))
            .min(room);
        crate::replacement::record_counters(state, target, kind, count);
    }
    res.pc += 1;
    run(state, res)
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
    if let AwaitingOp::ProtectionColor { target, duration } = awaiting {
        grant_protection_from(state, res, target, color, duration);
        res.pc += 1;
        return run(state, res);
    }
    let AwaitingOp::ManaChoice {
        recipient,
        colors,
        remaining,
        per_pick,
        restriction,
    } = awaiting
    else {
        panic!("resume_with_color on a question that is not about a color");
    };
    debug_assert!(colors.contains(&color));
    mana::add_to(state, res, recipient, color, per_pick, restriction);
    if remaining > 1 {
        res.awaiting = Some(AwaitingOp::ManaChoice {
            recipient,
            colors: colors.clone(),
            remaining: remaining - 1,
            per_pick,
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
    for &(id, to) in moves {
        // Already answered: this is the second visit, the one that runs.
        if state.commander_redirect.iter().any(|(o, _)| *o == id) {
            continue;
        }
        if let Some(owner) = state.commander_owner(id, to) {
            remaining.push((owner, id, matches!(to, ZoneLocation::Library(_))));
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

pub(super) fn resume_yes_no_inner(
    state: &mut GameState,
    res: &mut Resolution,
    answer: bool,
) -> Flow {
    if let Some(flow) = resume_cast_question(state, res, answer) {
        return flow;
    }
    if let Some(AwaitingOp::PlayerMayPayLife {
        player,
        amount,
        effect,
    }) = res.awaiting
    {
        res.awaiting = None;
        if answer && state.can_pay_life(player, i32::from(amount)) {
            state.change_life(player, -i32::from(amount), Cause::Cost);
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
        let Some(AwaitingOp::SkipDraw {
            source,
            mut declined,
        }) = res.awaiting.take()
        else {
            unreachable!("just matched")
        };
        if let Some((player, next)) = state.draw_offer_answered(source, answer, &mut declined) {
            res.awaiting = Some(AwaitingOp::SkipDraw {
                source: next,
                declined,
            });
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
            Some(pending) => Flow::Wait(pending),
            None => run(state, res),
        };
    }
    let since = state.journal.last_seq();
    let awaiting = res.awaiting.take().expect("resume without awaiting op");
    let library = ZoneLocation::Library(res.controller);
    match (awaiting, piles) {
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
        // CR 701.25a: any number into the graveyard, the rest on top in any
        // order. The owner's graveyard and not the controller's: a card only
        // ever goes to its owner's graveyard, and a surveil that met a stolen
        // card would still send it home.
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
        effects,
        on_payment,
    } = res.awaiting.take().expect("resume without awaiting op")
    else {
        panic!("resume_tax_choice on non-tax choice");
    };
    // `pay` mutates the pool — never hide the call behind `debug_assert!`,
    // which is not evaluated in release. A failed payment takes the
    // not-paid fallback, exactly as if the player had declined.
    let actually_paid = paid && crate::casting::pay_mana(state, player, &cost);
    debug_assert!(!paid || actually_paid, "tax was offered as payable");
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
    res.pc += 1;
    run(state, res)
}
