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
        tokens::create_tokens_with_text(
            state,
            res.controller,
            token,
            None,
            u32::from(count),
            res.text,
        );
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
    res: &Resolution,
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
        crate::replacement::put_counters_by(
            state,
            res.controller,
            source.object,
            baylee_cards_dsl::CounterKind::P1P1,
            count,
        );
    } else {
        tokens::create_tokens_with_text(
            state,
            res.controller,
            token,
            None,
            u32::from(count),
            res.text,
        );
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
