//! Token creation: plain tokens, copies, per-count sizing, Amass, and
//! the shared token factory.

#[allow(clippy::wildcard_imports)] // family modules share the resolve vocabulary
use super::*;

/// Executes one token-creation effect.
#[allow(clippy::too_many_lines)] // the family is one flat table
pub(super) fn exec(state: &mut GameState, res: &mut Resolution, op: Effect) -> Option<Pending> {
    let you = res.controller;
    match op {
        Effect::CreateTokenForTargetController { token } => {
            if let Some(&target_id) = res.targets.first() {
                let controller = state.object(target_id).map_or(you, |o| o.controller);
                create_one_token(state, controller, token);
            }
            None
        }
        Effect::Amass { subtype, amount } => {
            // Find an Army you control; if none, create a 0/0 Army token.
            static ARMY: baylee_core::ids::SubtypeId = baylee_core::ids::SubtypeId::new(0);
            let _ = ARMY;
            let army = state
                .zones
                .list(ZoneLocation::Battlefield)
                .iter()
                .copied()
                .find(|id| {
                    state.object(*id).is_some_and(|o| {
                        o.controller == you
                            && o.characteristics()
                                .types
                                .contains(baylee_core::types::TypeSet::CREATURE)
                            && o.characteristics().subtypes.contains(subtype)
                    })
                });
            let target_id = if let Some(id) = army {
                id
            } else {
                // Create the 0/0 Army token, then put counters on it.
                let name = state.names.intern("Army");
                let base = Characteristics {
                    name,
                    mana_cost: ManaCost::ZERO,
                    colors: ColorSet::EMPTY,
                    types: baylee_core::types::TypeSet::CREATURE,
                    supertypes: baylee_core::types::SupertypeSet::EMPTY,
                    subtypes: SubtypeSet::from_slice(&[subtype]),
                    keywords: baylee_cards_dsl::KeywordSet::EMPTY,
                    power: Some(0),
                    toughness: Some(0),
                    loyalty: None,
                    color_identity: ColorSet::EMPTY,
                    produced_colors: ColorSet::EMPTY,
                    produced_colorless: false,
                };
                let ts = state.next_timestamp();
                let id = state.arena.insert_with(|id| {
                    let mut obj = GameObject::new_bare(id, you, ObjectKind::Permanent, base);
                    obj.timestamp = ts;
                    obj
                });
                state
                    .zones
                    .insert(id, ZoneLocation::Battlefield, ZonePosition::Top);
                if let Some(obj) = state.object_mut(id) {
                    obj.zone = crate::zone::Zone::Battlefield;
                }
                id
            };
            if let Some(obj) = state.object_mut(target_id) {
                let old = obj.counters.get(baylee_cards_dsl::CounterKind::P1P1);
                let new = obj
                    .counters
                    .add(baylee_cards_dsl::CounterKind::P1P1, amount);
                state.journal.record(GameEvent::CounterChanged {
                    object: target_id,
                    kind: baylee_cards_dsl::CounterKind::P1P1,
                    old,
                    new,
                });
            }
            None
        }
        Effect::CreateTokenCopyOf {
            target,
            kicked_bonus,
        } => {
            let target_id = match target {
                Some(_) => res.targets.first().copied(),
                None => Some(res.source),
            };
            let kicked = state.object(res.on_stack).is_some_and(|o| o.kicked);
            let count = 1 + if kicked { u32::from(kicked_bonus) } else { 0 };
            if let Some(id) = target_id
                && let Some(base) = state.object(id).map(|o| o.base.clone())
            {
                for _ in 0..count {
                    let base = base.clone();
                    let ts = state.next_timestamp();
                    let new_id = state.arena.insert_with(|oid| {
                        let mut obj = GameObject::new_bare(oid, you, ObjectKind::Permanent, base);
                        obj.timestamp = ts;
                        obj
                    });
                    state
                        .zones
                        .insert(new_id, ZoneLocation::Battlefield, ZonePosition::Top);
                    if let Some(obj) = state.object_mut(new_id) {
                        obj.zone = crate::zone::Zone::Battlefield;
                    }
                }
            }
            None
        }
        Effect::CreateTokenCopyOfFirstToken => {
            let token = state
                .zones
                .list(ZoneLocation::Battlefield)
                .iter()
                .copied()
                .find(|id| {
                    state.object(*id).is_some_and(|o| {
                        o.card.is_none()
                            && o.controller == you
                            && o.characteristics()
                                .types
                                .contains(baylee_core::types::TypeSet::CREATURE)
                    })
                });
            if let Some(id) = token
                && let Some(base) = state.object(id).map(|o| o.base.clone())
            {
                let ts = state.next_timestamp();
                let new_id = state.arena.insert_with(|oid| {
                    let mut obj = GameObject::new_bare(oid, you, ObjectKind::Permanent, base);
                    obj.timestamp = ts;
                    obj
                });
                state
                    .zones
                    .insert(new_id, ZoneLocation::Battlefield, ZonePosition::Top);
                if let Some(obj) = state.object_mut(new_id) {
                    obj.zone = crate::zone::Zone::Battlefield;
                }
            }
            None
        }
        Effect::CreateTokenCopyOfEquipped { kicked_bonus, mods } => {
            let kicked = state.object(res.on_stack).is_some_and(|o| o.kicked);
            let count = 1 + if kicked { u32::from(kicked_bonus) } else { 0 };
            if let Some(equipped) = state.object(res.source).and_then(|o| o.attached_to)
                && let Some(base) = state.object(equipped).map(|o| o.base.clone())
            {
                for _ in 0..count {
                    let mut base = base.clone();
                    for m in mods {
                        apply_copy_mod(&mut base, m);
                    }
                    let ts = state.next_timestamp();
                    let id = state.arena.insert_with(|oid| {
                        let mut obj = GameObject::new_bare(oid, you, ObjectKind::Permanent, base);
                        obj.timestamp = ts;
                        obj
                    });
                    state
                        .zones
                        .insert(id, ZoneLocation::Battlefield, ZonePosition::Top);
                    if let Some(obj) = state.object_mut(id) {
                        obj.zone = crate::zone::Zone::Battlefield;
                    }
                }
            }
            None
        }
        Effect::CreateTokenN { token, amount } => {
            let mut count = amount2(&amount, state, you, res.source, res.x, &res.targets);
            // Token-creation replacements double the total (CR 614.1).
            if let Some(source_obj) = state.object(res.source) {
                for entry in &state.replacement_rules {
                    if let baylee_cards_dsl::ReplacementRule::DoubleTokenCreation {
                        controller_filter,
                    } = entry.rule
                        && eval::matches(
                            controller_filter,
                            state,
                            source_obj,
                            res.controller,
                            entry.source,
                        )
                    {
                        count *= 2;
                    }
                }
            }
            for _ in 0..count {
                create_one_token(state, you, token);
            }
            None
        }
        Effect::CreateTokenPtPerCount {
            token,
            filter,
            p,
            t,
        } => {
            let id = create_one_token(state, you, token);
            let ts = state.next_timestamp();
            state.effects.register(crate::effects::ContinuousEffect {
                id: baylee_core::ids::EffectId::new(0),
                source: Some(id),
                controller: you,
                layer: baylee_cards_dsl::Layer::PtModify,
                timestamp: ts,
                duration: baylee_cards_dsl::Duration::WhileSourceOnBattlefield,
                filter: crate::effects::EffectFilter::ObjectIs(id),
                modifier: baylee_cards_dsl::Modifier::ModifyPTPerCount { filter, p, t },
            });
            None
        }
        Effect::CreateToken { token } => {
            // Token-creation replacements (Doubling Season, CR 614.1).
            let mut count = 1u32;
            if let Some(source_obj) = state.object(res.source) {
                for entry in &state.replacement_rules {
                    if let baylee_cards_dsl::ReplacementRule::DoubleTokenCreation {
                        controller_filter,
                    } = entry.rule
                        && eval::matches(
                            controller_filter,
                            state,
                            source_obj,
                            res.controller,
                            entry.source,
                        )
                    {
                        count *= 2;
                    }
                }
            }
            for _ in 0..count {
                create_one_token(state, res.controller, token);
            }
            None
        }
        Effect::CreateTokenFromLinked { token } => {
            // The exiled card's owner creates the token; its power and
            // toughness are the exiled card's mana value.
            let mut owner = None;
            let mut cmc = 0;
            'scan: for seat in 0..state.players.len() {
                let p = PlayerId::new(seat as u8);
                for &card in state.zones.list(ZoneLocation::Exile(p)) {
                    if state.object(card).is_some_and(|o| {
                        o.riders
                            .iter()
                            .any(|r| matches!(r, crate::object::Rider::Linked { host } if *host == res.source))
                    }) {
                        owner = Some(p);
                        cmc = state
                            .object(card)
                            .map_or(0, |o| o.characteristics().mana_cost.cmc());
                        break 'scan;
                    }
                }
            }
            if let Some(owner) = owner {
                let mut def = *token;
                def.power = Some(cmc as i16);
                def.toughness = Some(cmc as i16);
                create_one_token(state, owner, &def);
            }
            None
        }
        _ => unreachable!("not a token effect"),
    }
}

