//! Untargeted graveyard choices, including a command-zone replacement.
#[allow(clippy::wildcard_imports)]
use super::*;

pub(super) fn start(
    state: &GameState,
    res: &mut Resolution,
    filter: &'static baylee_cards_dsl::Filter,
    destination: SearchDest,
    tapped: bool,
    optional: bool,
) -> Option<Pending> {
    let you = res.controller;
    if state.has_left(you) {
        return None;
    }
    let options: Vec<_> = state
        .zones
        .list(ZoneLocation::Graveyard(you))
        .iter()
        .copied()
        .filter(|card| {
            state.object(*card).is_some_and(|object| {
                eval::matches_with_context(filter, state, object, you, res.rule_context())
            })
        })
        .collect();
    if options.is_empty() {
        return None;
    }
    res.awaiting = Some(AwaitingOp::ChosenGraveyard {
        destination,
        tapped,
    });
    Some(Pending::ChooseCards {
        player: you,
        options,
        min: u8::from(!optional),
        max: 1,
        prompt: ChoicePrompt::FromGraveyard,
        total: None,
    })
}

pub(super) fn place(
    state: &mut GameState,
    res: &mut Resolution,
    chosen: &[ObjectId],
    destination: SearchDest,
    tapped: bool,
) -> Option<Pending> {
    let card = *chosen.first()?;
    let you = res.controller;
    let version = state
        .object(card)
        .filter(|object| {
            object.zone == crate::zone::Zone::Graveyard && object.zone_owner == Some(you)
        })?
        .version;
    let to = match destination {
        SearchDest::Graveyard => ZoneLocation::Graveyard(you),
        SearchDest::Exile => ZoneLocation::Exile(state.object(card).map_or(you, |o| o.owner)),
        SearchDest::Hand => ZoneLocation::Hand(you),
        SearchDest::TopOfLibrary => ZoneLocation::Library(you),
        SearchDest::Battlefield => ZoneLocation::Battlefield,
    };
    if let Some(pending) = ask_commander_replace(state, res, &[(card, to)]) {
        res.awaiting = Some(AwaitingOp::ChosenGraveyardCommander {
            card,
            version,
            destination,
            tapped,
        });
        return Some(pending);
    }
    if tapped && destination == SearchDest::Battlefield {
        state.set_tapped(card, true);
    }
    put_found(state, you, card, destination);
    None
}

/// Answer the command-zone replacement without replaying the preceding mill.
pub(super) fn resume_commander(
    state: &mut GameState,
    res: &mut Resolution,
    answer: bool,
) -> Option<Flow> {
    if let Some(AwaitingOp::ChosenGraveyardCommander {
        card,
        version,
        destination,
        tapped,
    }) = res.awaiting
    {
        if state
            .object(card)
            .is_none_or(|object| object.version != version)
        {
            res.awaiting = None;
            res.pc += 1;
            return Some(run(state, res));
        }
        state.commander_redirect.push((card, answer));
        res.awaiting = Some(AwaitingOp::ChosenGraveyard {
            destination,
            tapped,
        });
        return Some(super::resume::resume_inner(state, res, &[card]));
    }
    None
}
