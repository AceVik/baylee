//! Life totals and damage: gain/lose life, damage to players, objects,
//! and planeswalkers (loyalty removal).

#[allow(clippy::wildcard_imports)] // family modules share the resolve vocabulary
use super::*;
use crate::prevention::{Redirected, Shield, ShieldKind, Shielded, redirect};
use baylee_cards_dsl::Filter;
use baylee_core::types::TypeSet;

/// Executes one life/damage effect.
#[allow(clippy::too_many_lines)] // the family is one flat table
pub(super) fn exec(state: &mut GameState, res: &mut Resolution, op: Effect) -> Option<Pending> {
    let you = res.controller;
    match op {
        Effect::DealDamageEvenly { amount, target } => {
            let targets = recipients(state, res, you, target);
            let count = u32::try_from(targets.len()).unwrap_or(u32::MAX);
            if let Some(share) = amount2(&amount, state, you, res).checked_div(count) {
                let version = source_version(state, res);
                for recipient in targets {
                    deal_redirected(
                        state,
                        res.source,
                        recipient,
                        share,
                        &mut Redirected::default(),
                        version,
                    );
                }
            }
            None
        }
        Effect::DealDamageWithCappedLifeGain { amount } => {
            let recipient = *recipients(state, res, you, TargetSpec::AnyTarget).first()?;
            let before_damage_cap = match recipient {
                DamageTarget::Player(player) => {
                    state.players[usize::from(player.get())].life.max(0)
                }
                DamageTarget::Object(id) => state.object(id).map_or(0, |o| {
                    let c = o.characteristics();
                    if c.types.contains(TypeSet::PLANESWALKER) {
                        i32::from(o.counters.get(baylee_cards_dsl::CounterKind::Loyalty))
                    } else {
                        i32::MAX
                    }
                }),
            };
            let start = state.journal.len();
            let n = amount2(&amount, state, you, res);
            deal_to_spec(state, res, you, res.source, n, TargetSpec::AnyTarget);
            // Only life and loyalty are explicitly measured before damage.
            // The subsequent gain reads current toughness (CR 608.2c, h),
            // including counters changed by prevention or damage. Layers
            // update immediately (CR 613.5), before the next SBA check.
            state.refresh_characteristics();
            let cap = match recipient {
                DamageTarget::Object(_) => target_chars(res, state)
                    .filter(|c| c.types.contains(TypeSet::CREATURE))
                    .map_or(before_damage_cap, |c| {
                        before_damage_cap.min(i32::from(c.toughness.unwrap_or(0)).max(0))
                    }),
                DamageTarget::Player(_) => before_damage_cap,
            };
            let dealt: i32 = state.journal.entries()[start..]
                .iter()
                .filter_map(|entry| match entry.event {
                    GameEvent::DamageDealt { source, amount, .. } if source == Some(res.source) => {
                        Some(i32::try_from(amount).unwrap_or(i32::MAX))
                    }
                    _ => None,
                })
                .fold(0, i32::saturating_add);
            gain_life(state, you, dealt.min(cap));
            None
        }
        Effect::GainLife { amount } => {
            let n = amount2(&amount, state, you, res) as i32;
            gain_life(state, you, n);
            None
        }
        Effect::GainLifeFor { amount, who } => {
            let n = amount2(&amount, state, you, res) as i32;
            let players = super::players_of(who, state, you, res);
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
            let n = amount2(&amount, state, you, res) as i32;
            // "Can't lose life" is the door's to answer, for this loss as
            // for damage.
            for player in super::players_of(target, state, you, res) {
                state.change_life(player, -n, Cause::Effect);
            }
            None
        }
        Effect::DealDamage { amount, target } => {
            let n = amount2(&amount, state, you, res);
            deal_to_spec(state, res, you, res.source, n, target);
            None
        }
        Effect::DealDamageToAttached { amount } => {
            let version = source_version(state, res);
            let attachment = source_attachment_lki(state, res.on_stack);
            let host =
                crate::eval::attached_for_ability(state, res.source, version, attachment)?.id;
            let n = amount2(&amount, state, you, res);
            deal_to_object(
                state,
                host,
                n,
                res.source,
                &mut Redirected::default(),
                version,
            );
            None
        }
        Effect::DealDamageDivided { .. } => {
            // As divided when the ability went on the stack (CR 601.2d).
            // `res.targets` holds only the targets still legal, and one that
            // is not is dealt nothing: its share goes to nobody (CR 608.2b).
            let shares = state
                .divided
                .iter()
                .find(|(id, _)| *id == res.on_stack)
                .map(|(_, shares)| shares.clone())
                .unwrap_or_default();
            for &target in &res.targets.clone() {
                if let Some(&(_, n)) = shares.iter().find(|(t, _)| *t == target) {
                    let version = source_version(state, res);
                    deal_to_object(
                        state,
                        target,
                        n,
                        res.source,
                        &mut Redirected::default(),
                        version,
                    );
                }
            }
            None
        }
        Effect::Fight { fighter, foe } => {
            fight(state, res, fighter, foe);
            None
        }
        Effect::DamageEqualToPower { dealer, to } => {
            damage_equal_to_power(state, res, dealer, to);
            None
        }
        Effect::EventObjectDealsDamageEqualToPower { target } => {
            // "That creature": the object the event named. Its power now if
            // it is still on the battlefield as the same object, else as it
            // last existed there (CR 608.2h); with neither, the effect
            // fails to determine an amount and deals nothing.
            let dealer = res.event_object?;
            let identity = event_object_identity(state, res);
            let power = state
                .object(dealer)
                .filter(|o| {
                    o.zone == crate::zone::Zone::Battlefield
                        && identity.is_none_or(|(version, _)| o.version == version)
                })
                .map(|o| o.characteristics().power.unwrap_or(0))
                .or_else(|| identity.map(|(_, power)| power))
                .or_else(|| {
                    state
                        .ltb_powers
                        .iter()
                        .find(|(id, _)| *id == dealer)
                        .map(|(_, p)| *p)
                });
            if let Some(n) = power.map(|p| p.max(0)) {
                deal_to_spec(
                    state,
                    res,
                    you,
                    dealer,
                    u32::try_from(n).unwrap_or(0),
                    target,
                );
            }
            None
        }
        Effect::DealDamageToTargetController { amount } => {
            if let Some(&target_id) = res.targets.first() {
                let controller = state.object(target_id).map_or(you, |o| o.controller);
                let n = amount2(&amount, state, you, res);
                let version = source_version(state, res);
                deal_to_player_after(
                    state,
                    res.source,
                    controller,
                    n,
                    &mut Redirected::default(),
                    version,
                );
            }
            None
        }
        Effect::DealDamageEach { amount, filter } => {
            damage_each(state, res, &amount, filter);
            None
        }
        Effect::PreventNextDamage { target, amount } => {
            let n = amount2(&amount, state, you, res);
            if n == 0 {
                return None;
            }
            for recipient in recipients(state, res, you, target) {
                let protects = match recipient {
                    DamageTarget::Player(player) => Shielded::Player(player),
                    DamageTarget::Object(id) => match state.object(id) {
                        Some(obj) => Shielded::Object(id, obj.version),
                        None => continue,
                    },
                };
                state.shields.push(Shield {
                    protects,
                    kind: ShieldKind::Next(n),
                    controller: you,
                });
            }
            None
        }
        Effect::PreventAllCombatDamageThisTurn => {
            state.shields.push(Shield {
                protects: Shielded::Everything,
                kind: ShieldKind::AllCombat,
                controller: you,
            });
            None
        }
        // The source is chosen as this resolves (CR 609.7a), and the choice
        // is an instruction, so `min: 1`; with no source to choose there is
        // no shield and no question (CR 609.3).
        Effect::PreventNextFromChosenSource {
            sources,
            combat_only,
            all_but,
            gain_life,
        } => {
            let options = crate::prevention::source_options(state, sources, you, res.source);
            if options.is_empty() {
                return None;
            }
            res.awaiting = Some(AwaitingOp::ShieldFromChosenSource {
                sources,
                combat_only,
                all_but,
                gain_life,
            });
            Some(Pending::ChooseCards {
                player: you,
                options,
                min: 1,
                max: 1,
                prompt: ChoicePrompt::Generic,
                total: None,
            })
        }
        // Jade Monolith: the creature is the target (still legal, or this
        // would not be resolving), and the source is chosen now, as the
        // prevention sibling's is (CR 609.7a) — any source at all.
        Effect::RedirectNextFromChosenSource { target } => {
            let DamageTarget::Object(id) = *recipients(state, res, you, target).first()? else {
                return None;
            };
            let version = state.object(id)?.version;
            let options = crate::prevention::source_options(state, &Filter::Any, you, res.source);
            if options.is_empty() {
                return None;
            }
            res.awaiting = Some(AwaitingOp::RedirectFromChosenSource {
                protects: Shielded::Object(id, version),
            });
            Some(Pending::ChooseCards {
                player: you,
                options,
                min: 1,
                max: 1,
                prompt: ChoicePrompt::Generic,
                total: None,
            })
        }
        _ => unreachable!("not a life/damage effect"),
    }
}

