//! APNAP library choices with simultaneous placement (CR 101.4a).
#[allow(clippy::wildcard_imports)]
use super::*;

/// Selections remain in their libraries until every participant has decided.
#[derive(Clone, Debug)]
pub struct Selection {
    /// Current ordinary search, including any controlling opponent.
    pub(super) operation: Box<AwaitingOp>,
    players: Vec<PlayerId>,
    next: usize,
    filter: &'static baylee_cards_dsl::Filter,
    find: &'static baylee_cards_dsl::Find,
    found: Vec<(PlayerId, ObjectId, Option<PlayerId>, baylee_cards_dsl::Find)>,
}

impl Selection {
    pub(super) fn fingerprint(&self) -> u64 {
        crate::state::structural_fingerprint(&(
            self.operation.search_fingerprint(),
            &self.players,
            self.next,
            self.filter,
            self.find,
            &self.found,
        ))
    }
}

pub(super) fn begin(
    state: &mut GameState,
    res: &mut Resolution,
    filter: &'static baylee_cards_dsl::Filter,
    find: &'static baylee_cards_dsl::Find,
) -> Option<Pending> {
    let count = state.players.len();
    let active = usize::from(state.turn.active.get());
    let players = (0..count)
        .map(|offset| PlayerId::new(u8::try_from((active + offset) % count).expect("seat fits")))
        .filter(|&player| !state.has_left(player))
        .collect();
    next(
        state,
        res,
        Selection {
            // Replaced before this state can be suspended or observed.
            operation: Box::new(AwaitingOp::ChooseNumber),
            players,
            next: 0,
            filter,
            find,
            found: Vec::new(),
        },
    )
}

pub(super) fn resume(
    state: &mut GameState,
    res: &mut Resolution,
    mut selection: Selection,
    chosen: &[ObjectId],
) -> Option<Pending> {
    let (library, agent) = match &*selection.operation {
        AwaitingOp::SearchLibrary { library, .. } => (*library, None),
        AwaitingOp::SearchTakeover { library, agent, .. } => (*library, Some(*agent)),
        _ => unreachable!("group holds one ordinary search"),
    };
    if let Some(&card) = chosen.first() {
        let find = search::resolve_find(state, res, library, *selection.find, card);
        selection.found.push((library, card, agent, find));
    }
    next(state, res, selection)
}

fn next(state: &mut GameState, res: &mut Resolution, mut selection: Selection) -> Option<Pending> {
    while let Some(&player) = selection.players.get(selection.next) {
        selection.next += 1;
        let pending = search::begin_search(
            state,
            res,
            search::Search {
                library: player,
                searcher: player,
                filter: selection.filter,
                finds: core::slice::from_ref(selection.find),
                bound: None,
                count: None,
                distinct_names: false,
                split: None,
                optional: false,
            },
        );
        if let Some(pending) = pending {
            selection.operation = Box::new(res.awaiting.take().expect("search continuation"));
            res.awaiting = Some(AwaitingOp::SearchGroup(Box::new(selection)));
            return Some(pending);
        }
    }
    for (player, card, agent, find) in selection.found {
        if state.has_left(player)
            || !state
                .object(card)
                .is_some_and(|o| o.zone == crate::zone::Zone::Library && o.owner == player)
        {
            continue;
        }
        if let Some(agent) = agent.filter(|&agent| !state.has_left(agent)) {
            let _ = state.move_object(
                card,
                ZoneLocation::Exile(agent),
                ZonePosition::Top,
                Cause::Effect,
            );
            if let Some(object) = state.object_mut(card) {
                object
                    .riders
                    .push(crate::object::Rider::PlayableFromExileFor(agent));
            }
            continue;
        }
        if search::reveals(selection.filter, core::slice::from_ref(selection.find)) {
            state.journal.record(GameEvent::Revealed {
                player,
                cards: vec![card],
            });
        }
        state.entries.set_putter(player);
        if find.tapped && find.dest == SearchDest::Battlefield {
            state.set_tapped(card, true);
        }
        search::put_found(state, player, card, find.dest);
        if find.dest == SearchDest::Battlefield {
            if let Some((kind, count)) = find.counter {
                crate::entry::put_counters(state, res.controller, card, kind, count);
            }
            if let Some(reference) = crate::entry::battlefield_reference(state, card) {
                res.subject.found.push(reference);
            }
        }
    }
    None
}
