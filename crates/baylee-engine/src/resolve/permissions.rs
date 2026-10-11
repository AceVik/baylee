//! Permissions to play or cast individual cards from exile.
use super::{
    Cause, GameState, ObjectId, Pending, PlayerId, PlayerRel, Resolution, ZoneLocation,
    ZonePosition, players_of,
};

/// Gives `player` permission to play `card` this turn, for the object it
/// is now (`PlayPermission`).
pub(super) fn grant_play(state: &mut GameState, player: PlayerId, card: ObjectId, free: bool) {
    grant_permission(state, player, card, free, false);
}

/// [`grant_play`], or with `cast_only` the permission to cast `card` and
/// not to play it as a land (Ragavan, Nimble Pilferer, CR 601.1a).
fn grant_permission(
    state: &mut GameState,
    player: PlayerId,
    card: ObjectId,
    free: bool,
    cast_only: bool,
) {
    if let Some(version) = state.object(card).map(|o| o.version) {
        state.play_permissions.push(crate::state::PlayPermission {
            player,
            card,
            version,
            free,
            cast_only,
            spell_types: None,
            window: baylee_cards_dsl::PlayWindow::ThisTurn,
        });
    }
}

pub(super) fn exile_top_may_cast(
    state: &mut GameState,
    res: &Resolution,
    who: PlayerRel,
    nonland_only: bool,
) -> Option<Pending> {
    let you = res.controller;
    for player in players_of(who, state, you, res) {
        let Some(top) = state
            .zones
            .list(ZoneLocation::Library(player))
            .last()
            .copied()
        else {
            continue;
        };
        if state
            .move_object(
                top,
                ZoneLocation::Exile(player),
                ZonePosition::Top,
                Cause::Effect,
            )
            .is_ok()
            && (!nonland_only
                || state.object(top).is_some_and(|object| {
                    !object
                        .characteristics()
                        .types
                        .contains(baylee_core::types::TypeSet::LAND)
                }))
        {
            grant_permission(state, you, top, false, true);
        }
    }
    None
}

/// Exile up to the requested count; empty libraries never cause a draw loss.
pub(super) fn exile_top_to_play(
    state: &mut GameState,
    res: &Resolution,
    who: PlayerRel,
    play: baylee_cards_dsl::ExilePlay,
) -> Option<Pending> {
    for player in players_of(who, state, res.controller, res) {
        for _ in 0..play.count {
            let Some(&card) = state.zones.list(ZoneLocation::Library(player)).last() else {
                break;
            };
            if state
                .move_object(
                    card,
                    ZoneLocation::Exile(player),
                    ZonePosition::Top,
                    Cause::Effect,
                )
                .is_ok()
                && let Some(object) = state.object(card)
            {
                state.play_permissions.push(crate::state::PlayPermission {
                    player: res.controller,
                    card,
                    version: object.version,
                    free: play.free,
                    cast_only: play.spell_types.is_some(),
                    spell_types: play.spell_types,
                    window: play.window,
                });
            }
        }
    }
    None
}
