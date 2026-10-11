//! Token creation: plain tokens, copies, per-count sizing, Amass, and
//! the shared token factory.

#[allow(clippy::wildcard_imports)] // family modules share the resolve vocabulary
use super::*;
use crate::text_changes::TextChangeMap;

#[derive(Clone, Copy, Default)]
struct TokenEntry {
    tapped: bool,
    temporary_haste_source: Option<ObjectId>,
}

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
                //
                // "Its controller" once the sentence before has exiled it:
                // the controller it had as it last existed on the
                // battlefield (CR 608.2h), not the field on the exiled card.
                let controller = state.last_known_controller(target_id).unwrap_or(you);
                create_tokens_with_text(state, controller, token, None, 1, res.text);
            }
            None
        }
        Effect::Amass {
            token,
            subtype,
            amount,
        } => super::amass::start(state, res, token, subtype, amount),
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
            // Copiable values, not `base`: a token copy of a Cursed Mirror
            // that became an Elf is a copy of the Elf (CR 707.2), and `base`
            // is the artifact underneath it.
            if let Some(id) = target_id
                && let Some(base) = crate::layers::copiable_values(state, id)
            {
                create_token_copies(state, you, id, &base, count);
            }
            None
        }
        Effect::Populate => {
            let options: Vec<ObjectId> = state
                .battlefield_seen()
                .filter(|id| {
                    state.object(*id).is_some_and(|o| {
                        o.card.is_none()
                            && o.controller == you
                            && o.characteristics()
                                .types
                                .contains(baylee_core::types::TypeSet::CREATURE)
                    })
                })
                .collect();
            if options.is_empty() {
                return None;
            }
            res.awaiting = Some(AwaitingOp::Populate);
            Some(Pending::ChooseCards {
                player: you,
                options,
                min: 1,
                max: 1,
                prompt: ChoicePrompt::Generic,
                total: None,
            })
        }
        Effect::CreateTokenCopyOfFirstToken => {
            let token = state.battlefield_seen().find(|id| {
                state.object(*id).is_some_and(|o| {
                    o.card.is_none()
                        && o.controller == you
                        && o.characteristics()
                            .types
                            .contains(baylee_core::types::TypeSet::CREATURE)
                })
            });
            if let Some(id) = token
                && let Some(base) = crate::layers::copiable_values(state, id)
            {
                create_token_copies(state, you, id, &base, 1);
            }
            None
        }
        Effect::CreateTokenCopyOfTarget {
            mods,
            sacrifice_at_next_end_step,
        } => {
            if let Some(original) = res.targets.first().copied() {
                let made = create_modified_token_copies(state, res, original, mods, 1);
                // One delayed trigger per token, naming that token and no
                // other object: a token that has left and been replaced is
                // not "it" (CR 400.7), and the version says so.
                if sacrifice_at_next_end_step {
                    for id in made {
                        let version = state.object(id).map_or(0, |o| o.version);
                        state.delayed.push(crate::state::DelayedTrigger {
                            controller: you,
                            when: crate::state::DelayedWhen::NextEndStep,
                            action: crate::state::DelayedAction::Sacrifice { card: id, version },
                        });
                    }
                }
            }
            None
        }
        Effect::CreateTokenCopyOfSource { mods } => {
            create_modified_token_copies(state, res, res.source, mods, 1);
            None
        }
        Effect::CreateTokenCopyOfEquipped { kicked_bonus, mods } => {
            let kicked = state.object(res.on_stack).is_some_and(|o| o.kicked);
            let count = 1 + if kicked { u32::from(kicked_bonus) } else { 0 };
            if let Some(equipped) = state.object(res.source).and_then(|o| o.attached_to) {
                create_modified_token_copies(state, res, equipped, mods, count);
            }
            None
        }
        Effect::CreateTokensFor {
            player,
            token,
            amount,
        } => {
            let count = amount2(&amount, state, you, res);
            for controller in players_of(player, state, you, res) {
                create_tokens_with_text(state, controller, token, None, count, res.text);
            }
            None
        }
        Effect::CreateTokenN { token, amount } => {
            let count = amount2(&amount, state, you, res);
            create_tokens_with_text(state, you, token, None, count, res.text);
            None
        }
        Effect::CreateTokenModified {
            token,
            amount,
            tapped,
            haste,
        } => {
            let count = amount2(&amount, state, you, res);
            create_tokens_entering(
                state,
                you,
                token,
                None,
                count,
                res.text,
                TokenEntry {
                    tapped,
                    temporary_haste_source: haste.then_some(res.source),
                },
            );
            None
        }
        Effect::Fabricate { count, token } => {
            super::token_choices::fabricate(state, res, count, token)
        }
        Effect::CreateTokenThenAttachSelf { token } => {
            super::token_choices::living_weapon(state, res, token)
        }
        Effect::CreateTokenThenCounters {
            token,
            kind,
            amount,
        } => {
            let created = create_tokens_with_text(state, you, token, None, 1, res.text);
            let n = u16::try_from(amount2(&amount, state, you, res)).unwrap_or(u16::MAX);
            for id in created {
                crate::entry::put_counters(state, res.controller, id, kind, n);
            }
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
            for id in create_tokens_with_text(state, you, token, None, 1, res.text) {
                let modifier = baylee_cards_dsl::Modifier::ModifyPTPerCount { filter, p, t };
                let list = state
                    .printed_ability_list(id)
                    .expect("new token has its rules");
                let index = list.abilities.len() + list.runtime_statics().len();
                let list = list.with_runtime_static(crate::copiable_abilities::RuntimeStatic {
                    modifier,
                    base_text: res.text,
                });
                state
                    .object_mut(id)
                    .expect("new token exists")
                    .take_abilities(list);
                let ts = state.next_timestamp();
                let effect = state.effects.register(crate::effects::ContinuousEffect {
                    id: baylee_core::ids::EffectId::new(0),
                    source: Some(id),
                    controller: you,
                    origin: crate::effects::EffectOrigin::Static,
                    layer: baylee_cards_dsl::Layer::PtModify,
                    timestamp: ts,
                    duration: baylee_cards_dsl::Duration::WhileSourceOnBattlefield,
                    filter: crate::effects::EffectFilter::object(state, id),
                    modifier,
                });
                state.effect_text_overrides.push((
                    effect,
                    crate::text_changes::TextOrigin::Ability {
                        source: state.source_identity(id).expect("new token identity"),
                        index: u32::try_from(index).expect("ability index fits u32"),
                        base: res.text,
                    },
                ));
            }
            None
        }
        Effect::CreateToken { token } => {
            create_tokens_with_text(state, res.controller, token, None, 1, res.text);
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
                            .any(|r| matches!(r, crate::object::Rider::Linked { host, .. } if *host == res.source))
                    }) {
                        owner = Some(p);
                        cmc = state
                            .object(card)
                            .map_or(0, |o| o.characteristics().mana_value());
                        break 'scan;
                    }
                }
            }
            if let Some(owner) = owner {
                create_tokens_with_text(state, owner, token, Some(cmc as i16), 1, res.text);
            }
            None
        }
        _ => unreachable!("not a token effect"),
    }
}