/// Deals `n` damage from `source` to what `target` names — the object or
/// player it chose, every player it names, or both halves of "any target".
///
/// The recipient half of [`Effect::DealDamage`], shared with the effects
/// whose damage comes from something other than the resolving ability's
/// own source (Pyrogoyf's "that creature deals damage").
fn deal_to_spec(
    state: &mut GameState,
    res: &Resolution,
    you: PlayerId,
    source: ObjectId,
    n: u32,
    target: TargetSpec,
) {
    let version = if Some(source) == res.event_object {
        event_object_identity(state, res).map(|(version, _)| version)
    } else if source == res.source {
        source_version(state, res)
    } else {
        None
    };
    for recipient in recipients(state, res, you, target) {
        deal_redirected(
            state,
            source,
            recipient,
            n,
            &mut Redirected::default(),
            version,
        );
    }
}

/// Apply paid prevention through the ordinary damage pipeline, and discard
/// unused prevention immediately: it belongs to this event, not the turn.
pub(super) fn damage_with_payment(
    state: &mut GameState,
    res: &Resolution,
    player: PlayerId,
    damage: u32,
    paid: u32,
) {
    state.shields.push(Shield {
        protects: Shielded::Everything,
        kind: ShieldKind::ThisEvent(paid),
        controller: player,
    });
    deal_redirected(
        state,
        res.source,
        DamageTarget::Player(player),
        damage,
        &mut Redirected::default(),
        source_version(state, res),
    );
    state
        .shields
        .retain(|shield| !matches!(shield.kind, ShieldKind::ThisEvent(_)));
}

