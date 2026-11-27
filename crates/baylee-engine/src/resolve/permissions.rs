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
            mana_any_type: false,
            cast_only,
            spell_types: None,
            window: baylee_cards_dsl::PlayWindow::ThisTurn.into(),
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
                    mana_any_type: play.mana_any_type,
                    cast_only: play.spell_types.is_some(),
                    spell_types: play.spell_types,
                    window: play.window.into(),
                });
            }
        }
    }
    None
}

fn grant_exile_permission(
    state: &mut GameState,
    res: &Resolution,
    card: ObjectId,
    permission: baylee_cards_dsl::ExilePermission,
) {
    let Some(card_obj) = state.object(card) else {
        return;
    };
    let card_version = card_obj.version;
    let card_owner = card_obj.owner;
    let source_ref = super::source_version(state, res)
        .or_else(|| state.object(res.source).map(|o| o.version))
        .filter(|&ver| {
            permission.window != baylee_cards_dsl::PlayWindow::WhileSourceOnBattlefield
                || (state.object(res.source).is_some_and(|o| {
                    o.zone == crate::zone::Zone::Battlefield
                        && o.version == ver
                        && !o.status.contains(crate::object::Status::PHASED_OUT)
                }) && state
                    .object(res.on_stack)
                    .is_none_or(|s| s.source_power_lki.is_none()))
        })
        .map(|version| baylee_core::ids::DamageSourceRef {
            object: res.source,
            version,
        });
    let any_color = permission.mana == baylee_cards_dsl::ExileMana::AnyColor;
    let Some(window) =
        crate::state::PlayLifetime::from_window(permission.window, source_ref, any_color)
    else {
        return;
    };
    let grantees: smallvec::SmallVec<[PlayerId; 2]> = match permission.caster {
        PlayerRel::OwnerOfTarget => (!state.has_left(card_owner))
            .then_some(card_owner)
            .into_iter()
            .collect(),
        PlayerRel::OwnerOfEvent => {
            let from_event = players_of(PlayerRel::OwnerOfEvent, state, res.controller, res);
            if from_event.is_empty() {
                (!state.has_left(card_owner))
                    .then_some(card_owner)
                    .into_iter()
                    .collect()
            } else {
                from_event
                    .into_iter()
                    .filter(|p| !state.has_left(*p))
                    .collect()
            }
        }
        other => players_of(other, state, res.controller, res)
            .into_iter()
            .filter(|p| !state.has_left(*p))
            .collect(),
    };
    for player in grantees {
        state.play_permissions.push(crate::state::PlayPermission {
            player,
            card,
            version: card_version,
            free: permission.free,
            mana_any_type: permission.mana == baylee_cards_dsl::ExileMana::AnyType,
            cast_only: permission.cast_only,
            spell_types: permission.spell_types,
            window,
        });
    }
}

/// Exiles up to `count` cards from the top of `who`'s library and grants
/// `permission` on each exiled card (`CR 609.4`, `CR 609.4b`, `CR 601.1a`).
pub(super) fn exile_top_with_permission(
    state: &mut GameState,
    res: &Resolution,
    who: PlayerRel,
    count: &baylee_cards_dsl::Amount,
    permission: baylee_cards_dsl::ExilePermission,
) -> Option<Pending> {
    let n =
        crate::eval::amount_with_context(count, state, res.controller, res.rule_context(), res.x)
            as usize;
    for player in players_of(who, state, res.controller, res) {
        for _ in 0..n {
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
            {
                grant_exile_permission(state, res, card, permission);
            }
        }
    }
    None
}