/// Maps literal words in a copy exception, leaving copied values unchanged.
fn create_modified_token_copies(
    state: &mut GameState,
    res: &Resolution,
    original: ObjectId,
    mods: &[baylee_cards_dsl::CopyMod],
    count: u32,
) -> Vec<ObjectId> {
    let Some(base) = crate::layers::copiable_values(state, original) else {
        return Vec::new();
    };
    let mut modified = (*base).clone();
    for modifier in mods {
        apply_copy_mod_with_text(&mut modified, modifier, res.text);
    }
    let types = modified.types;
    let made = create_token_copies(
        state,
        res.controller,
        original,
        &std::sync::Arc::new(modified),
        count,
    );
    // Copy exceptions also suppress the abilities defining the replaced values
    // (CR 707.9d), before the next projection or entry trigger scan.
    if let Some(copied) = state.printed_ability_list(original) {
        let own = state
            .printed_ability_list(res.source)
            .unwrap_or_else(|| copied.clone());
        let index = state
            .object(res.on_stack)
            .and_then(|o| o.ability)
            .map_or(0, |a| a.index);
        let abilities = crate::copiable_abilities::compose(copied, &own, index, mods, None);
        for id in &made {
            if let Some(object) = state.object_mut(*id) {
                object.take_abilities(abilities.clone());
            }
        }
    }
    // No rules processing intervenes before these entry counters are placed.
    // Each token receives them through the replacement path, independently of
    // the token multiplier. They must not enter the copied base or ability list.
    for &id in &made {
        for modifier in mods {
            if matches!(modifier, baylee_cards_dsl::CopyMod::EnterTapped) {
                state.set_tapped(id, true);
            }
            if let Some((kind, amount)) =
                crate::copiable_abilities::copy_entry_counter(modifier, types, res.x.unwrap_or(0))
                && amount > 0
            {
                crate::entry::put_counters(state, res.controller, id, kind, amount);
            }
        }
    }
    made
}