/// Applies a copy modification to a token's base characteristics.
pub(super) fn apply_copy_mod(base: &mut Characteristics, m: &baylee_cards_dsl::CopyMod) {
    match m {
        baylee_cards_dsl::CopyMod::AddType(t) => {
            base.types = base.types.union(*t);
        }
        baylee_cards_dsl::CopyMod::RemoveType(t) => {
            base.types = base.types.difference(*t);
        }
        baylee_cards_dsl::CopyMod::RemoveSupertype(s) => {
            base.supertypes = base.supertypes.difference(*s);
        }
        baylee_cards_dsl::CopyMod::AddSubtype(s) => {
            base.subtypes.insert(*s);
        }
        baylee_cards_dsl::CopyMod::AddKeyword(k) => {
            base.keywords = base.keywords.union(*k);
        }
        baylee_cards_dsl::CopyMod::AddCounter(_, _) => {}
    }
}

pub(super) fn create_one_token(
    state: &mut GameState,
    controller: PlayerId,
    token: &baylee_cards_dsl::TokenDef,
) -> ObjectId {
    let name = state.names.intern(token.name);
    let base = Characteristics {
        name,
        mana_cost: ManaCost::ZERO,
        colors: token.colors,
        types: token.types,
        supertypes: token.supertypes,
        subtypes: SubtypeSet::from_slice(token.subtypes),
        keywords: token.keywords,
        power: token.power,
        toughness: token.toughness,
        loyalty: None,
        color_identity: ColorSet::EMPTY,
        produced_colors: ColorSet::EMPTY,
        produced_colorless: false,
    };
    let ts = state.next_timestamp();
    let id = state.arena.insert_with(|id| {
        let mut obj = GameObject::new_bare(id, controller, ObjectKind::Permanent, base);
        obj.timestamp = ts;
        obj
    });
    state
        .zones
        .insert(id, ZoneLocation::Battlefield, ZonePosition::Top);
    if let Some(obj) = state.object_mut(id) {
        obj.zone = crate::zone::Zone::Battlefield;
    }
    id
}