/// Exiles `target` (optionally until `until`, honoring `CR 610.3a–b` if that
/// event already passed), links the exiled card(s) to `res.source`, and grants
/// `permission` on each exiled card component.
pub(super) fn exile_with_permission(
    state: &mut GameState,
    res: &Resolution,
    target: baylee_cards_dsl::TargetSpec,
    until: Option<baylee_cards_dsl::ExileUntil>,
    permission: baylee_cards_dsl::ExilePermission,
) -> Option<Pending> {
    let you = res.controller;
    let until = until.map(|until| match until {
        baylee_cards_dsl::ExileUntil::SourceLeavesBattlefield => {
            crate::object::LinkUntil::HostLeaves
        }
        baylee_cards_dsl::ExileUntil::OpponentBecomesMonarch => {
            crate::object::LinkUntil::OpponentBecomesMonarch { of: you }
        }
    });
    if until == Some(crate::object::LinkUntil::HostLeaves)
        && (state
            .object(res.source)
            .is_none_or(|host| host.zone != crate::zone::Zone::Battlefield)
            || state
                .object(res.on_stack)
                .is_some_and(|ability| ability.source_power_lki.is_some()))
    {
        return None;
    }
    if until == Some(crate::object::LinkUntil::OpponentBecomesMonarch { of: you })
        && super::zones::an_opponent_was_crowned_since(state, res.on_stack, you)
    {
        return None;
    }
    let host_version =
        super::source_version(state, res).or_else(|| state.object(res.source).map(|o| o.version));
    for target_id in super::zones::moving_objects(state, res, target) {
        let components = state.merged_components(target_id);
        let owner = state.object(target_id).map_or(you, |o| o.owner);
        let already_in_exile = state
            .object(target_id)
            .is_some_and(|o| o.zone == crate::zone::Zone::Exile);
        if !already_in_exile {
            if let Some(obj) = state.object_mut(target_id) {
                obj.kind = crate::object::ObjectKind::Card;
                obj.riders.push(crate::object::Rider::Linked {
                    host: res.source,
                    until,
                });
            }
            let _ = state.move_object(
                target_id,
                ZoneLocation::Exile(owner),
                ZonePosition::Top,
                Cause::Effect,
            );
        }
        for component in components {
            let in_exile = state
                .object(component)
                .is_some_and(|o| o.zone == crate::zone::Zone::Exile);
            if !in_exile {
                continue;
            }
            if let Some(version) = host_version {
                if let Some(object) = state.object_mut(component) {
                    let link = crate::object::Rider::Linked {
                        host: res.source,
                        until,
                    };
                    if !object.riders.contains(&link) {
                        object.riders.push(link);
                    }
                }
                state.remember_link(
                    component,
                    baylee_core::ids::DamageSourceRef {
                        object: res.source,
                        version,
                    },
                );
            }
            grant_exile_permission(state, res, component, permission);
        }
    }
    None
}

/// Grants `permission` on every face-up card in exile currently linked to `res.source`.
pub(super) fn grant_linked_exile_permission(
    state: &mut GameState,
    res: &Resolution,
    permission: baylee_cards_dsl::ExilePermission,
) -> Option<Pending> {
    let cards = crate::eval::linked_exile_cards(
        state,
        res.controller,
        res.rule_context(),
        super::source_version(state, res),
        &baylee_cards_dsl::Filter::Any,
    );
    for card in cards {
        grant_exile_permission(state, res, card, permission);
    }
    None
}

/// Grants `permission` on every currently remembered face-up card in exile matching `filter`.
pub(super) fn grant_remembered_exile_permission(
    state: &mut GameState,
    res: &Resolution,
    filter: &'static baylee_cards_dsl::Filter,
    permission: baylee_cards_dsl::ExilePermission,
) -> Option<Pending> {
    let remembered = res.subject.remembered()?;
    let cards: Vec<ObjectId> = remembered
        .iter()
        .filter_map(|rem| {
            let obj = state.object(rem.reference.object).filter(|o| {
                o.zone == crate::zone::Zone::Exile && o.version == rem.reference.version
            })?;
            super::matches_in_resolution(filter, state, obj, res.controller, res, None)
                .then_some(obj.id)
        })
        .collect();
    for card in cards {
        grant_exile_permission(state, res, card, permission);
    }
    None
}
