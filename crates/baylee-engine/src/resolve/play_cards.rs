//! Composable mid-resolution play/cast instructions (`Effect::Play`, `CR 608.2g`, `CR 707.12`).

use super::{
    AwaitingOp, ChoicePrompt, Effect, GameState, ObjectId, Pending, Resolution, ZoneLocation,
    players_of, subjects,
};
use crate::eval;
use crate::zone::Zone;
use baylee_cards_dsl::{PlayCostMode, PlayCount, PlaySource, TargetSpec, ZoneRef};
use baylee_core::ids::{DamageSourceRef, PlayerId};

/// Per-play mode flags carried by [`PlayOffer`].
#[derive(Clone, Copy, Debug, Hash)]
pub struct PlayFlags {
    /// Whether land plays are excluded.
    pub cast_only: bool,
    /// Whether a `SpellCopy` copy of the chosen card is cast instead of the original.
    pub copy: bool,
    /// Whether cast spells gain `Rider::ExileInsteadOfGraveyard`.
    pub exile_instead_of_graveyard: bool,
}

/// Suspended state for [`Effect::Play`] across one or more nested plays.
#[derive(Clone, Debug, Hash)]
pub struct PlayOffer {
    /// Seat instructed to play/cast.
    pub player: PlayerId,
    /// Candidate card incarnations remaining to be offered.
    pub cards: Vec<DamageSourceRef>,
    /// Remaining plays allowed (`None` for [`PlayCount::All`]).
    pub remaining_plays: Option<u8>,
    /// Cost mode for each play.
    pub cost: PlayCostMode,
    /// Casting flags (`cast_only`, `copy`, `exile_instead_of_graveyard`).
    pub flags: PlayFlags,
    /// Whether declining (`&[]`) is legal.
    pub optional: bool,
}

pub(super) fn start(state: &GameState, res: &mut Resolution, op: &Effect) -> Option<Pending> {
    let Effect::Play {
        source,
        filter,
        controller,
        count,
        cost,
        cast_only,
        copy,
        exile_instead_of_graveyard,
        optional,
    } = *op
    else {
        unreachable!("Effect::Play expected");
    };
    let remaining_plays = match count {
        PlayCount::One => Some(1),
        PlayCount::UpTo(0) => return None,
        PlayCount::UpTo(n) => Some(n),
        PlayCount::All => None,
    };
    let player = *players_of(controller, state, res.controller, res).first()?;
    if state.has_left(player) {
        return None;
    }
    let raw_candidates = gather_candidates(state, res, source);
    let mut cards = Vec::new();
    for candidate in raw_candidates {
        if cards.contains(&candidate) {
            continue;
        }
        let Some(obj) = state
            .object(candidate.object)
            .filter(|o| o.version == candidate.version && o.zone != Zone::Battlefield)
        else {
            continue;
        };
        if eval::matches_for_x(
            filter,
            state,
            obj,
            res.controller,
            res.rule_context(),
            res.x,
        ) {
            cards.push(candidate);
        }
    }
    continue_play(
        state,
        res,
        PlayOffer {
            player,
            cards,
            remaining_plays,
            cost,
            flags: PlayFlags {
                cast_only,
                copy,
                exile_instead_of_graveyard,
            },
            optional,
        },
    )
}

pub(crate) fn continue_play(
    state: &GameState,
    res: &mut Resolution,
    mut offer: PlayOffer,
) -> Option<Pending> {
    if offer.remaining_plays == Some(0) || state.has_left(offer.player) {
        return None;
    }
    offer.cards.retain(|r| {
        state
            .object(r.object)
            .is_some_and(|o| o.version == r.version && o.zone != Zone::Battlefield)
    });
    if offer.cards.is_empty() {
        return None;
    }
    let options: Vec<ObjectId> = offer.cards.iter().map(|r| r.object).collect();
    let player = offer.player;
    let min = u8::from(!offer.optional);
    res.awaiting = Some(AwaitingOp::PlayCards(Box::new(offer)));
    Some(Pending::ChooseCards {
        player,
        options,
        min,
        max: 1,
        prompt: ChoicePrompt::PlayFromExile,
        total: None,
    })
}

fn gather_candidates(
    state: &GameState,
    res: &Resolution,
    source: PlaySource,
) -> Vec<DamageSourceRef> {
    match source {
        PlaySource::LinkedExile => super::hidden_exile::linked(state, res),
        PlaySource::Subject(spec) => match spec {
            TargetSpec::ThisObject => subjects::moving_source(state, res).into_iter().collect(),
            TargetSpec::EventObject => subjects::event(state, res)
                .filter(|r| subjects::is_current(state, *r))
                .into_iter()
                .collect(),
            TargetSpec::CardInFirstTargetsGraveyard(_)
            | TargetSpec::ObjectOfFirstTargetsPlayer(_) => res
                .second_targets
                .iter()
                .filter_map(|&id| {
                    subjects::target(state, res, id).or_else(|| state.source_identity(id))
                })
                .filter(|r| subjects::is_current(state, *r))
                .collect(),
            _ => {
                if res.targets.is_empty() {
                    res.subject
                        .found
                        .iter()
                        .copied()
                        .filter(|r| subjects::is_current(state, *r))
                        .collect()
                } else {
                    res.targets
                        .iter()
                        .filter_map(|&id| {
                            subjects::target(state, res, id).or_else(|| state.source_identity(id))
                        })
                        .filter(|r| subjects::is_current(state, *r))
                        .collect()
                }
            }
        },
        PlaySource::Zone { zone, owner } => {
            let owners = players_of(owner, state, res.controller, res);
            let mut out = Vec::new();
            for seat in owners {
                match zone {
                    ZoneRef::Hand => {
                        for &id in state.zones.list(ZoneLocation::Hand(seat)) {
                            if let Some(r) = state.source_identity(id) {
                                out.push(r);
                            }
                        }
                    }
                    ZoneRef::Graveyard => {
                        for &id in state.zones.list(ZoneLocation::Graveyard(seat)) {
                            if let Some(r) = state.source_identity(id) {
                                out.push(r);
                            }
                        }
                    }
                    ZoneRef::Exile => {
                        for &id in state.zones.list(ZoneLocation::Exile(seat)) {
                            if let Some(r) = state.source_identity(id) {
                                out.push(r);
                            }
                        }
                    }
                    ZoneRef::Library => {
                        if let Some(&top) = state.zones.list(ZoneLocation::Library(seat)).last()
                            && let Some(r) = state.source_identity(top)
                        {
                            out.push(r);
                        }
                    }
                    ZoneRef::Command => {
                        for &id in state.zones.list(ZoneLocation::Command(seat)) {
                            if let Some(r) = state.source_identity(id) {
                                out.push(r);
                            }
                        }
                    }
                    ZoneRef::Battlefield
                    | ZoneRef::Stack
                    | ZoneRef::NotBattlefield
                    | ZoneRef::BattlefieldOrCommand => {}
                }
            }
            out
        }
    }
}