/// Whom `target` names as this resolves: the objects and players an
/// effect that deals damage deals it to, and an effect that prevents
/// damage shields, in the order they are dealt to.
fn recipients(
    state: &GameState,
    res: &Resolution,
    you: PlayerId,
    target: TargetSpec,
) -> Vec<DamageTarget> {
    let players = |list: baylee_core::ids::SeatSet| list.iter().map(DamageTarget::Player);
    match target {
        TargetSpec::Player(rel) => super::players_of(rel, state, you, res)
            .into_iter()
            .map(DamageTarget::Player)
            .collect(),
        // "Any target" (CR 115.4) chose from one set spanning both,
        // so both halves are dealt to — a spell with two any-targets
        // can have picked a creature and a face.
        TargetSpec::AnyTarget => res
            .targets
            .iter()
            .copied()
            .map(DamageTarget::Object)
            .chain(players(res.target_players))
            .collect(),
        // "Target opponent or planeswalker": one choice over both lists, so
        // whichever half it landed in is dealt to.
        TargetSpec::OpponentOrObject(_) => res
            .targets
            .first()
            .copied()
            .map(DamageTarget::Object)
            .into_iter()
            .chain(players(res.target_players))
            .collect(),
        // Only ever a second instance of "target": the damage goes to what
        // that instance chose, if it chose anything ("up to one").
        TargetSpec::ObjectOfFirstTargetsPlayer(_) => res
            .second_targets
            .first()
            .copied()
            .map(DamageTarget::Object)
            .into_iter()
            .collect(),
        // A chosen player is a player. The choice landed in
        // `target_players`, so reading `targets` here would deal to
        // whatever object the spell also happened to point at — or,
        // far more often, to nothing at all. No card in the pool
        // says this yet; they all spell "target opponent" as
        // `Player(Chosen)` with the choice on the ability's
        // `TargetReq`, which is why the catch-all that used to be
        // here could hold this and stay green.
        TargetSpec::AnyPlayer | TargetSpec::AnyOpponent => players(res.target_players).collect(),
        // "This creature": the source, which nothing chose (#147, the rule
        // `zones::spec_object` keeps for the moving effects). Rock Hydra's
        // "prevent the next 1 damage that would be dealt to this creature"
        // read `targets`, found nothing, and shielded nobody.
        TargetSpec::ThisObject => vec![DamageTarget::Object(res.source)],
        // Everything else names an object, and the damage goes to
        // the one that was chosen. Spelled out rather than left to
        // a `_` arm: a new player-flavoured `TargetSpec` would land
        // in a catch-all silently and be dealt to as an object.
        TargetSpec::Object(_)
        | TargetSpec::ObjectOfEachOpponent(_)
        | TargetSpec::ObjectControlledBy(..)
        | TargetSpec::ObjectOfEventPlayer(_)
        | TargetSpec::Spell(_)
        | TargetSpec::StackOrBattlefield(_)
        | TargetSpec::CardInGraveyard(..)
        | TargetSpec::CardInGraveyardBelowEvent(..)
        | TargetSpec::CardInGraveyardBelowValue(..)
        | TargetSpec::AbilityOnStack(_)
        | TargetSpec::SpellOrAbility(_)
        | TargetSpec::EventObject => res
            .targets
            .first()
            .copied()
            .map(DamageTarget::Object)
            .into_iter()
            .collect(),
    }
}

/// `Effect::DealDamageEach` — "deals N damage to each <noun>".
fn damage_each(state: &mut GameState, res: &Resolution, amount: &Amount, filter: &Filter) {
    // The amount and the set are both read once, before anything is dealt
    // (CR 608.2h), and every recipient is dealt its share before the
    // state-based actions look (CR 704.3) — which is what makes the loop
    // simultaneous in effect (CR 608.2f).
    //
    // `battlefield_view` and not the raw zone list: a phased-out permanent is
    // treated as though it does not exist (CR 702.26b). `DestroyAll` walks the
    // raw list, which is #209.
    let you = res.controller;
    let n = amount2(amount, state, you, res);
    let can_be_dealt = TypeSet::CREATURE.union(TypeSet::PLANESWALKER);
    let hit: Vec<ObjectId> = state
        .battlefield_view()
        .into_iter()
        .filter(|id| {
            state.object(*id).is_some_and(|o| {
                o.characteristics().types.intersects(can_be_dealt)
                    && eval::matches(filter, state, o, you, res.source)
            })
        })
        .collect();
    let version = source_version(state, res);
    for id in hit {
        deal_to_object(
            state,
            id,
            n,
            res.source,
            &mut Redirected::default(),
            version,
        );
    }
}

/// `Effect::Fight` (CR 701.14a).
fn fight(state: &mut GameState, res: &Resolution, fighter: TargetSlot, foe: TargetSlot) {
    // CR 701.14b, both halves at once: a side that is gone — left the
    // battlefield, stopped being a creature, or was dropped by CR 608.2b's
    // re-check as an illegal target, which is what an empty slot means here —
    // and *neither* creature deals damage.
    let (Some(a), Some(b)) = (
        fighting(state, slot_object(res, fighter)),
        fighting(state, slot_object(res, foe)),
    ) else {
        return;
    };
    // Both amounts are read before either is dealt: the damage is dealt at
    // once (CR 701.14a, "each of those creatures deals damage"), and a
    // creature's power does not depend on the damage marked on it, so this is
    // the order that cannot matter — which is what makes it the right one to
    // write.
    let (power_a, power_b) = (power_of(state, a), power_of(state, b));
    // Each creature is the source of its own damage, which is what makes
    // deathtouch and protection read the right object (CR 702.2b,
    // CR 702.16e). A creature fighting itself runs both lines at itself —
    // twice its power, as CR 701.14c says.
    deal_to_object_with_loyalty(state, b, power_a, a);
    deal_to_object_with_loyalty(state, a, power_b, b);
}

/// `Effect::DamageEqualToPower` — "deals damage equal to its power to".
fn damage_equal_to_power(
    state: &mut GameState,
    res: &Resolution,
    dealer: TargetSlot,
    to: TargetSlot,
) {
    // Not a fight, so CR 701.14b does not govern it; CR 608.2b does, and it
    // lands in the same place. An illegal dealer is one whose power the
    // effect "fails to determine", so no damage happens, and an illegal
    // recipient is not affected by the part of the effect it is illegal for.
    // The recipient may be a planeswalker (Stump Stomp), which
    // `deal_to_object_with_loyalty` turns into loyalty (CR 306.8), so only the
    // dealer has to be a creature.
    let Some(from) = fighting(state, slot_object(res, dealer)) else {
        return;
    };
    let Some(target) = slot_object(res, to).filter(|id| {
        state
            .object(*id)
            .is_some_and(|o| o.zone == crate::zone::Zone::Battlefield)
    }) else {
        return;
    };
    let n = power_of(state, from);
    deal_to_object_with_loyalty(state, target, n, from);
}

/// The object a [`TargetSlot`] names as the effect resolves, if there still
/// is one.
///
/// Empty when the slot's target was dropped by CR 608.2b's re-check, and when
/// a "choose up to one" was answered with none — the two are the same fact to
/// every effect that reads it: there is nothing on that side.
fn slot_object(res: &Resolution, slot: TargetSlot) -> Option<ObjectId> {
    match slot {
        TargetSlot::This => Some(res.source),
        TargetSlot::First => res.targets.first().copied(),
        TargetSlot::Second => res.second_targets.first().copied(),
    }
}

