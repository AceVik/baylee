//! Non-targeted selections for proliferate, with no 255-permanent rules limit.
use super::{
    AwaitingOp, ChoicePrompt, GameState, ObjectId, Pending, PlayerId, Resolution, YesNoPrompt,
};
use baylee_core::ids::DamageSourceRef;

/// Choices are gathered before the single proliferate instruction changes counters.
#[derive(Clone, Hash, Debug)]
pub struct Selection {
    objects: Vec<DamageSourceRef>,
    next_object: usize,
    chosen: Vec<DamageSourceRef>,
    players: Vec<PlayerId>,
    next_player: usize,
    chosen_players: Vec<PlayerId>,
}

pub(super) fn start(state: &mut GameState, res: &mut Resolution) -> Option<Pending> {
    let selection = Selection {
        objects: state
            .battlefield_seen()
            .filter_map(|id| {
                let o = state.object(id)?;
                (!o.counters.is_empty()).then_some(DamageSourceRef {
                    object: id,
                    version: o.version,
                })
            })
            .collect(),
        next_object: 0,
        chosen: Vec::new(),
        players: state
            .players
            .iter()
            .filter(|p| !p.has_lost() && p.counters().iter().any(|(_, n)| *n > 0))
            .map(|p| p.id)
            .collect(),
        next_player: 0,
        chosen_players: Vec::new(),
    };
    advance(state, res, selection)
}

pub(super) fn objects(
    state: &mut GameState,
    res: &mut Resolution,
    mut selection: Selection,
    chosen: &[ObjectId],
) -> Option<Pending> {
    let end = (selection.next_object + usize::from(u8::MAX)).min(selection.objects.len());
    selection.chosen.extend(
        selection.objects[selection.next_object..end]
            .iter()
            .filter(|r| chosen.contains(&r.object))
            .copied(),
    );
    selection.next_object = end;
    advance(state, res, selection)
}

fn player(
    state: &mut GameState,
    res: &mut Resolution,
    mut selection: Selection,
    yes: bool,
) -> Option<Pending> {
    if let Some(player) = selection.players.get(selection.next_player).copied() {
        if yes {
            selection.chosen_players.push(player);
        }
        selection.next_player += 1;
    }
    advance(state, res, selection)
}

fn advance(
    state: &mut GameState,
    res: &mut Resolution,
    mut selection: Selection,
) -> Option<Pending> {
    if selection.next_object < selection.objects.len() {
        let end = (selection.next_object + usize::from(u8::MAX)).min(selection.objects.len());
        let options: Vec<_> = selection.objects[selection.next_object..end]
            .iter()
            .map(|r| r.object)
            .collect();
        let max = u8::try_from(options.len()).expect("bounded choice page");
        res.awaiting = Some(AwaitingOp::Proliferate(Box::new(selection)));
        return Some(Pending::ChooseCards {
            player: res.controller,
            options,
            min: 0,
            max,
            prompt: ChoicePrompt::Proliferate,
            total: None,
        });
    }
    while let Some(player) = selection.players.get(selection.next_player).copied() {
        if state.has_left(player) {
            selection.next_player += 1;
            continue;
        }
        res.awaiting = Some(AwaitingOp::Proliferate(Box::new(selection)));
        return Some(Pending::YesNo {
            player: res.controller,
            prompt: YesNoPrompt::ProliferatePlayer { player },
            source: super::resolving_ability(state, res),
        });
    }
    // Freeze kinds before placing any counter; changes of characteristics from
    // one placement cannot change what the same instruction selected elsewhere.
    let objects: Vec<_> = selection
        .chosen
        .into_iter()
        .filter_map(|reference| {
            let o = state.object(reference.object)?;
            (o.version == reference.version
                && o.zone == crate::zone::Zone::Battlefield
                && !o.status.contains(crate::object::Status::PHASED_OUT))
            .then(|| {
                (
                    reference.object,
                    o.counters.iter().map(|(kind, _)| kind).collect::<Vec<_>>(),
                )
            })
        })
        .collect();
    let plans = objects
        .into_iter()
        .filter_map(|(object, kinds)| {
            crate::replacement::counter_order::plan(
                state,
                res.controller,
                object,
                kinds.into_iter().map(|kind| (kind, 1)).collect(),
                true,
                None,
            )
        })
        .collect();
    let player_mult =
        i32::try_from(state.player_counter_multiplier(res.controller)).unwrap_or(i32::MAX);
    let players = selection
        .chosen_players
        .into_iter()
        .flat_map(|player| {
            state.players[usize::from(player.get())]
                .counters()
                .into_iter()
                .filter(|(_, count)| *count > 0)
                .map(move |(kind, _)| (player, kind, player_mult))
        })
        .collect();
    crate::replacement::counter_order::enqueue_batch(state, plans, players);
    None
}

/// The player choice shares the same resolution continuation as its object pages.
pub(super) fn resume_player(
    state: &mut GameState,
    res: &mut Resolution,
    yes: bool,
) -> Option<super::Flow> {
    if !matches!(res.awaiting, Some(AwaitingOp::Proliferate(_))) {
        return None;
    }
    let Some(AwaitingOp::Proliferate(selection)) = res.awaiting.take() else {
        unreachable!("proliferate choice")
    };
    if let Some(pending) = player(state, res, *selection, yes) {
        return Some(super::Flow::Wait(pending));
    }
    res.pc += 1;
    Some(super::run(state, res))
}
