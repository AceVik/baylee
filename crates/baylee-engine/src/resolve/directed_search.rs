//! Dynamic library searches with separate choosing and receiving players.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_cards_dsl::{Filter, Find};

/// Preserve the ordinary search (including takeovers) while gathering large choices.
#[derive(Clone, Debug)]
pub struct Selection {
    /// Ordinary search continuation; also defines private visibility.
    pub operation: Box<AwaitingOp>,
    chooser: PlayerId,
    receiver: PlayerId,
    options: Vec<ObjectId>,
    count: u32,
    optional: bool,
    selected: Vec<ObjectId>,
}

impl Selection {
    pub(super) fn fingerprint(&self) -> u64 {
        crate::state::structural_fingerprint(&(
            self.operation.search_fingerprint(),
            self.chooser,
            self.receiver,
            &self.options,
            self.count,
            self.optional,
            &self.selected,
        ))
    }
}

#[allow(clippy::too_many_arguments)] // explicit independent operands of a search
pub(super) fn start(
    state: &mut GameState,
    res: &mut Resolution,
    library: PlayerRel,
    receiver: PlayerRel,
    count: Amount,
    filter: &'static Filter,
    find: &'static Find,
    optional: bool,
) -> Option<Pending> {
    let library = players_of(library, state, res.controller, res)
        .first()
        .copied()?;
    let receiver = players_of(receiver, state, res.controller, res)
        .first()
        .copied()?;
    let count = amount2(&count, state, res.controller, res);
    let pending = search::begin_search(
        state,
        res,
        search::Search {
            library,
            searcher: res.controller,
            filter,
            bound: None,
            finds: core::slice::from_ref(find),
            count: None,
            distinct_names: false,
            split: None,
            optional,
        },
    )?;
    let Pending::ChooseCards {
        player: chooser,
        options,
        ..
    } = pending
    else {
        unreachable!("ordinary search asks for cards");
    };
    let mut operation = res.awaiting.take().expect("ordinary search continuation");
    if let AwaitingOp::SearchLibrary {
        receiver: recipient,
        ..
    } = &mut operation
    {
        *recipient = receiver;
    }
    let count = count.min(options.len() as u32);
    Some(offer(
        res,
        Selection {
            operation: Box::new(operation),
            chooser,
            receiver,
            options,
            count,
            optional: optional || !matches!(filter, Filter::Any),
            selected: Vec::new(),
        },
    ))
}

fn offer(res: &mut Resolution, selection: Selection) -> Pending {
    let max = selection
        .count
        .saturating_sub(selection.selected.len() as u32)
        .min(u32::from(u8::MAX)) as u8;
    let pending = Pending::ChooseCards {
        player: selection.chooser,
        options: selection.options.clone(),
        min: if selection.optional { 0 } else { max },
        max,
        prompt: ChoicePrompt::SearchLibrary,
        total: None,
    };
    res.awaiting = Some(AwaitingOp::DirectedSearch(Box::new(selection)));
    pending
}

pub(super) fn resume(
    state: &mut GameState,
    res: &mut Resolution,
    mut selection: Selection,
    chosen: &[ObjectId],
) -> Flow {
    // Losing control of a search restores its original searcher, while the
    // printed recipient remains independent of who makes that choice.
    selection.chooser = match &mut *selection.operation {
        AwaitingOp::SearchTakeover { agent, .. } => *agent,
        AwaitingOp::SearchLibrary { receiver, .. } => {
            *receiver = selection.receiver;
            res.controller
        }
        _ => unreachable!("directed search wraps an ordinary search"),
    };
    let max = selection
        .count
        .saturating_sub(selection.selected.len() as u32)
        .min(u32::from(u8::MAX)) as usize;
    selection.selected.extend_from_slice(chosen);
    if chosen.len() == max
        && selection.selected.len() < selection.count as usize
        && !state.has_left(selection.chooser)
    {
        selection.options.retain(|card| !chosen.contains(card));
        return Flow::Wait(offer(res, selection));
    }
    res.awaiting = Some(*selection.operation);
    super::resume::resume_inner(state, res, &selection.selected)
}