/// `id`, if it is still a creature on the battlefield — CR 701.14b's two
/// conditions for a creature to fight at all.
fn fighting(state: &GameState, id: Option<ObjectId>) -> Option<ObjectId> {
    id.filter(|id| {
        state.object(*id).is_some_and(|o| {
            o.zone == crate::zone::Zone::Battlefield
                && o.characteristics()
                    .types
                    .contains(baylee_core::types::TypeSet::CREATURE)
        })
    })
}

/// "Damage equal to its power": the projected power, and nought for a
/// negative one (CR 107.1b — a negative number of damage is no damage).
fn power_of(state: &GameState, id: ObjectId) -> i16 {
    state
        .object(id)
        .and_then(|o| o.characteristics().power)
        .map_or(0, |p| p.max(0))
}

pub(super) fn gain_life(state: &mut GameState, player: PlayerId, n: i32) {
    if n <= 0 {
        return;
    }
    state.change_life(player, n, Cause::Effect);
}

pub(super) fn deal_to_object_with_loyalty(
    state: &mut GameState,
    target: ObjectId,
    n: i16,
    source: ObjectId,
) {
    deal_to_object(
        state,
        target,
        u32::try_from(n).unwrap_or(0),
        source,
        &mut Redirected::default(),
        None,
    );
}

/// Damage a redirection moved (CR 614.9), dealt through the door for its
/// new recipient with the redirections that already moved it (CR 614.5).
fn deal_redirected(
    state: &mut GameState,
    source: ObjectId,
    recipient: DamageTarget,
    n: u32,
    done: &mut Redirected,
    source_version: Option<u32>,
) {
    match recipient {
        DamageTarget::Player(player) => {
            deal_to_player_after(state, source, player, n, done, source_version);
        }
        DamageTarget::Object(id) => deal_to_object(state, id, n, source, done, source_version),
    }
}

fn deal_to_object(
    state: &mut GameState,
    target: ObjectId,
    n: u32,
    source: ObjectId,
    done: &mut Redirected,
    source_version: Option<u32>,
) {
    if n == 0 {
        return;
    }
    // Protection (CR 702.16e): matching sources deal no damage.
    if eval::protected_from(state, target, source) {
        return;
    }
    let n = crate::prevention::apply(state, source, DamageTarget::Object(target), n, false);
    if n == 0 {
        return;
    }
    if let Some(to) = redirect(state, source, DamageTarget::Object(target), done) {
        deal_redirected(state, source, to, n, done, source_version);
        return;
    }
    let n = crate::prevention::absorb(state, source, target, n, false);
    if n == 0 {
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
        let new = old.saturating_sub(u16::try_from(n).unwrap_or(u16::MAX));
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
            obj.damage = obj
                .damage
                .saturating_add(u16::try_from(n).unwrap_or(u16::MAX));
            obj.deathtouched |= deathtouch;
        }
    }
    state.record_permanent_damage(source, source_version, target, n, false);
}

#[cfg(test)]
pub(super) fn deal_to_player(state: &mut GameState, source: ObjectId, player: PlayerId, n: i16) {
    deal_to_player_after(
        state,
        source,
        player,
        u32::try_from(n).unwrap_or(0),
        &mut Redirected::default(),
        None,
    );
}

