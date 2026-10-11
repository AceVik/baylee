//! Pair-constrained land searches, retaining the ordinary search permissions.
#[allow(clippy::wildcard_imports)]
use super::*;

/// A single search with two successive, non-targeted selections.
#[derive(Clone, Debug)]
pub struct Selection {
    /// Normal placement or Opposition Agent's replacement.
    pub(super) operation: Box<AwaitingOp>,
    options: Vec<ObjectId>,
    first: Option<ObjectId>,
}

impl Selection {
    pub(super) fn fingerprint(&self) -> u64 {
        crate::state::structural_fingerprint(&(
            self.operation.search_fingerprint(),
            &self.options,
            self.first,
        ))
    }
}

pub(super) fn begin(state: &mut GameState, res: &mut Resolution) -> Option<Pending> {
    let mut question = search::begin_search(
        state,
        res,
        search::Search {
            library: res.controller,
            searcher: res.controller,
            filter: &baylee_cards_dsl::Filter::BASIC_LAND,
            bound: None,
            finds: &[baylee_cards_dsl::Find::BATTLEFIELD_TAPPED; 2],
            count: None,
            distinct_names: false,
            split: None,
            optional: true,
        },
    )?;
    let Pending::ChooseCards { options, max, .. } = &mut question else {
        unreachable!("library search asks for cards")
    };
    *max = (*max).min(1);
    res.awaiting = Some(AwaitingOp::SharedLandSearch(Box::new(Selection {
        operation: Box::new(res.awaiting.take().expect("search continuation")),
        options: options.clone(),
        first: None,
    })));
    Some(question)
}

pub(super) fn resume(
    state: &mut GameState,
    res: &mut Resolution,
    mut selection: Selection,
    chosen: &[ObjectId],
) -> Flow {
    if selection.first.is_none()
        && let Some(&first) = chosen.first()
    {
        selection.first = Some(first);
        selection.options.retain(|&other| {
            other != first
                && state
                    .object(first)
                    .zip(state.object(other))
                    .is_some_and(|(a, b)| {
                        baylee_core::generated::subtypes::ALL_LAND_TYPES
                            .iter()
                            .any(|subtype| {
                                a.characteristics().subtypes.contains(subtype)
                                    && b.characteristics().subtypes.contains(subtype)
                            })
                    })
        });
        if !selection.options.is_empty() {
            let player = match &*selection.operation {
                AwaitingOp::SearchLibrary { receiver, .. } => *receiver,
                AwaitingOp::SearchTakeover { agent, .. } => *agent,
                _ => unreachable!("ordinary search continuation"),
            };
            let question = Pending::ChooseCards {
                player,
                options: selection.options.clone(),
                min: 0,
                max: 1,
                prompt: ChoicePrompt::SearchLibrary,
                total: None,
            };
            res.awaiting = Some(AwaitingOp::SharedLandSearch(Box::new(selection)));
            return Flow::Wait(question);
        }
        res.awaiting = Some(*selection.operation);
        return super::resume::resume_inner(state, res, &[first]);
    }
    let mut found: Vec<_> = selection.first.into_iter().collect();
    found.extend_from_slice(chosen);
    res.awaiting = Some(*selection.operation);
    super::resume::resume_inner(state, res, &found)
}
