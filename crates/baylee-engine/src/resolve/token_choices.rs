//! Choices made during token-producing instructions, before state-based actions.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_cards_dsl::TokenDef;
use baylee_core::ids::DamageSourceRef;

pub(super) fn fabricate(
    state: &mut GameState,
    res: &mut Resolution,
    count: u16,
    token: &'static TokenDef,
) -> Option<Pending> {
    let source = subjects::source(state, res)
        .filter(|id| subjects::is_current(state, *id))
        .filter(|id| {
            state
                .object(id.object)
                .is_some_and(|o| o.zone == crate::zone::Zone::Battlefield)
        });
    let Some(source) = source else {
        let made = tokens::create_tokens_with_text(
            state,
            res.controller,
            token,
            None,
            u32::from(count),
            res.text,
        );
        tokens::remember_created_tokens(state, res, &made);
        return None;
    };
    res.awaiting = Some(AwaitingOp::Fabricate {
        source,
        count,
        token,
    });
    Some(Pending::ChooseCards {
        player: res.controller,
        options: vec![source.object],
        min: 0,
        max: 1,
        prompt: ChoicePrompt::Generic,
        total: None,
    })
}

pub(super) fn finish_fabricate(
    state: &mut GameState,
    res: &mut Resolution,
    chosen: &[ObjectId],
    source: DamageSourceRef,
    count: u16,
    token: &'static TokenDef,
) {
    if chosen.contains(&source.object)
        && subjects::is_current(state, source)
        && state
            .object(source.object)
            .is_some_and(|o| o.zone == crate::zone::Zone::Battlefield)
    {
        res.subject.found.clear();
        crate::replacement::put_counters_by(
            state,
            res.controller,
            source.object,
            baylee_cards_dsl::CounterKind::P1P1,
            count,
        );
    } else {
        let made = tokens::create_tokens_with_text(
            state,
            res.controller,
            token,
            None,
            u32::from(count),
            res.text,
        );
        tokens::remember_created_tokens(state, res, &made);
    }
}

pub(super) fn living_weapon(
    state: &mut GameState,
    res: &mut Resolution,
    token: &'static TokenDef,
) -> Option<Pending> {
    // Creation happens even if the Equipment has already left the battlefield.
    let source = subjects::source(state, res).filter(|id| subjects::is_current(state, *id));
    let created = tokens::create_tokens_with_text(state, res.controller, token, None, 1, res.text);
    tokens::remember_created_tokens(state, res, &created);
    let source = source.filter(|id| {
        state
            .object(id.object)
            .is_some_and(|o| o.zone == crate::zone::Zone::Battlefield)
    })?;
    if created.len() == 1 {
        attachments::attach_to(state, source, created[0], false);
        return None;
    }
    if created.is_empty() {
        return None;
    }
    res.awaiting = Some(AwaitingOp::AttachCreated { source });
    Some(Pending::ChooseCards {
        player: res.controller,
        options: created,
        min: 1,
        max: 1,
        prompt: ChoicePrompt::Generic,
        total: None,
    })
}

/// Starts `Effect::Endure(amount)` (CR 701.63a–b).
pub(super) fn endure(
    state: &mut GameState,
    res: &mut Resolution,
    amount: &baylee_cards_dsl::Amount,
) -> Option<Pending> {
    let n = amount2(amount, state, res.controller, res);
    if n == 0 {
        return None;
    }
    let raw_source = subjects::source(state, res);
    let live_source = raw_source.filter(|id| {
        subjects::is_current(state, *id)
            && state
                .object(id.object)
                .is_some_and(|o| o.zone == crate::zone::Zone::Battlefield)
    });
    let Some(source) = live_source else {
        let controller = raw_source
            .and_then(|s| state.last_known_controller(s.object))
            .unwrap_or(res.controller);
        if !state.has_left(controller) {
            let size = i16::try_from(n).unwrap_or(i16::MAX);
            let made = tokens::create_tokens_with_text(
                state,
                controller,
                &baylee_cards_dsl::SPIRIT_X_X_WHITE,
                Some(size),
                1,
                res.text,
            );
            tokens::remember_created_tokens(state, res, &made);
        }
        return None;
    };
    let controller = state
        .object(source.object)
        .map_or(res.controller, |o| o.controller);
    if state.has_left(controller) {
        return None;
    }
    let count = u16::try_from(n).unwrap_or(u16::MAX);
    res.awaiting = Some(AwaitingOp::Endure {
        source,
        controller,
        count,
    });
    Some(Pending::YesNo {
        player: controller,
        prompt: crate::choice::YesNoPrompt::Endure(n),
        source: resolving_ability(state, res),
    })
}

/// Resumes `AwaitingOp::Endure` with `answer` (`true` = put `N` +1/+1 counters
/// on `source`; `false` = create an `N/N` white Spirit creature token).
pub(super) fn finish_endure(
    state: &mut GameState,
    res: &mut Resolution,
    answer: bool,
    source: DamageSourceRef,
    controller: PlayerId,
    count: u16,
) {
    if state.has_left(controller) || count == 0 {
        return;
    }
    if answer
        && subjects::is_current(state, source)
        && state
            .object(source.object)
            .is_some_and(|o| o.zone == crate::zone::Zone::Battlefield)
    {
        res.subject.found.clear();
        crate::replacement::put_counters_by(
            state,
            controller,
            source.object,
            baylee_cards_dsl::CounterKind::P1P1,
            count,
        );
    } else {
        let size = i16::try_from(count).unwrap_or(i16::MAX);
        let made = tokens::create_tokens_with_text(
            state,
            controller,
            &baylee_cards_dsl::SPIRIT_X_X_WHITE,
            Some(size),
            1,
            res.text,
        );
        tokens::remember_created_tokens(state, res, &made);
    }
}
