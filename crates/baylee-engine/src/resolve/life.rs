//! Life totals and damage: gain/lose life, damage to players, objects,
//! and planeswalkers (loyalty removal).

#[allow(clippy::wildcard_imports)] // family modules share the resolve vocabulary
use super::*;

/// Executes one life/damage effect.
pub(super) fn exec(state: &mut GameState, res: &mut Resolution, op: Effect) -> Option<Pending> {
    let you = res.controller;
    match op {
        Effect::GainLife { amount } => {
            let n = amount2(&amount, state, you, res.source, res.x, &res.targets) as i32;
            gain_life(state, you, n);
            None
        }
        Effect::GainLifeFor { amount, who } => {
            let n = amount2(&amount, state, you, res.source, res.x, &res.targets) as i32;
            let players = match who {
                PlayerRel::ControllerOfTarget => res
                    .targets
                    .first()
                    .and_then(|t| state.object(*t))
                    .map_or_else(Vec::new, |o| vec![o.controller]),
                other => eval::players(other, state, you),
            };
            for player in players {
                gain_life(state, player, n);
            }
            None
        }
        Effect::GainLifeDoubleX => {
            let n = res.x.unwrap_or(0).saturating_mul(2) as i32;
            gain_life(state, you, n);
            None
        }
        Effect::LoseLife { amount, target } => {
            let n = amount2(&amount, state, you, res.source, res.x, &res.targets) as i32;
            for player in eval::players(target, state, you) {
                // Everybody Lives: the controller can't lose life this turn.
                let cant = state.effects.iter().any(|fx| {
                    matches!(fx.modifier, baylee_cards_dsl::Modifier::CantLoseLife)
                        && fx.controller == you
                });
                if cant {
                    continue;
                }
                let p = &mut state.players[player.get() as usize];
                let old = p.life;
                p.life -= n;
                let new = p.life;
                state.journal.record(GameEvent::LifeChanged {
                    player,
                    old,
                    new,
                    cause: Cause::Effect,
                });
            }
            None
        }
        Effect::DealDamage { amount, target } => {
            let n = amount2(&amount, state, you, res.source, res.x, &res.targets) as i16;
            match target {
                TargetSpec::Player(rel) => {
                    for player in eval::players(rel, state, you) {
                        deal_to_player(state, res.source, player, n);
                    }
                }
                _ => {
                    if let Some(&target_id) = res.targets.first() {
                        deal_to_object_with_loyalty(state, target_id, n, res.source);
                    }
                }
            }
            None
        }
        Effect::DealDamageToTargetController { amount } => {
            if let Some(&target_id) = res.targets.first() {
                let controller = state.object(target_id).map_or(you, |o| o.controller);
                let n = amount2(&amount, state, you, res.source, res.x, &res.targets) as i16;
                deal_to_player(state, res.source, controller, n);
            }
            None
        }
        _ => unreachable!("not a life/damage effect"),
    }
}

pub(super) fn gain_life(state: &mut GameState, player: PlayerId, n: i32) {
    if n <= 0 {
        return;
    }
    let p = &mut state.players[player.get() as usize];
    let old = p.life;
    p.life += n;
    let new = p.life;
    state.journal.record(GameEvent::LifeChanged {
        player,
        old,
        new,
        cause: Cause::Effect,
    });
}

pub(super) fn deal_to_object_with_loyalty(
    state: &mut GameState,
    target: ObjectId,
    n: i16,
    source: ObjectId,
) {
    if n <= 0 {
        return;
    }
    // Protection (CR 702.16b): matching sources deal no damage.
    if eval::protected_from(state, target, source) {
        return;
    }
    let is_walker = state.object(target).is_some_and(|o| {
        o.characteristics()
            .types
            .contains(baylee_core::types::TypeSet::PLANESWALKER)
    });
    if is_walker {
        // Damage to a planeswalker removes loyalty counters (CR 306.8).
        let old = state.object(target).map_or(0, |o| {
            o.counters.get(baylee_cards_dsl::CounterKind::Loyalty)
        });
        let new = old.saturating_sub(n as u16);
        if let Some(obj) = state.object_mut(target) {
            obj.counters
                .set(baylee_cards_dsl::CounterKind::Loyalty, new);
        }
        state.journal.record(GameEvent::CounterChanged {
            object: target,
            kind: baylee_cards_dsl::CounterKind::Loyalty,
            old,
            new,
        });
    } else {
        // CR 702.2b: deathtouch is a property of the *source*, and it
        // applies to any damage it deals, not just combat damage.
        let deathtouch = state.object(source).is_some_and(|o| {
            o.characteristics()
                .keywords
                .contains(baylee_cards_dsl::KeywordSet::DEATHTOUCH)
        });
        if let Some(obj) = state.object_mut(target) {
            obj.damage = obj.damage.saturating_add(n as u16);
            obj.deathtouched |= deathtouch;
        }
    }
    state.journal.record(GameEvent::DamageDealt {
        source: Some(source),
        target: DamageTarget::Object(target),
        amount: n as u16,
        is_combat: false,
    });
}

pub(super) fn deal_to_player(state: &mut GameState, source: ObjectId, player: PlayerId, n: i16) {
    if n <= 0 {
        return;
    }
    let p = &mut state.players[player.get() as usize];
    let old = p.life;
    p.life -= i32::from(n);
    let new = p.life;
    state.journal.record(GameEvent::LifeChanged {
        player,
        old,
        new,
        cause: Cause::Effect,
    });
    state.journal.record(GameEvent::DamageDealt {
        source: Some(source),
        target: DamageTarget::Player(player),
        amount: n as u16,
        is_combat: false,
    });
}