pub(super) fn apply_copy_mod_with_text(
    base: &mut Characteristics,
    modifier: &baylee_cards_dsl::CopyMod,
    text: TextChangeMap,
) {
    let previous = base.clone();
    crate::copiable_abilities::apply_characteristic_exceptions_with_text(
        base,
        &previous,
        std::slice::from_ref(modifier),
        text,
    );
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
    create_tokens_with_text(
        state,
        controller,
        token,
        size,
        count,
        TextChangeMap::IDENTITY,
    )
}

pub(super) fn create_tokens_with_text(
    state: &mut GameState,
    controller: PlayerId,
    token: &'static baylee_cards_dsl::TokenDef,
    size: Option<i16>,
    count: u32,
    text: TextChangeMap,
) -> Vec<ObjectId> {
    create_tokens_entering(
        state,
        controller,
        token,
        size,
        count,
        text,
        TokenEntry::default(),
    )
}

fn create_tokens_entering(
    state: &mut GameState,
    controller: PlayerId,
    token: &'static baylee_cards_dsl::TokenDef,
    size: Option<i16>,
    count: u32,
    text: TextChangeMap,
    entry: TokenEntry,
) -> Vec<ObjectId> {
    // The player a token is created under owns it (CR 111.2), and nothing
    // is created for a player who has left the game (CR 800.4b, 800.4d).
    if state.has_left(controller) {
        return Vec::new();
    }
    let Some(count) = crate::replacement::token_multiplier(state, controller, token.types)
        .and_then(|factor| count.checked_mul(factor))
    else {
        state.numeric_failure = Some("token creation exceeds u32 objects");
        return Vec::new();
    };
    (0..count)
        .map(|_| create_token_entering(state, controller, token, size, text, entry))
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
/// carries. The state's immutable rules cache supplies the current face's
/// definitions before any of the new objects is created.
/// Creates the copy of `token` that populate chose (CR 701.36a), from its
/// copiable values (CR 707.2).
pub(super) fn populate(state: &mut GameState, you: PlayerId, token: ObjectId) {
    if let Some(base) = crate::layers::copiable_values(state, token) {
        create_token_copies(state, you, token, &base, 1);
    }
}

pub(super) fn create_token_copies(
    state: &mut GameState,
    controller: PlayerId,
    original: ObjectId,
    base: &std::sync::Arc<Characteristics>,
    count: u32,
) -> Vec<ObjectId> {
    // As `create_tokens` (CR 800.4b, 800.4d).
    if state.has_left(controller) {
        return Vec::new();
    }
    let own = state.printed_ability_list(original);
    let (token, face) = state.object(original).map_or((None, None), |object| {
        (
            object.token,
            object.card.map(|card| (card.index, object.face_index)),
        )
    });
    let Some(count) = crate::replacement::token_multiplier(state, controller, base.types)
        .and_then(|factor| count.checked_mul(factor))
    else {
        state.numeric_failure = Some("token creation exceeds u32 objects");
        return Vec::new();
    };
    (0..count)
        .map(|_| {
            let ts = state.next_timestamp();
            let id = state.arena.insert_with(|oid| {
                let mut obj =
                    GameObject::new_bare(oid, controller, ObjectKind::Permanent, base.clone());
                obj.timestamp = ts;
                obj.controlled_since = ts;
                if let Some(own) = &own {
                    obj.take_abilities(own.clone());
                }
                obj.token = token;
                obj
            });
            if let Some((card, face)) = face {
                state.pending_copied_faces.push((id, card, face));
            }
            arrive(state, id);
            id
        })
        .collect()
}

/// Puts a freshly created token onto the battlefield, which is a permanent
/// **entering** it (CR 111.1).
///
/// The journal entry is the point. A token is not moved between zones — it
/// is created where it lands, which is why this is not
/// [`GameState::move_object`], the one other place a `ZoneChanged` is
/// recorded: that one begins by removing the object from the zone it was in,
/// and a token has never been in one. But CR 603.6a asks whether a permanent
/// entered the battlefield and not how it got there, and the engine reads
/// that question off exactly this event — so a battlefield full of tokens
/// was arriving without anything on the board being told. Nesting Dovehawk
/// watches for a creature token entering and had never once seen one.
///
/// `from` is [`crate::zone::Zone::OutsideGame`], the only variant that means *no zone at
/// all* (CR 400.1). That side of the event is read by the leaves-, dies- and
/// exiled-from-battlefield triggers, every one of which wants
/// `Zone::Battlefield` there, so naming a zone the token was never in would
/// be both a lie and a trigger.
///
/// The other reader is [`Engine::apply_enter_modifiers`], which scans the
/// same entries: a token now takes the as-it-enters half of its own rules
/// text too. Nothing in the pool's token definitions has any — no `TokenDef`
/// carries a triggered ability at all — so today that reaches only a token
/// **copy**, which is handed the original's list.
///
/// [`Engine::apply_enter_modifiers`]: crate::Engine::apply_enter_modifiers
fn arrive(state: &mut GameState, id: ObjectId) {
    if let Some(object) = state.object_mut(id) {
        object.zone = crate::zone::Zone::OutsideGame;
    }
    if crate::entry::defer(state, id, ZonePosition::Top, Cause::Effect) {
        return;
    }
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
    state.per_turn.entered_battlefield.push(id);
    state.journal.record(crate::event::GameEvent::ZoneChanged {
        object: id,
        from: crate::zone::Zone::OutsideGame,
        to: crate::zone::Zone::Battlefield,
        cause: crate::event::Cause::Effect,
        place: None,
    });
}

fn create_token_entering(
    state: &mut GameState,
    controller: PlayerId,
    token: &'static baylee_cards_dsl::TokenDef,
    size: Option<i16>,
    text: TextChangeMap,
    entry: TokenEntry,
) -> ObjectId {
    // Every Zombie of the same kind shares one printed face: the board this
    // has to survive is thousands of them.
    let mut base = state.token_base(token, size);
    if text != TextChangeMap::IDENTITY {
        let changed = std::sync::Arc::make_mut(&mut base);
        changed.colors = text.color_words(changed.colors);
        changed.subtypes = text.land_types(changed.subtypes);
        changed.keywords = text.keywords(changed.keywords);
    }
    let rules =
        crate::object::AbilityList::from_static(token.abilities, None, None).with_base_text(text);
    let ts = state.next_timestamp();
    let id = state.arena.insert_with(|id| {
        let mut obj = GameObject::new_bare(id, controller, ObjectKind::Permanent, base);
        obj.timestamp = ts;
        obj.controlled_since = ts;
        // What makes this a Treasure rather than a blank artifact: the
        // definition is where the token's abilities live, and it is the only
        // record of which token this is once the characteristics are copied
        // out of it.
        obj.token = Some(token);
        if text != TextChangeMap::IDENTITY {
            obj.take_abilities(rules);
        }
        obj
    });
    if entry.tapped {
        state.set_tapped(id, true);
    }
    if let Some(source) = entry.temporary_haste_source {
        let timestamp = state.next_timestamp();
        state.effects.register(crate::effects::ContinuousEffect {
            id: baylee_core::ids::EffectId::new(0),
            source: Some(source),
            controller,
            origin: crate::effects::EffectOrigin::Resolution,
            layer: baylee_cards_dsl::Layer::Ability,
            timestamp,
            duration: baylee_cards_dsl::Duration::UntilEndOfTurn,
            filter: crate::effects::EffectFilter::object(state, id),
            modifier: baylee_cards_dsl::Modifier::AddKeyword(baylee_cards_dsl::KeywordSet::HASTE),
        });
    }
    arrive(state, id);
    id
}

#[cfg(test)]
mod tests;
