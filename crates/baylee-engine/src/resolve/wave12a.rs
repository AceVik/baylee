//! Engine Wave 12a resolution operations:
//! - [`reveal_until_then`] (`Effect::RevealUntilThen`, `CR 701.20a`)
//! - [`repeat_each_object`] (`Effect::RepeatEachObject`, `CR 101.4`, `CR 608.2h`)

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::zone::Zone;
use baylee_cards_dsl::{Filter, RevealUntilFound, RevealUntilRest, ZoneSel};
use baylee_core::ids::DamageSourceRef;

/// Resolve [`Effect::RevealUntilThen`].
#[allow(clippy::too_many_arguments)]
pub(super) fn reveal_until_then(
    state: &mut GameState,
    res: &mut Resolution,
    library: PlayerRel,
    filter: &'static Filter,
    count: Amount,
    max_revealed: Option<Amount>,
    exile_while_revealing: bool,
    found: RevealUntilFound,
    rest: RevealUntilRest,
    then: &'static [Effect],
) -> Option<Pending> {
    let you = res.controller;
    let Some(lib_owner) = players_of(library, state, you, res).first().copied() else {
        clear_found_bindings(res);
        return None;
    };
    let target_hits = amount2(&count, state, you, res) as usize;
    let cap = max_revealed.map_or(usize::MAX, |m| amount2(&m, state, you, res) as usize);
    if target_hits == 0 || cap == 0 {
        clear_found_bindings(res);
        return None;
    }
    let (turned_over, hits) = turn_over_until(
        state,
        res,
        lib_owner,
        filter,
        target_hits,
        cap,
        exile_while_revealing,
    );
    if !matches!(found, RevealUntilFound::SameAsRest) {
        for &card in &hits {
            place_found_card(state, res, lib_owner, card, exile_while_revealing, found);
        }
    }
    let rest_cards: Vec<ObjectId> = if matches!(found, RevealUntilFound::SameAsRest) {
        turned_over
    } else {
        turned_over
            .into_iter()
            .filter(|c| !hits.contains(c))
            .collect()
    };
    let pending = place_rest_cards(
        state,
        res,
        lib_owner,
        rest_cards,
        exile_while_revealing,
        rest,
    );
    bind_hits_and_splice_then(state, res, &hits, then);
    pending
}

fn clear_found_bindings(res: &mut Resolution) {
    res.subject.found.clear();
    res.event_object = None;
    res.subject.set_event_identity(None);
}

fn turn_over_until(
    state: &mut GameState,
    res: &Resolution,
    lib_owner: PlayerId,
    filter: &'static Filter,
    target_hits: usize,
    cap: usize,
    exile_while_revealing: bool,
) -> (Vec<ObjectId>, Vec<ObjectId>) {
    let you = res.controller;
    let context = res.rule_context();
    let x = res.x;
    let mut turned_over = Vec::new();
    let mut hits = Vec::new();
    if exile_while_revealing {
        for _ in 0..cap {
            let Some(&top) = state.zones.list(ZoneLocation::Library(lib_owner)).last() else {
                break;
            };
            let owner = state.object(top).map_or(lib_owner, |o| o.owner);
            let _ = state.move_object(
                top,
                ZoneLocation::Exile(owner),
                ZonePosition::Top,
                Cause::Effect,
            );
            let Some(obj) = state.object(top).filter(|o| o.zone == Zone::Exile) else {
                break;
            };
            turned_over.push(top);
            if eval::matches_for_x(filter, state, obj, you, context, x) {
                hits.push(top);
                if hits.len() == target_hits {
                    break;
                }
            }
        }
        return (turned_over, hits);
    }
    let snapshot: Vec<ObjectId> = state
        .zones
        .list(ZoneLocation::Library(lib_owner))
        .iter()
        .rev()
        .take(cap)
        .copied()
        .collect();
    for card in snapshot {
        turned_over.push(card);
        if state
            .object(card)
            .is_some_and(|o| eval::matches_for_x(filter, state, o, you, context, x))
        {
            hits.push(card);
            if hits.len() == target_hits {
                break;
            }
        }
    }
    if !turned_over.is_empty() {
        state.journal.record(GameEvent::Revealed {
            player: lib_owner,
            cards: turned_over.clone(),
        });
    }
    (turned_over, hits)
}

