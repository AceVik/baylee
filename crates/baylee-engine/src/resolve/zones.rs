//! Zone movements: exile, blink, bounce, destruction, sacrifice,
//! graveyard recursion, mill — and countering spells/abilities (their
//! target's journey from the stack to another zone).

#[allow(clippy::wildcard_imports)] // family modules share the resolve vocabulary
use super::*;

/// Executes one zone-movement effect.
#[allow(clippy::too_many_lines)] // the zone vocabulary is one flat table
pub(super) fn exec(state: &mut GameState, res: &mut Resolution, op: Effect) -> Option<Pending> {
    let you = res.controller;
    match op {
        Effect::Exile { .. } => {
            if let Some(&target_id) = res.targets.first() {
                let owner = state.object(target_id).map_or(you, |o| o.owner);
                if let Some(obj) = state.object_mut(target_id) {
                    obj.kind = ObjectKind::Card;
                }
                let _ = state.move_object(
                    target_id,
                    ZoneLocation::Exile(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            None
        }
        Effect::Blink { .. } => {
            if let Some(&target_id) = res.targets.first() {
                let owner = state.object(target_id).map_or(you, |o| o.owner);
                if let Some(obj) = state.object_mut(target_id) {
                    obj.kind = ObjectKind::Card;
                }
                let _ = state.move_object(
                    target_id,
                    ZoneLocation::Exile(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
                if let Some(obj) = state.object_mut(target_id) {
                    obj.kind = ObjectKind::Permanent;
                    // Blink returns under its OWNER's control (Eerie
                    // Interlude, Momentary Blink family, CR 610.1 note:
                    // "return … under its owner's control").
                    obj.set_controller(owner);
                }
                let _ = state.move_object(
                    target_id,
                    ZoneLocation::Battlefield,
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            None
        }
        Effect::ReturnToHand { .. } => {
            if let Some(&target_id) = res.targets.first() {
                let owner = state.object(target_id).map_or(you, |o| o.owner);
                if let Some(obj) = state.object_mut(target_id) {
                    obj.kind = ObjectKind::Card;
                }
                let _ = state.move_object(
                    target_id,
                    ZoneLocation::Hand(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            None
        }
        Effect::ReturnAllToHand {
            filter,
            opponents_only,
        } => {
            let all: Vec<ObjectId> = state
                .zones
                .list(ZoneLocation::Battlefield)
                .iter()
                .filter(|id| {
                    state.object(**id).is_some_and(|o| {
                        (!opponents_only || state.is_opponent(o.controller, you))
                            && eval::matches(filter, state, o, you, res.source)
                    })
                })
                .copied()
                .collect();
            for id in all {
                let owner = state.object(id).map_or(you, |o| o.owner);
                if let Some(obj) = state.object_mut(id) {
                    obj.kind = ObjectKind::Card;
                }
                let _ = state.move_object(
                    id,
                    ZoneLocation::Hand(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            None
        }
        Effect::DestroyAll { filter } => {
            let all: Vec<ObjectId> = state
                .zones
                .list(ZoneLocation::Battlefield)
                .iter()
                .filter(|id| {
                    state
                        .object(**id)
                        .is_some_and(|o| eval::matches(filter, state, o, you, res.source))
                })
                .copied()
                .collect();
            for id in all {
                sba::destroy(state, id);
            }
            None
        }
        Effect::ExileGraveyard { player } => {
            for player in eval::players(player, state, you) {
                let cards: Vec<ObjectId> =
                    state.zones.list(ZoneLocation::Graveyard(player)).clone();
                for card in cards {
                    let _ = state.move_object(
                        card,
                        ZoneLocation::Exile(player),
                        ZonePosition::Top,
                        Cause::Effect,
                    );
                }
            }
            None
        }
        Effect::GraveyardToHand { .. } => {
            if let Some(&target_id) = res.targets.first() {
                let owner = state.object(target_id).map_or(you, |o| o.owner);
                let _ = state.move_object(
                    target_id,
                    ZoneLocation::Hand(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            None
        }
        Effect::GraveyardToTop { .. } => {
            if let Some(&target_id) = res.targets.first() {
                let owner = state.object(target_id).map_or(you, |o| o.owner);
                let _ = state.move_object(
                    target_id,
                    ZoneLocation::Library(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            None
        }
        Effect::GraveyardToBattlefield { .. } => {
            if let Some(&target_id) = res.targets.first() {
                if let Some(obj) = state.object_mut(target_id) {
                    obj.kind = ObjectKind::Permanent;
                    obj.set_controller(you);
                }
                let _ = state.move_object(
                    target_id,
                    ZoneLocation::Battlefield,
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            None
        }
        Effect::PutSourceOnTopOfLibrary => {
            let owner = state.object(res.source).map_or(you, |o| o.owner);
            let _ = state.move_object(
                res.source,
                ZoneLocation::Library(owner),
                ZonePosition::Top,
                Cause::Effect,
            );
            None
        }
        Effect::BottomCardFromHand { player, filter } => {
            let player = eval::players(player, state, you).first().copied()?;
            let options: Vec<ObjectId> = state
                .zones
                .list(ZoneLocation::Hand(player))
                .iter()
                .filter(|id| {
                    state
                        .object(**id)
                        .is_some_and(|o| eval::matches(filter, state, o, player, res.source))
                })
                .copied()
                .collect();
            if options.is_empty() {
                return None;
            }
            res.awaiting = Some(AwaitingOp::BottomFromHand { player });
            Some(Pending::ChooseCards {
                player,
                options,
                min: 0,
                max: 1,
                prompt: ChoicePrompt::Generic,
            })
        }
        Effect::ShuffleGraveyardIntoLibrary => {
            let graveyard: Vec<ObjectId> = state.zones.list(ZoneLocation::Graveyard(you)).clone();
            for card in graveyard {
                let _ = state.move_object(
                    card,
                    ZoneLocation::Library(you),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            state.shuffle_library(you);
            None
        }
        Effect::PhaseOut { target } => {
            let target_id = match target {
                Some(_) => res.targets.first().copied(),
                None => Some(res.source),
            };
            if let Some(id) = target_id {
                if let Some(obj) = state.object_mut(id) {
                    obj.status.insert(Status::PHASED_OUT);
                }
                state.journal.record(GameEvent::PhaseChanged {
                    object: id,
                    phased_out: true,
                });
            }
            None
        }
        Effect::ExileLinked { .. } => {
            if let Some(&target_id) = res.targets.first() {
                let owner = state.object(target_id).map_or(you, |o| o.owner);
                if let Some(obj) = state.object_mut(target_id) {
                    obj.kind = ObjectKind::Card;
                    obj.riders
                        .push(crate::object::Rider::Linked { host: res.source });
                }
                let _ = state.move_object(
                    target_id,
                    ZoneLocation::Exile(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            None
        }
        Effect::SacrificeSelf => {
            let owner = state.object(res.source).map_or(you, |o| o.owner);
            if let Some(obj) = state.object_mut(res.source) {
                obj.kind = ObjectKind::Card;
            }
            let _ = state.move_object(
                res.source,
                ZoneLocation::Graveyard(owner),
                ZonePosition::Top,
                Cause::Effect,
            );
            None
        }
        Effect::PutTargetOnBottomOfLibrary => {
            for &target in &res.targets {
                let owner = state.object(target).map_or(you, |o| o.owner);
                if let Some(obj) = state.object_mut(target) {
                    obj.kind = ObjectKind::Card;
                }
                let _ = state.move_object(
                    target,
                    ZoneLocation::Library(owner),
                    ZonePosition::Bottom,
                    Cause::Effect,
                );
            }
            None
        }
        Effect::ExileSource => {
            let owner = state.object(res.source).map_or(you, |o| o.owner);
            if let Some(obj) = state.object_mut(res.source) {
                obj.kind = ObjectKind::Card;
            }
            let _ = state.move_object(
                res.source,
                ZoneLocation::Exile(owner),
                ZonePosition::Top,
                Cause::Effect,
            );
            None
        }
        Effect::ExileAndReturnAtEndStep => {
            for &target in &res.targets {
                let owner = state.object(target).map_or(you, |o| o.owner);
                let _ = state.move_object(
                    target,
                    ZoneLocation::Exile(owner),
                    ZonePosition::Top,
                    crate::event::Cause::Effect,
                );
                state.delayed.push(crate::state::DelayedTrigger {
                    controller: you,
                    when: crate::state::DelayedWhen::NextEndStep,
                    action: crate::state::DelayedAction::ReturnToBattlefield { card: target },
                });
            }
            None
        }
        Effect::ExileLibraryAndShuffleHand { player } => {
            for player in eval::players(player, state, you) {
                let lib: Vec<ObjectId> = state.zones.list(ZoneLocation::Library(player)).clone();
                for card in lib {
                    let _ = state.move_object(
                        card,
                        ZoneLocation::Exile(player),
                        ZonePosition::Top,
                        Cause::Effect,
                    );
                }
                let hand: Vec<ObjectId> = state.zones.list(ZoneLocation::Hand(player)).clone();
                for card in hand {
                    let _ = state.move_object(
                        card,
                        ZoneLocation::Library(player),
                        ZonePosition::Bottom,
                        Cause::Effect,
                    );
                }
                state.shuffle_library(player);
            }
            None
        }
        Effect::Mill { amount, target } => {
            let n = amount2(&amount, state, you, res.source, res.x, &res.targets) as usize;
            for player in super::players_of(target, state, you, res) {
                let top: Vec<ObjectId> = state
                    .zones
                    .list(ZoneLocation::Library(player))
                    .iter()
                    .rev()
                    .take(n)
                    .copied()
                    .collect();
                for card in top {
                    let _ = state.move_object(
                        card,
                        ZoneLocation::Graveyard(player),
                        ZonePosition::Top,
                        Cause::Effect,
                    );
                }
            }
            None
        }
        Effect::Destroy { .. } => {
            if let Some(&target_id) = res.targets.first() {
                sba::destroy(state, target_id);
            }
            None
        }
        Effect::DestroyChosenForPlayers { who, filter } => {
            // Same per-player choice chain as SacrificeFilter, destroying
            // instead (The True Scriptures I).
            let mut players = eval::players(who, state, you);
            players.retain(|p| {
                state
                    .zones
                    .list(ZoneLocation::Battlefield)
                    .iter()
                    .any(|id| {
                        state.object(*id).is_some_and(|o| {
                            o.controller == *p && eval::matches(filter, state, o, you, res.source)
                        })
                    })
            });
            let player = players.first().copied()?;
            players.remove(0);
            let options: Vec<ObjectId> = state
                .zones
                .list(ZoneLocation::Battlefield)
                .iter()
                .filter(|id| {
                    state.object(**id).is_some_and(|o| {
                        o.controller == player && eval::matches(filter, state, o, you, res.source)
                    })
                })
                .copied()
                .collect();
            res.awaiting = Some(AwaitingOp::DestroyChosen {
                filter,
                remaining: players,
            });
            Some(Pending::ChooseCards {
                player,
                options,
                min: 0,
                max: 1,
                prompt: ChoicePrompt::Generic,
            })
        }
        Effect::SacrificeFilter { who, filter } => {
            let mut players = eval::players(who, state, you);
            players.retain(|p| {
                state
                    .zones
                    .list(ZoneLocation::Battlefield)
                    .iter()
                    .any(|id| {
                        state.object(*id).is_some_and(|o| {
                            o.controller == *p && eval::matches(filter, state, o, you, res.source)
                        })
                    })
            });
            let player = players.first().copied()?;
            players.remove(0);
            let options: Vec<ObjectId> = state
                .zones
                .list(ZoneLocation::Battlefield)
                .iter()
                .filter(|id| {
                    state.object(**id).is_some_and(|o| {
                        o.controller == player && eval::matches(filter, state, o, you, res.source)
                    })
                })
                .copied()
                .collect();
            res.awaiting = Some(AwaitingOp::SacrificeFilter {
                filter,
                remaining: players,
            });
            Some(Pending::ChooseCards {
                player,
                options,
                min: 1,
                max: 1,
                prompt: ChoicePrompt::Generic,
            })
        }
        Effect::DiscardForPlayers { who, count } => {
            let players = eval::players(who, state, you);
            let mut remaining: Vec<PlayerId> = players
                .iter()
                .copied()
                .filter(|p| !state.zones.list(ZoneLocation::Hand(*p)).is_empty())
                .collect();
            let player = remaining.first().copied()?;
            remaining.remove(0);
            let hand: Vec<ObjectId> = state.zones.list(ZoneLocation::Hand(player)).clone();
            let n = (count as usize).min(hand.len()) as u8;
            res.awaiting = Some(AwaitingOp::DiscardChain {
                player,
                count,
                remaining,
            });
            Some(Pending::ChooseCards {
                player,
                options: hand,
                min: n,
                max: n,
                prompt: ChoicePrompt::Generic,
            })
        }
        Effect::AllGraveyardCreaturesToBattlefield => {
            for seat in 0..state.players.len() {
                let p = PlayerId::new(seat as u8);
                for &card in &state.zones.list(ZoneLocation::Graveyard(p)).clone() {
                    let is_creature = state.object(card).is_some_and(|o| {
                        o.characteristics()
                            .types
                            .contains(baylee_core::types::TypeSet::CREATURE)
                    });
                    if is_creature {
                        let _ = state.move_object(
                            card,
                            ZoneLocation::Battlefield,
                            ZonePosition::Top,
                            Cause::Effect,
                        );
                        if let Some(obj) = state.object_mut(card) {
                            obj.set_controller(you);
                        }
                    }
                }
            }
            None
        }
        Effect::ExileSelfReturnAsFace { face } => {
            let owner = state.object(res.source).map_or(you, |o| o.owner);
            if let Some(obj) = state.object_mut(res.source) {
                obj.kind = ObjectKind::Card;
            }
            let _ = state.move_object(
                res.source,
                ZoneLocation::Exile(owner),
                ZonePosition::Top,
                Cause::Effect,
            );
            let _ = state.move_object(
                res.source,
                ZoneLocation::Battlefield,
                ZonePosition::Top,
                Cause::Effect,
            );
            if let Some(obj) = state.object_mut(res.source) {
                obj.kind = ObjectKind::Permanent;
                obj.set_controller(owner);
                // The face switch needs the card definition (lookup);
                // finish_resolution applies it.
                obj.pending_face_change = Some(face);
            }
            None
        }
        Effect::ReturnLinkedToBattlefield => {
            // Everything exiled with a link to the source returns under its
            // owner's control (Skyclave Apparition & co.).
            let mut returning = Vec::new();
            for seat in 0..state.players.len() {
                let p = PlayerId::new(seat as u8);
                for &card in state.zones.list(ZoneLocation::Exile(p)) {
                    if state.object(card).is_some_and(|o| {
                        o.riders
                            .iter()
                            .any(|r| matches!(r, crate::object::Rider::Linked { host } if *host == res.source))
                    }) {
                        returning.push(card);
                    }
                }
            }
            for card in returning {
                if let Some(obj) = state.object_mut(card) {
                    obj.kind = ObjectKind::Permanent;
                    obj.riders
                        .retain(|r| !matches!(r, crate::object::Rider::Linked { host } if *host == res.source));
                }
                let _ = state.move_object(
                    card,
                    ZoneLocation::Battlefield,
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            None
        }
        Effect::ExileTargetsCreateTokens { token } => {
            let targets = res.targets.clone();
            for target_id in targets {
                let Some(obj) = state.object(target_id) else {
                    continue;
                };
                let owner = obj.owner;
                let controller = obj.controller;
                if let Some(obj) = state.object_mut(target_id) {
                    obj.kind = ObjectKind::Card;
                }
                let _ = state.move_object(
                    target_id,
                    ZoneLocation::Exile(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
                // Token replacement applies per token created (CR 614.1
                // applies to the total, per controller).
                tokens::create_one_token(state, controller, token);
            }
            None
        }
        Effect::CounterTargetAbility => {
            if let Some(&target_id) = res.targets.first() {
                state
                    .journal
                    .record(GameEvent::SpellCountered { object: target_id });
                // Abilities on the stack cease to exist when countered.
                state.zones.remove(target_id, ZoneLocation::Stack);
                let _ = state.arena.remove(target_id);
            }
            None
        }
        Effect::CounterTargetSpellOrAbility => {
            if let Some(&target_id) = res.targets.first()
                && !state
                    .object(target_id)
                    .is_some_and(|o| o.riders.contains(&crate::object::Rider::Uncounterable))
            {
                let kind = state.object(target_id).map(|o| o.kind);
                if kind == Some(ObjectKind::AbilityOnStack) {
                    state
                        .journal
                        .record(GameEvent::SpellCountered { object: target_id });
                    state.zones.remove(target_id, ZoneLocation::Stack);
                    let _ = state.arena.remove(target_id);
                } else {
                    state
                        .journal
                        .record(GameEvent::SpellCountered { object: target_id });
                    let owner = state.object(target_id).map_or(you, |o| o.owner);
                    if let Some(obj) = state.object_mut(target_id) {
                        obj.kind = ObjectKind::Card;
                    }
                    let _ = state.move_object(
                        target_id,
                        ZoneLocation::Graveyard(owner),
                        ZonePosition::Top,
                        Cause::Effect,
                    );
                }
            }
            None
        }
        Effect::CounterTargetSpellToExile => {
            if let Some(&target_id) = res.targets.first()
                && !state
                    .object(target_id)
                    .is_some_and(|o| o.riders.contains(&crate::object::Rider::Uncounterable))
            {
                state
                    .journal
                    .record(GameEvent::SpellCountered { object: target_id });
                let owner = state.object(target_id).map_or(you, |o| o.owner);
                if let Some(obj) = state.object_mut(target_id) {
                    obj.kind = ObjectKind::Card;
                }
                let _ = state.move_object(
                    target_id,
                    ZoneLocation::Exile(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            None
        }
        Effect::CounterTargetSpell => {
            if let Some(&target_id) = res.targets.first()
                && !state
                    .object(target_id)
                    .is_some_and(|o| o.riders.contains(&crate::object::Rider::Uncounterable))
            {
                state
                    .journal
                    .record(GameEvent::SpellCountered { object: target_id });
                let owner = state.object(target_id).map_or(you, |o| o.owner);
                if let Some(obj) = state.object_mut(target_id) {
                    obj.kind = ObjectKind::Card;
                }
                let _ = state.move_object(
                    target_id,
                    ZoneLocation::Graveyard(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            None
        }
        _ => unreachable!("not a zone effect"),
    }
}
