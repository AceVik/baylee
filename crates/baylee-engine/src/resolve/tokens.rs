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
                // Crib Swap and An Offer You Can't Refuse hand the tokens to
                // the player whose permanent or spell was answered, so the
                // replacement is read off *them*: CR 614.1 asks whose control
                // the tokens would be created under, not whose spell is
                // creating them. My Doubling Season must not double the
                // Shapeshifter my own removal hands them, and theirs must.
                let controller = state.object(target_id).map_or(you, |o| o.controller);
                create_tokens(state, controller, token, None, 1);
            }
            None
        }
        Effect::Amass {
            token,
            subtype,
            amount,
        } => {
            // CR 701.44a: choose an *Army* you control — not a creature of the
            // named type. Searching for the named type instead is how "amass
            // Orcs 1" used to grow Orcish Bowmasters itself, which is an Orc
            // Archer and no Army at all.
            let army_type = token
                .subtypes
                .first()
                .copied()
                .unwrap_or(baylee_core::ids::SubtypeId::new(0));
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
                            && o.characteristics().subtypes.contains(army_type)
                    })
                });
            // The one token creation that deliberately does *not* go through
            // [`create_tokens`]. Doubling Season would make two Armies and
            // the counters then go on one Army you control, which is a
            // choice — and amass has nowhere to ask it. Doubling it here
            // would silently pick for the player; leaving it is a known
            // undercount, and the honest one until the choice exists.
            let target_id = army.unwrap_or_else(|| create_token(state, you, token, None));
            // CR 701.44b: the Army becomes the named type in addition to its
            // other types, whether it was just created or was already there.
            // Written into the base rather than registered as a continuous
            // effect because it has no duration and an Army is always a
            // token, so there is nothing underneath for it to shadow.
            if let Some(obj) = state.object_mut(target_id) {
                obj.base_mut().subtypes.insert(subtype);
                obj.cache.clear();
            }
            crate::replacement::put_counters(
                state,
                target_id,
                baylee_cards_dsl::CounterKind::P1P1,
                amount,
            );
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
                create_token_copies(state, you, id, &base, count);
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
                create_token_copies(state, you, id, &base, 1);
            }
            None
        }
        Effect::CreateTokenCopyOfEquipped { kicked_bonus, mods } => {
            let kicked = state.object(res.on_stack).is_some_and(|o| o.kicked);
            let count = 1 + if kicked { u32::from(kicked_bonus) } else { 0 };
            if let Some(equipped) = state.object(res.source).and_then(|o| o.attached_to)
                && let Some(base) = state.object(equipped).map(|o| o.base.clone())
            {
                // The modifications are applied once and the result copied,
                // rather than per token: they do not depend on how many
                // there are, and the doubling below must see the same base
                // every copy gets.
                let mut modified = (*base).clone();
                for m in mods {
                    apply_copy_mod(&mut modified, m);
                }
                create_token_copies(state, you, equipped, &std::sync::Arc::new(modified), count);
            }
            None
        }
        Effect::CreateTokenN { token, amount } => {
            let count = amount2(&amount, state, you, res.source, res.x, &res.targets);
            create_tokens(state, you, token, None, count);
            None
        }
        Effect::CreateTokenPtPerCount {
            token,
            filter,
            p,
            t,
        } => {
            // One effect per token: `EffectFilter::ObjectIs` names exactly
            // one object, so a doubled token that shared its twin's effect
            // would be the printed 0/0 the definition leaves behind.
            for id in create_tokens(state, you, token, None, 1) {
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
            }
            None
        }
        Effect::CreateToken { token } => {
            create_tokens(state, res.controller, token, None, 1);
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
                create_tokens(state, owner, token, Some(cmc as i16), 1);
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

/// The door every token-creating effect goes through, and the only place
/// CR 614.1 is read.
///
/// `count` is what the effect asks for; what arrives is that many times the
/// recipient's multiplier, because "if one or more tokens would be created
/// under **your** control" is a statement about who ends up with them and
/// says nothing about whose effect is creating them. So `controller` is the
/// player the tokens are created under — the exiled creature's controller
/// for Crib Swap, the exiled card's owner for Skyclave Apparition — and not
/// the resolving effect's own.
///
/// `size` is for a token the effect measures rather than the definition
/// printing it. Skyclave Apparition's Illusion is "X/X, where X is the
/// exiled card's mana value": the definition deliberately leaves power and
/// toughness unset and this is what fills them in. Overriding here rather
/// than copying the definition and editing it is what keeps the token's
/// identity — a copy is a different `TokenDef` with no registry entry, and
/// the art key would be lost.
pub(super) fn create_tokens(
    state: &mut GameState,
    controller: PlayerId,
    token: &'static baylee_cards_dsl::TokenDef,
    size: Option<i16>,
    count: u32,
) -> Vec<ObjectId> {
    let count = count.saturating_mul(crate::replacement::token_multiplier(state, controller));
    (0..count)
        .map(|_| create_token(state, controller, token, size))
        .collect()
}

/// The same door for a token that is a **copy** of something already in
/// play, which has a set of characteristics where the others have a
/// definition.
///
/// A token copy is a token, so the same replacement applies: Rite of
/// Replication under a Doubling Season makes two copies, or ten when it was
/// kicked. It was the three copy branches reading no replacement at all
/// that made this a second door rather than one more argument — a
/// `TokenDef` and a copied set of characteristics are different inputs, and
/// a token built from the second carries no definition, which is the
/// engine's other notion of what a token is.
///
/// Carrying no card is not the same as carrying no rules text. CR 707.2
/// copies the original's abilities along with its characteristics, and
/// `original` is read for all three places those can live: a copy of a
/// Treasure keeps the definition, a copy of a copy keeps the list that copy
/// was given, and a copy of a card names the face whose printed abilities it
/// still owes — [`GameState::pending_copied_faces`], because a face's
/// abilities are behind the card registry and this crate has no lookup for
/// it.
pub(super) fn create_token_copies(
    state: &mut GameState,
    controller: PlayerId,
    original: ObjectId,
    base: &std::sync::Arc<Characteristics>,
    count: u32,
) -> Vec<ObjectId> {
    let (own, token, face) = state.object(original).map_or((None, None, None), |o| {
        (
            o.own_abilities,
            o.token,
            o.card.map(|c| (c.index, o.face_index)),
        )
    });
    let count = count.saturating_mul(crate::replacement::token_multiplier(state, controller));
    (0..count)
        .map(|_| {
            let ts = state.next_timestamp();
            let id = state.arena.insert_with(|oid| {
                let mut obj =
                    GameObject::new_bare(oid, controller, ObjectKind::Permanent, base.clone());
                obj.timestamp = ts;
                obj.own_abilities = own;
                obj.token = token;
                obj
            });
            if let Some((card, face)) = face {
                state.pending_copied_faces.push((id, card, face));
            }
            state
                .zones
                .insert(id, ZoneLocation::Battlefield, ZonePosition::Top, true);
            if let Some(obj) = state.object_mut(id) {
                obj.zone = crate::zone::Zone::Battlefield;
            }
            state.invalidate_projections();
            id
        })
        .collect()
}

fn create_token(
    state: &mut GameState,
    controller: PlayerId,
    token: &'static baylee_cards_dsl::TokenDef,
    size: Option<i16>,
) -> ObjectId {
    // Every Zombie of the same kind shares one printed face: the board this
    // has to survive is thousands of them.
    let base = state.token_base(token, size);
    let ts = state.next_timestamp();
    let id = state.arena.insert_with(|id| {
        let mut obj = GameObject::new_bare(id, controller, ObjectKind::Permanent, base);
        obj.timestamp = ts;
        // What makes this a Treasure rather than a blank artifact: the
        // definition is where the token's abilities live, and it is the only
        // record of which token this is once the characteristics are copied
        // out of it.
        obj.token = Some(token);
        obj
    });
    state
        .zones
        .insert(id, ZoneLocation::Battlefield, ZonePosition::Top, true);
    if let Some(obj) = state.object_mut(id) {
        obj.zone = crate::zone::Zone::Battlefield;
    }
    // A permanent that just arrived has never been projected: the anthem it
    // is standing under, and any counter about to be placed on it, are both
    // invisible until something asks for the pass.
    state.invalidate_projections();
    id
}