fn deal_to_player_after(
    state: &mut GameState,
    source: ObjectId,
    player: PlayerId,
    n: u32,
    done: &mut Redirected,
    source_version: Option<u32>,
) {
    if n == 0 {
        return;
    }
    let n = crate::prevention::apply(state, source, DamageTarget::Player(player), n, false);
    if n == 0 {
        return;
    }
    // After the shields and before the life, as combat's door does it.
    if let Some(to) = redirect(state, source, DamageTarget::Player(player), done) {
        deal_redirected(state, source, to, n, done, source_version);
        return;
    }
    state.damage_player(source, player, n, false, Cause::Effect);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::{ContinuousEffect, EffectFilter};
    use crate::object::ObjectKind;
    use crate::state::CardLookup;
    use crate::zone::ZoneLocation;
    use baylee_cards_dsl::{Duration, Filter, KeywordSet, Modifier};
    use baylee_core::ids::CardIndex;
    use baylee_core::preset::{
        AIProfile, DeckEntry, FormatId, GamePreset, HouseRules, PrintInfo, SeatCapabilities,
        SeatController, SeatSpec,
    };
    use baylee_core::types::TypeSet;

    struct RegistryLookup;
    impl CardLookup for RegistryLookup {
        fn card(&self, index: CardIndex) -> Option<&'static baylee_cards_dsl::CardDef> {
            baylee_cards::by_index(index)
        }
    }

    fn me() -> PlayerId {
        PlayerId::new(0)
    }

    fn state() -> GameState {
        let forest = baylee_cards::by_oracle_id("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
            .expect("registry contains Forest")
            .index;
        let deck: Vec<DeckEntry> = (0..60)
            .map(|_| DeckEntry {
                card: forest,
                print: baylee_core::ids::PrintRef::new(0),
            })
            .collect();
        let seat = || SeatSpec {
            controller: SeatController::Ai(AIProfile::default()),
            capabilities: SeatCapabilities::default(),
            deck: deck.clone(),
            sideboard: vec![],
            commanders: vec![],
            starting_life: None,
            starting_hand: None,
            starting_battlefield: vec![],
            emblems: vec![],
            team: None,
        };
        let preset = GamePreset {
            format: FormatId::Freeform,
            seed: 4,
            house_rules: HouseRules::default(),
            modifiers: vec![],
            prints: vec![PrintInfo {
                scryfall_id: uuid::Uuid::nil(),
                lang: "EN".into(),
                finish: baylee_core::preset::Finish::Normal,
            }],
            seats: vec![seat(), seat()],
        };
        GameState::from_preset(&preset, &RegistryLookup).expect("game starts")
    }

    fn permanent(state: &mut GameState, name: &str) -> ObjectId {
        let name = state.names.intern(name);
        state.create_bare(me(), ObjectKind::Permanent, name, ZoneLocation::Battlefield)
    }

    fn life(state: &GameState, player: PlayerId) -> i32 {
        state.players[player.get() as usize].life
    }

    /// "Whenever you gain life" reads the journal, so a gain of nothing has
    /// to leave nothing behind: a lifelink source dealing 0 damage, a
    /// `GainLife` with an [`Amount`] that counted an empty board, and every
    /// negative that arithmetic can hand this door would otherwise fire a
    /// trigger the player never earned — and the negative one would *take*
    /// life through the gain door.
    #[test]
    fn gaining_no_life_is_neither_a_change_nor_an_event() {
        let mut state = state();
        let start = life(&state, me());
        let entries = state.journal.len();

        gain_life(&mut state, me(), 0);
        gain_life(&mut state, me(), -3);

        assert_eq!(life(&state, me()), start, "no life moved");
        assert_eq!(state.journal.len(), entries, "and nothing was recorded");

        gain_life(&mut state, me(), 3);
        assert_eq!(life(&state, me()), start + 3);
        assert_eq!(state.journal.len(), entries + 1);
        assert!(
            matches!(
                state.journal.entries().last().expect("an entry").event,
                GameEvent::LifeChanged { player, old, new, .. }
                    if player == me() && old == start && new == start + 3
            ),
            "a life change carries both totals, because the triggers that \
             read it care by how much"
        );
    }

    /// Damage to a player is two events and not one: CR 120.3c reduces the
    /// life total, and the damage itself is what "whenever a source deals
    /// damage to a player" reads. A door recording only the life change
    /// would leave every damage trigger blind, and one recording only the
    /// damage would leave every life trigger blind.
    /// Both writers here ask the prevention shields (CR 615.7): damage an
    /// effect deals to a shielded player or permanent is prevented before
    /// anything is lost, marked or journalled, and what the shield does not
    /// cover is dealt. On the old writers the player lost all 3 and the
    /// creature was marked with all 3.
    #[test]
    fn an_effects_damage_meets_the_prevention_shields() {
        let mut state = state();
        let source = permanent(&mut state, "Shock");
        let creature = permanent(&mut state, "Grizzly Bears");
        state.shields.push(Shield {
            protects: Shielded::Player(me()),
            kind: ShieldKind::Next(2),
            controller: me(),
        });
        let version = state.object(creature).unwrap().version;
        state.shields.push(Shield {
            protects: Shielded::Object(creature, version),
            kind: ShieldKind::Next(1),
            controller: me(),
        });
        let start = life(&state, me());
        let entries = state.journal.len();

        deal_to_player(&mut state, source, me(), 3);
        assert_eq!(life(&state, me()), start - 1, "2 of the 3 prevented");
        assert!(matches!(
            state.journal.entries().last().expect("an entry").event,
            GameEvent::DamageDealt { amount: 1, .. }
        ));

        deal_to_object_with_loyalty(&mut state, creature, 3, source);
        assert_eq!(state.object(creature).unwrap().damage, 2);
        assert!(state.shields.is_empty(), "both used up");

        // Damage a shield prevents in full is never dealt at all.
        state.shields.push(Shield {
            protects: Shielded::Player(me()),
            kind: ShieldKind::Next(5),
            controller: me(),
        });
        let entries_now = state.journal.len();
        deal_to_player(&mut state, source, me(), 3);
        assert_eq!(life(&state, me()), start - 1);
        assert_eq!(state.journal.len(), entries_now, "no event, no life change");
        assert!(entries_now > entries);
    }

    #[test]
    fn damage_to_a_player_is_a_life_change_and_a_damage_event() {
        let mut state = state();
        let source = permanent(&mut state, "Shock");
        let start = life(&state, me());
        let entries = state.journal.len();

        deal_to_player(&mut state, source, me(), 3);

        assert_eq!(life(&state, me()), start - 3);
        assert_eq!(state.journal.len(), entries + 2);
        let recorded = &state.journal.entries()[entries..];
        assert!(matches!(
            recorded[0].event,
            GameEvent::LifeChanged { player, cause: Cause::Effect, .. } if player == me()
        ));
        assert!(
            matches!(
                recorded[1].event,
                GameEvent::DamageDealt {
                    source: Some(src),
                    target: DamageTarget::Player(player),
                    amount: 3,
                    is_combat: false,
                } if src == source && player == me()
            ),
            "the damage names its source: protection, prevention and \
             lifelink are all properties of the source rather than of the \
             number"
        );

        let entries = state.journal.len();
        deal_to_player(&mut state, source, me(), 0);
        deal_to_player(&mut state, source, me(), -2);
        assert_eq!(life(&state, me()), start - 3, "no damage moves no life");
        assert_eq!(state.journal.len(), entries, "and records no event");
    }

    /// Damage to a player who can't lose life (Everybody Lives!) is still
    /// damage dealt, so "whenever a source deals damage" and lifelink both
    /// have something to read. The loss is what doesn't happen: no life
    /// moves, no `LifeChanged` is recorded, and the turn has no loss in it
    /// (#244).
    ///
    /// The effect belongs to the other seat, because "players" is the whole
    /// table and the check used to be keyed on the wrong player.
    #[test]
    fn damage_to_a_player_who_cant_lose_life_is_dealt_and_costs_nothing() {
        let mut state = state();
        let source = permanent(&mut state, "Shock");
        let modifier = Modifier::CantLoseLife {
            who: baylee_cards_dsl::PlayerRel::EachPlayer,
        };
        state.effects.register(ContinuousEffect {
            // `register` assigns the real one.
            id: baylee_core::ids::EffectId::new(0),
            source: None,
            controller: PlayerId::new(1),
            origin: crate::effects::EffectOrigin::Resolution,
            layer: modifier.layer(),
            timestamp: 1,
            duration: Duration::UntilEndOfTurn,
            filter: EffectFilter::Dsl(&Filter::Any),
            modifier,
        });
        let start = life(&state, me());
        let entries = state.journal.len();

        deal_to_player(&mut state, source, me(), 3);

        assert_eq!(life(&state, me()), start, "no life moved");
        let recorded = &state.journal.entries()[entries..];
        assert_eq!(recorded.len(), 1, "one event: {recorded:?}");
        assert!(matches!(
            recorded[0].event,
            GameEvent::DamageDealt { amount: 3, .. }
        ));
        assert!(!state.per_turn.life_lost[me().get() as usize]);
        assert!(
            !state.can_pay_life(me(), 1) && state.can_pay_life(me(), 0),
            "nor can life be paid, except none at all (CR 119.8, CR 119.4b)"
        );
    }

    /// "The damage dealt to you this turn" (Simulacrum) is damage and only
    /// damage: what reached the player past the shields (CR 615.1), whether
    /// or not it cost life, and not a payment of life or a gain after it.
    /// The turn's reset starts it again. On the old doors nothing counted it,
    /// and `Amount::DamageDealtToYouThisTurn` reads what this door counts.
    #[test]
    fn the_damage_dealt_to_a_player_this_turn_is_counted_where_it_is_dealt() {
        let mut state = state();
        let source = permanent(&mut state, "Shock");
        let seat = me().get() as usize;
        let dealt = |state: &GameState| {
            crate::eval::amount(
                &baylee_cards_dsl::Amount::DamageDealtToYouThisTurn,
                state,
                me(),
                source,
                None,
            )
        };

        deal_to_player(&mut state, source, me(), 3);
        assert_eq!(state.per_turn.damage_dealt_to[seat], 3);
        assert_eq!(dealt(&state), 3, "the amount reads the tally");
        gain_life(&mut state, me(), 1);
        state.change_life(me(), -2, Cause::Cost);
        assert_eq!(dealt(&state), 3, "a gain and a payment are no damage");

        state.shields.push(Shield {
            protects: Shielded::Player(me()),
            kind: ShieldKind::Next(1),
            controller: me(),
        });
        deal_to_player(&mut state, source, me(), 2);
        assert_eq!(dealt(&state), 4, "what the shield let through");
        state.shields.push(Shield {
            protects: Shielded::Player(me()),
            kind: ShieldKind::Next(5),
            controller: me(),
        });
        deal_to_player(&mut state, source, me(), 2);
        assert_eq!(dealt(&state), 4, "prevented in full, never dealt");
        state.shields.clear(); // three of the five are left

        let modifier = Modifier::CantLoseLife {
            who: baylee_cards_dsl::PlayerRel::EachPlayer,
        };
        state.effects.register(ContinuousEffect {
            id: baylee_core::ids::EffectId::new(0),
            source: None,
            controller: me(),
            origin: crate::effects::EffectOrigin::Resolution,
            layer: modifier.layer(),
            timestamp: 1,
            duration: Duration::UntilEndOfTurn,
            filter: EffectFilter::Dsl(&Filter::Any),
            modifier,
        });
        let before = life(&state, me());
        deal_to_player(&mut state, source, me(), 2);
        assert_eq!(life(&state, me()), before, "no life moved");
        assert_eq!(dealt(&state), 6, "and the damage was still dealt");

        assert_eq!(
            state.per_turn.damage_dealt_to[1], 0,
            "the other seat's is its own"
        );
        state.per_turn.reset();
        assert_eq!(dealt(&state), 0, "a new turn counts from nothing");
    }

    /// A point of damage and a point of life gained back leave the total
    /// where it was and the turn different: "if you didn't lose life this
    /// turn" (Luminarch Ascension) is now false, because damage is life lost
    /// (CR 119.2) and gaining it back does not unlose it.
    ///
    /// The two states have the same total on purpose. A pair whose totals
    /// differed would hash apart through `life` alone and prove nothing
    /// about the history. With the totals equal, the fact has to be held
    /// somewhere `snapshot_hash` reads (#241). It used to be only in the
    /// journal, which the hash does not read.
    #[test]
    fn a_life_lost_and_gained_back_is_still_a_life_lost_this_turn() {
        let mut untouched = state();
        let mut touched = state();
        permanent(&mut untouched, "Shock");
        let source = permanent(&mut touched, "Shock");

        deal_to_player(&mut touched, source, me(), 1);
        gain_life(&mut touched, me(), 1);

        assert_eq!(life(&touched, me()), life(&untouched, me()));
        let seat = me().get() as usize;
        assert!(
            touched.per_turn.life_lost[seat],
            "the damage is a loss of life"
        );
        assert!(!untouched.per_turn.life_lost[seat]);
        assert_ne!(
            touched.snapshot_hash(),
            untouched.snapshot_hash(),
            "the two states answer the Ascension differently, so they are \
             not the same state"
        );
    }

    /// CR 306.8: damage to a planeswalker removes that many loyalty
    /// counters. It is not marked on the permanent — a planeswalker has no
    /// toughness for it to be measured against, and a walker that took
    /// damage *and* kept its loyalty would die to neither rule.
    #[test]
    fn damage_to_a_planeswalker_removes_loyalty_instead_of_marking_damage() {
        let mut state = state();
        let source = permanent(&mut state, "Bolt");
        let walker = permanent(&mut state, "Walker");
        {
            let obj = state.object_mut(walker).expect("just made it");
            obj.base_mut().types = TypeSet::PLANESWALKER;
            obj.counters.set(baylee_cards_dsl::CounterKind::Loyalty, 4);
        }

        deal_to_object_with_loyalty(&mut state, walker, 3, source);
        let obj = state.object(walker).expect("still there");
        assert_eq!(obj.counters.get(baylee_cards_dsl::CounterKind::Loyalty), 1);
        assert_eq!(obj.damage, 0, "and no damage is marked on it");

        // More than it has takes it to nought rather than wrapping: the
        // subtraction is on a u16, and 1 - 5 there is 65532.
        deal_to_object_with_loyalty(&mut state, walker, 5, source);
        assert_eq!(
            state
                .object(walker)
                .expect("still there")
                .counters
                .get(baylee_cards_dsl::CounterKind::Loyalty),
            0
        );
    }

    /// CR 702.2b: deathtouch is a property of the **source** and applies to
    /// any damage it deals, not only combat damage. So the flag is read off
    /// the source at the moment the damage lands and remembered on the
    /// creature that took it — the state-based action that destroys it
    /// (CR 704.5h) runs later and has no way back to the source.
    #[test]
    fn deathtouch_is_read_off_the_source_and_owes_nothing_to_combat() {
        let mut state = state();
        let plain = permanent(&mut state, "Plain Source");
        let deadly = permanent(&mut state, "Deadly Source");
        state
            .object_mut(deadly)
            .expect("just made it")
            .base_mut()
            .keywords = KeywordSet::DEATHTOUCH;
        let bear = permanent(&mut state, "Bear");

        deal_to_object_with_loyalty(&mut state, bear, 1, plain);
        let obj = state.object(bear).expect("still there");
        assert_eq!(obj.damage, 1);
        assert!(
            !obj.deathtouched,
            "an ordinary source marks ordinary damage"
        );

        deal_to_object_with_loyalty(&mut state, bear, 1, deadly);
        let obj = state.object(bear).expect("still there");
        assert_eq!(obj.damage, 2, "damage accumulates until cleanup");
        assert!(obj.deathtouched);
        assert!(
            matches!(
                state.journal.entries().last().expect("an entry").event,
                GameEvent::DamageDealt {
                    target: DamageTarget::Object(target),
                    is_combat: false,
                    ..
                } if target == bear
            ),
            "this door is never combat damage — combat has its own"
        );
    }

    /// CR 702.16e: a matching source deals no damage at all. Not zero
    /// damage — *no* damage, so the event is not recorded either and
    /// "whenever this creature is dealt damage" never fires.
    #[test]
    fn protection_stops_the_damage_and_the_event_with_it() {
        let mut state = state();
        let source = permanent(&mut state, "Bolt");
        let bear = permanent(&mut state, "Bear");
        let version = state.object(bear).expect("just made it").version;
        let modifier = Modifier::ProtectionFrom(&Filter::ControlledByYou);
        state.effects.register(ContinuousEffect {
            // `register` assigns the real one.
            id: baylee_core::ids::EffectId::new(0),
            source: Some(bear),
            controller: me(),
            origin: crate::effects::EffectOrigin::Resolution,
            layer: modifier.layer(),
            timestamp: 1,
            duration: Duration::WhileSourceOnBattlefield,
            filter: EffectFilter::ObjectIs(bear, version),
            modifier,
        });
        let entries = state.journal.len();

        deal_to_object_with_loyalty(&mut state, bear, 3, source);

        let obj = state.object(bear).expect("still there");
        assert_eq!(obj.damage, 0);
        assert!(!obj.deathtouched);
        assert_eq!(
            state.journal.len(),
            entries,
            "no damage was dealt, so nothing happened that a trigger could see"
        );

        // The other branch of the same call, so a green run cannot mean the
        // damage failed to land for some reason of its own: an identical
        // creature without the effect takes it.
        let unprotected = permanent(&mut state, "Other Bear");
        deal_to_object_with_loyalty(&mut state, unprotected, 3, source);
        assert_eq!(state.object(unprotected).expect("still there").damage, 3);
        assert_eq!(state.journal.len(), entries + 1);
    }

    /// Maze of Ith's two modifiers prevent combat damage only ("Prevent
    /// all combat damage that would be dealt to and dealt by that
    /// creature"), as Kor Haven's, the other card that carries one, does.
    /// Combat's doors ask them; an effect's damage is dealt.
    #[test]
    fn maze_of_ith_s_modifiers_leave_an_effect_s_damage_alone() {
        let mut state = state();
        let source = permanent(&mut state, "Bolt");
        let bear = permanent(&mut state, "Bear");
        for (on, modifier) in [
            (bear, Modifier::PreventDamageToIt),
            (source, Modifier::PreventDamageFromIt),
        ] {
            let version = state.object(on).expect("just made it").version;
            state.effects.register(ContinuousEffect {
                // `register` assigns the real one.
                id: baylee_core::ids::EffectId::new(0),
                source: Some(on),
                controller: me(),
                origin: crate::effects::EffectOrigin::Resolution,
                layer: modifier.layer(),
                timestamp: 1,
                duration: Duration::UntilEndOfTurn,
                filter: EffectFilter::ObjectIs(on, version),
                modifier,
            });
        }

        deal_to_object_with_loyalty(&mut state, bear, 3, source);
        assert_eq!(
            state.object(bear).expect("still there").damage,
            3,
            "neither modifier stops an effect's damage to the creature"
        );
        let before = life(&state, me());
        deal_to_player(&mut state, source, me(), 2);
        assert_eq!(
            life(&state, me()),
            before - 2,
            "nor the creature's damage to a player"
        );
    }

    /// A permanent of `types` on `seat`'s side, and nothing else about it.
    fn typed(state: &mut GameState, seat: PlayerId, name: &str, types: TypeSet) -> ObjectId {
        let name = state.names.intern(name);
        let id = state.create_bare(seat, ObjectKind::Permanent, name, ZoneLocation::Battlefield);
        state.object_mut(id).expect("just made it").base_mut().types = types;
        id
    }

    /// A resolution of `source`, controlled by seat 0, targeting nothing.
    fn untargeted(source: ObjectId) -> Resolution {
        Resolution {
            source,
            on_stack: source,
            controller: me(),
            effects: vec![],
            pc: 0,
            targets: SmallVec::new(),
            second_targets: SmallVec::new(),
            x: None,
            chosen_player: None,
            target_lki: None,
            event_mana: None,
            retarget_left: None,
            target_players: baylee_core::ids::SeatSet::new(),
            event_object: None,
            awaiting: None,
            targeted: false,
            mana_ability: false,
            countered_source: None,
        }
    }

    #[test]
    fn capped_gain_reprojects_toughness_after_damage_prevention_changes_counters() {
        use baylee_cards_dsl::CounterKind;

        let mut state = state();
        let source = permanent(&mut state, "Capped damage source");
        let target = typed(&mut state, me(), "Counter body", TypeSet::CREATURE);
        let obj = state.object_mut(target).unwrap();
        obj.base_mut().power = Some(0);
        obj.base_mut().toughness = Some(0);
        obj.counters.set(CounterKind::P1P1, 3);
        let modifier = Modifier::CountersPreventDamage(CounterKind::P1P1);
        state.effects.register(ContinuousEffect {
            id: baylee_core::ids::EffectId::new(0),
            source: Some(target),
            controller: me(),
            origin: crate::effects::EffectOrigin::Static,
            layer: modifier.layer(),
            timestamp: 1,
            duration: Duration::WhileSourceOnBattlefield,
            filter: EffectFilter::Dsl(&Filter::This),
            modifier,
        });
        state.refresh_characteristics();
        assert_eq!(
            state.object(target).unwrap().characteristics().toughness,
            Some(3)
        );
        let mut res = untargeted(source);
        res.targets.push(target);
        res.targeted = true;
        let before = life(&state, me());
        assert!(
            exec(
                &mut state,
                &mut res,
                Effect::DealDamageWithCappedLifeGain {
                    amount: Amount::Fixed(4),
                },
            )
            .is_none()
        );
        let body = state.object(target).unwrap();
        assert_eq!(body.damage, 1, "the unprevented damage was dealt");
        assert_eq!(body.characteristics().toughness, Some(0));
        assert_eq!(
            body.zone,
            crate::zone::Zone::Battlefield,
            "no mid-effect SBA"
        );
        assert_eq!(
            life(&state, me()),
            before,
            "current zero toughness caps gain"
        );
    }

    /// "~ deals 2 damage to each …" over a filter that matches everything:
    /// both sides' creatures are marked, the planeswalker loses loyalty, and
    /// the land, the artifact and the phased-out creature get nothing.
    ///
    /// The filter is `Any` on purpose. A card's filter names its printed
    /// noun, so no card in the pool would ever put a land in front of this
    /// resolver — which is exactly why the type skip needs a test of its own:
    /// `deal_to_object_with_loyalty` marks damage on anything that is not a
    /// walker (CR 120.1a says it may not), and a phased-out permanent is
    /// treated as though it does not exist (CR 702.26b).
    #[test]
    fn damage_to_each_reaches_what_can_be_dealt_damage_and_nothing_else() {
        let mut state = state();
        let them = PlayerId::new(1);
        let source = permanent(&mut state, "Sweep");
        let mine = typed(&mut state, me(), "Bear", TypeSet::CREATURE);
        let theirs = typed(&mut state, them, "Ogre", TypeSet::CREATURE);
        let walker = typed(&mut state, them, "Walker", TypeSet::PLANESWALKER);
        state
            .object_mut(walker)
            .expect("just made it")
            .counters
            .set(baylee_cards_dsl::CounterKind::Loyalty, 5);
        let land = typed(&mut state, them, "Land", TypeSet::LAND);
        let relic = typed(&mut state, them, "Relic", TypeSet::ARTIFACT);
        let gone = typed(&mut state, them, "Phased Bear", TypeSet::CREATURE);
        state
            .object_mut(gone)
            .expect("just made it")
            .status
            .insert(crate::object::Status::PHASED_OUT);
        let entries = state.journal.len();

        let mut res = untargeted(source);
        exec(
            &mut state,
            &mut res,
            Effect::DealDamageEach {
                amount: Amount::Fixed(2),
                filter: &Filter::Any,
            },
        );

        let damage = |id: ObjectId| state.object(id).expect("on the battlefield").damage;
        assert_eq!(damage(mine), 2, "the controller's own creature is dealt it");
        assert_eq!(damage(theirs), 2, "and so is the opponent's");
        let walked = state.object(walker).expect("on the battlefield");
        assert_eq!(
            walked.counters.get(baylee_cards_dsl::CounterKind::Loyalty),
            3,
            "a planeswalker loses loyalty (CR 120.3c)"
        );
        assert_eq!(walked.damage, 0, "and has no damage marked");
        assert_eq!(damage(land), 0, "a land is not dealt damage");
        assert_eq!(damage(relic), 0, "nor an artifact");
        assert_eq!(damage(gone), 0, "nor a phased-out creature");

        let dealt: Vec<ObjectId> = state.journal.entries()[entries..]
            .iter()
            .filter_map(|e| match e.event {
                GameEvent::DamageDealt {
                    source: Some(from),
                    target: DamageTarget::Object(to),
                    is_combat: false,
                    ..
                } if from == source => Some(to),
                _ => None,
            })
            .collect();
        assert_eq!(
            dealt,
            vec![mine, theirs, walker],
            "one damage event per recipient, and none for what was skipped"
        );
    }

    /// Among what can be dealt damage, the filter decides: "each creature"
    /// leaves a planeswalker alone, which is Surtland Frostpyre's sentence
    /// and not Dragonback Assault's.
    #[test]
    fn damage_to_each_creature_leaves_a_planeswalker_alone() {
        let mut state = state();
        let source = permanent(&mut state, "Sweep");
        let bear = typed(&mut state, me(), "Bear", TypeSet::CREATURE);
        let walker = typed(&mut state, me(), "Walker", TypeSet::PLANESWALKER);
        state
            .object_mut(walker)
            .expect("just made it")
            .counters
            .set(baylee_cards_dsl::CounterKind::Loyalty, 5);

        let mut res = untargeted(source);
        exec(
            &mut state,
            &mut res,
            Effect::damage_each(2, &Filter::CREATURE),
        );

        assert_eq!(state.object(bear).expect("there").damage, 2);
        assert_eq!(
            state
                .object(walker)
                .expect("there")
                .counters
                .get(baylee_cards_dsl::CounterKind::Loyalty),
            5
        );
    }
}