fn place_found_card(
    state: &mut GameState,
    res: &Resolution,
    lib_owner: PlayerId,
    card: ObjectId,
    exile_while_revealing: bool,
    found: RevealUntilFound,
) {
    let owner = state.object(card).map_or(lib_owner, |o| o.owner);
    match found {
        RevealUntilFound::SameAsRest => {}
        RevealUntilFound::StayInLibrary => {
            if exile_while_revealing {
                let _ = state.move_object(
                    card,
                    ZoneLocation::Library(lib_owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
        }
        RevealUntilFound::TopOfLibrary => {
            let _ = state.move_object(
                card,
                ZoneLocation::Library(lib_owner),
                ZonePosition::Top,
                Cause::Effect,
            );
        }
        RevealUntilFound::Hand => {
            let _ = state.move_object(
                card,
                ZoneLocation::Hand(owner),
                ZonePosition::Top,
                Cause::Effect,
            );
        }
        RevealUntilFound::Battlefield {
            tapped,
            under_your_control,
        } => {
            if tapped {
                state.restore_tapped(card, true);
            }
            let controller = if under_your_control {
                res.controller
            } else {
                lib_owner
            };
            super::search::put_found(state, controller, card, SearchDest::Battlefield);
        }
        RevealUntilFound::Graveyard => {
            let _ = state.move_object(
                card,
                ZoneLocation::Graveyard(owner),
                ZonePosition::Top,
                Cause::Effect,
            );
        }
        RevealUntilFound::Exile => {
            if !exile_while_revealing {
                let _ = state.move_object(
                    card,
                    ZoneLocation::Exile(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
        }
        RevealUntilFound::ExileLinked => link_in_exile(state, res, owner, card),
    }
}

fn link_in_exile(state: &mut GameState, res: &Resolution, owner: PlayerId, id: ObjectId) {
    let fallback_version =
        super::source_version(state, res).or_else(|| state.object(res.source).map(|o| o.version));
    let source = state
        .recorded_ability_source(res.on_stack)
        .or_else(|| super::subjects::source(state, res))
        .unwrap_or(DamageSourceRef {
            object: res.source,
            version: fallback_version.unwrap_or(0),
        });
    let acquisition = super::subjects::acquisition(state, res);
    let components = state.merged_components(id);
    let link = crate::object::Rider::Linked {
        host: source.object,
        until: None,
    };
    let with = crate::object::Rider::ExiledWith {
        host: source.object,
        version: u16::try_from(source.version).unwrap_or(u16::MAX),
    };
    let linked_exile = crate::object::Rider::LinkedExile {
        source,
        acquisition,
    };
    if state.object(id).is_some_and(|o| o.zone != Zone::Exile) {
        if let Some(obj) = state.object_mut(id) {
            obj.kind = ObjectKind::Card;
            obj.riders.push(link);
            obj.riders.push(with);
        }
        let _ = state.move_object(
            id,
            ZoneLocation::Exile(owner),
            ZonePosition::Top,
            Cause::Effect,
        );
    }
    for component in components {
        if let Some(object) = state
            .object_mut(component)
            .filter(|o| o.zone == Zone::Exile)
        {
            for rider in [link, with, linked_exile] {
                if !object.riders.contains(&rider) {
                    object.riders.push(rider);
                }
            }
            state.remember_link(component, source);
        }
    }
}

fn place_rest_cards(
    state: &mut GameState,
    res: &mut Resolution,
    lib_owner: PlayerId,
    rest_cards: Vec<ObjectId>,
    exile_while_revealing: bool,
    rest: RevealUntilRest,
) -> Option<Pending> {
    if matches!(rest, RevealUntilRest::ShuffleIntoLibrary) {
        if exile_while_revealing {
            let lib = ZoneLocation::Library(lib_owner);
            for card in rest_cards {
                let _ = state.move_object(card, lib, ZonePosition::Bottom, Cause::Effect);
            }
        }
        state.shuffle_library(lib_owner);
        return None;
    }
    if rest_cards.is_empty() {
        return None;
    }
    match rest {
        RevealUntilRest::StayInLibrary => {
            if exile_while_revealing {
                let lib = ZoneLocation::Library(lib_owner);
                for &card in rest_cards.iter().rev() {
                    let _ = state.move_object(card, lib, ZonePosition::Top, Cause::Effect);
                }
            }
            None
        }
        RevealUntilRest::TopOfLibrary => {
            let lib = ZoneLocation::Library(lib_owner);
            for &card in rest_cards.iter().rev() {
                let _ = state.move_object(card, lib, ZonePosition::Top, Cause::Effect);
            }
            None
        }
        RevealUntilRest::BottomRandom => {
            state.put_on_bottom_in_random_order(lib_owner, rest_cards);
            None
        }
        RevealUntilRest::BottomChosenOrder => {
            if rest_cards.len() == 1 {
                let _ = state.move_object(
                    rest_cards[0],
                    ZoneLocation::Library(lib_owner),
                    ZonePosition::Bottom,
                    Cause::Effect,
                );
                return None;
            }
            res.awaiting = Some(AwaitingOp::PeekBottom { player: lib_owner });
            let n = u32::try_from(rest_cards.len()).unwrap_or(u32::MAX);
            Some(Pending::Arrange {
                player: res.controller,
                cards: rest_cards,
                piles: vec![ArrangePile::all_of(ArrangePlace::LibraryBottom, n)],
                prompt: ArrangePrompt::Order,
            })
        }
        RevealUntilRest::ShuffleIntoLibrary => unreachable!("handled above"),
        RevealUntilRest::Hand => {
            move_rest_to_owner_zone(state, lib_owner, rest_cards, ZoneLocation::Hand);
            None
        }
        RevealUntilRest::Graveyard => {
            move_rest_to_owner_zone(state, lib_owner, rest_cards, ZoneLocation::Graveyard);
            None
        }
        RevealUntilRest::Exile => {
            if !exile_while_revealing {
                move_rest_to_owner_zone(state, lib_owner, rest_cards, ZoneLocation::Exile);
            }
            None
        }
    }
}

fn move_rest_to_owner_zone(
    state: &mut GameState,
    lib_owner: PlayerId,
    rest_cards: Vec<ObjectId>,
    zone_of: fn(PlayerId) -> ZoneLocation,
) {
    for card in rest_cards {
        let owner = state.object(card).map_or(lib_owner, |o| o.owner);
        let _ = state.move_object(card, zone_of(owner), ZonePosition::Top, Cause::Effect);
    }
}

fn bind_hits_and_splice_then(
    state: &GameState,
    res: &mut Resolution,
    hits: &[ObjectId],
    then: &'static [Effect],
) {
    res.subject.found.clear();
    if hits.is_empty() {
        res.event_object = None;
        res.subject.set_event_identity(None);
        return;
    }
    for &card in hits {
        if let Some(id_ref) = state.source_identity(card) {
            res.subject.found.push(id_ref);
        }
    }
    super::wave3_ops::bind_remembered_object(state, res, hits[0]);
    if !then.is_empty() {
        let next = res.pc + 1;
        res.effects.splice(next..next, super::flatten(then));
    }
}

/// Entry and step-advance handler for [`Effect::RepeatEachObject`].
pub(super) fn repeat_each_object(
    state: &mut GameState,
    res: &mut Resolution,
    zone: ZoneSel,
    filter: &'static Filter,
    only_found: bool,
    then: &'static [Effect],
    op: Effect,
) -> Option<Pending> {
    let after_len = res.effects.len().saturating_sub(res.pc + 1);
    let resuming = res
        .subject
        .repeat_frames
        .last()
        .is_some_and(|frame| frame.tail_from_end == after_len);
    if !resuming {
        if then.is_empty() {
            return None;
        }
        let steps = build_object_steps(state, res, zone, filter, only_found);
        if steps.is_empty() {
            return None;
        }
        res.subject
            .repeat_frames
            .push(super::wave3_ops::RepeatFrame {
                tail_from_end: after_len,
                steps: super::wave3_ops::RepeatSteps::Items(steps),
                saved_event_object: res.event_object,
                saved_event_identity: res.subject.event_identity(),
                saved_event_player: res.subject.event_player(),
            });
    }
    super::wave3_ops::advance_repeat_frame(state, res, then, op)
}

fn build_object_steps(
    state: &GameState,
    res: &Resolution,
    zone: ZoneSel,
    filter: &'static Filter,
    only_found: bool,
) -> Vec<super::wave3_ops::RepeatStep> {
    let you = res.controller;
    let context = res.rule_context();
    let x = res.x;
    let cap = super::immediate::REPEAT_CAP as usize;
    let raw = snapshot_zone_candidates(state, res, zone);
    let mut matched: Vec<super::wave3_ops::RepeatStep> = Vec::new();
    for id in raw {
        let Some(obj) = state.object(id) else {
            continue;
        };
        if only_found
            && !res
                .subject
                .found
                .iter()
                .any(|r| r.object == id && r.version == obj.version)
        {
            continue;
        }
        if !eval::matches_for_x(filter, state, obj, you, context, x) {
            continue;
        }
        let event_player = if obj.zone == Zone::Battlefield {
            obj.controller
        } else {
            obj.owner
        };
        matched.push(super::wave3_ops::RepeatStep::ObjectInZone {
            identity: DamageSourceRef {
                object: id,
                version: obj.version,
            },
            zone: obj.zone,
            event_player,
        });
        if matched.len() >= cap {
            break;
        }
    }
    matched.reverse();
    matched
}

fn apnap_seats_of(state: &GameState, mut seats: Vec<PlayerId>) -> Vec<PlayerId> {
    seats.sort_by_key(|p| crate::battle::sba::apnap_distance(state, *p));
    seats.dedup();
    seats
}

fn all_seats_apnap(state: &GameState) -> Vec<PlayerId> {
    apnap_seats_of(state, state.players.iter().map(|p| p.id).collect())
}

fn snapshot_zone_candidates(state: &GameState, res: &Resolution, zone: ZoneSel) -> Vec<ObjectId> {
    let you = res.controller;
    match zone {
        ZoneSel::Battlefield => {
            let mut pairs: Vec<(ObjectId, PlayerId)> = state
                .battlefield_seen()
                .filter_map(|id| state.object(id).map(|o| (id, o.controller)))
                .collect();
            pairs.sort_by_key(|(_, c)| crate::battle::sba::apnap_distance(state, *c));
            pairs.into_iter().map(|(id, _)| id).collect()
        }
        ZoneSel::LinkedExile => {
            let mut cards = eval::linked_exile_cards(
                state,
                you,
                res.rule_context(),
                super::source_version(state, res),
                &Filter::Any,
            );
            cards.sort_by_key(|&id| {
                state.object(id).map_or(usize::MAX, |o| {
                    crate::battle::sba::apnap_distance(state, o.owner)
                })
            });
            cards
        }
        ZoneSel::ExileAll => all_seats_apnap(state)
            .into_iter()
            .flat_map(|s| state.zones.list(ZoneLocation::Exile(s)).iter().copied())
            .collect(),
        ZoneSel::GraveyardYou => state.zones.list(ZoneLocation::Graveyard(you)).clone(),
        ZoneSel::GraveyardOpponents => all_seats_apnap(state)
            .into_iter()
            .filter(|&s| !state.has_left(s) && state.is_opponent(s, you))
            .flat_map(|s| state.zones.list(ZoneLocation::Graveyard(s)).iter().copied())
            .collect(),
        ZoneSel::GraveyardAll => all_seats_apnap(state)
            .into_iter()
            .flat_map(|s| state.zones.list(ZoneLocation::Graveyard(s)).iter().copied())
            .collect(),
        ZoneSel::GraveyardOf(who) => apnap_seats_of(state, players_of(who, state, you, res))
            .into_iter()
            .flat_map(|s| state.zones.list(ZoneLocation::Graveyard(s)).iter().copied())
            .collect(),
        ZoneSel::HandYou => state.zones.list(ZoneLocation::Hand(you)).clone(),
        ZoneSel::HandActivePlayer => state
            .zones
            .list(ZoneLocation::Hand(state.turn.active))
            .clone(),
        ZoneSel::HandChosen => {
            apnap_seats_of(state, players_of(PlayerRel::Chosen, state, you, res))
                .into_iter()
                .flat_map(|s| state.zones.list(ZoneLocation::Hand(s)).iter().copied())
                .collect()
        }
        ZoneSel::HandOf(who) => apnap_seats_of(state, players_of(who, state, you, res))
            .into_iter()
            .flat_map(|s| state.zones.list(ZoneLocation::Hand(s)).iter().copied())
            .collect(),
        ZoneSel::LibraryYou => state
            .zones
            .list(ZoneLocation::Library(you))
            .iter()
            .rev()
            .copied()
            .collect(),
        ZoneSel::LibraryOf(who) => apnap_seats_of(state, players_of(who, state, you, res))
            .into_iter()
            .flat_map(|s| {
                state
                    .zones
                    .list(ZoneLocation::Library(s))
                    .iter()
                    .rev()
                    .copied()
            })
            .collect(),
    }
}
