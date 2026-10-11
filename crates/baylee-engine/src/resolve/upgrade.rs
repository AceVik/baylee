//! A library find whose entry depends on paying a mana-value difference.
#[allow(clippy::wildcard_imports)]
use super::*;

pub(super) fn begin(
    state: &mut GameState,
    res: &mut Resolution,
    filter: &'static baylee_cards_dsl::Filter,
    value: Amount,
) -> Option<Pending> {
    let value = amount2(&value, state, res.controller, res);
    let pending = search::begin_search(
        state,
        res,
        search::Search {
            library: res.controller,
            searcher: res.controller,
            filter,
            bound: None,
            finds: &[baylee_cards_dsl::Find::BATTLEFIELD],
            count: None,
            distinct_names: false,
            split: None,
            optional: false,
        },
    )?;
    res.awaiting = Some(AwaitingOp::SearchUpgrade {
        operation: Box::new(res.awaiting.take().expect("search continuation")),
        value,
    });
    Some(pending)
}

pub(super) fn found(
    state: &mut GameState,
    res: &mut Resolution,
    operation: AwaitingOp,
    value: u32,
    chosen: &[ObjectId],
) -> Flow {
    if matches!(operation, AwaitingOp::SearchTakeover { .. }) {
        // Agent's replacement makes the found card unavailable to later
        // instructions of the search spell (its 2020-11-10 Oracle ruling).
        res.awaiting = Some(operation);
        return super::resume::resume_inner(state, res, chosen);
    }
    let since = state.journal.last_seq();
    let Some(&card) = chosen.first() else {
        return finish_choice(state, res, since);
    };
    let Some(object) = state.object(card) else {
        return finish_choice(state, res, since);
    };
    let extra = object.characteristics().mana_value().saturating_sub(value);
    let reference = baylee_core::ids::DamageSourceRef {
        object: card,
        version: object.version,
    };
    if extra == 0 {
        place(state, res, reference, res.controller, true);
        return finish_choice(state, res, since);
    }
    let cost = baylee_core::mana::ManaCost::from_symbol_generic(extra);
    res.awaiting = Some(AwaitingOp::PlayerMayPay {
        player: res.controller,
        cost,
        outcome: ManaPaymentOutcome::Upgrade {
            card: reference,
            player: res.controller,
        },
    });
    Flow::Wait(Pending::YesNo {
        player: res.controller,
        prompt: YesNoPrompt::PayMana { cost },
        source: resolving_ability(state, res),
    })
}

pub(super) fn place(
    state: &mut GameState,
    res: &mut Resolution,
    card: baylee_core::ids::DamageSourceRef,
    player: PlayerId,
    paid: bool,
) {
    if !state.object(card.object).is_some_and(|object| {
        object.version == card.version && object.zone == crate::zone::Zone::Library
    }) {
        return;
    }
    state.entries.set_putter(player);
    search::put_found(
        state,
        player,
        card.object,
        if paid {
            SearchDest::Battlefield
        } else {
            SearchDest::Graveyard
        },
    );
    if paid && let Some(reference) = crate::entry::battlefield_reference(state, card.object) {
        res.subject.found.push(reference);
    }
}
