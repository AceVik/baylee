//! Token creation: plain tokens, copies, per-count sizing, Amass, and
//! the shared token factory.

#[allow(clippy::wildcard_imports)] // family modules share the resolve vocabulary
use super::*;
use crate::text_changes::TextChangeMap;

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
        } => {
            // CR 701.47a: choose an *Army* you control — not a creature of the
            // named type. Searching for the named type instead is how "amass
            // Orcs 1" used to grow Orcish Bowmasters itself, which is an Orc
            // Archer and no Army at all.
            let army_type = token
                .subtypes
                .first()
                .copied()
                .unwrap_or(baylee_core::ids::SubtypeId::new(0));
            let army = state.battlefield_seen().find(|id| {
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
            let target_id =
                army.unwrap_or_else(|| create_token_with_text(state, you, token, None, res.text));
            // CR 701.47a: the Army becomes the named type in addition to its
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
            if let Some(original) = res.targets.first().copied()
                && let Some(base) = crate::layers::copiable_values(state, original)
            {
                let mut modified = (*base).clone();
                for m in mods {
                    apply_copy_mod_with_text(&mut modified, m, res.text);
                }
                let made =
                    create_token_copies(state, you, original, &std::sync::Arc::new(modified), 1);
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
            // The source card, wherever the cost put it: exile, for
            // eternalize. Its copiable values there are the card's own.
            if let Some(base) = crate::layers::copiable_values(state, res.source) {
                let mut modified = (*base).clone();
                for m in mods {
                    apply_copy_mod_with_text(&mut modified, m, res.text);
                }
                create_token_copies(state, you, res.source, &std::sync::Arc::new(modified), 1);
            }
            None
        }
        Effect::CreateTokenCopyOfEquipped { kicked_bonus, mods } => {
            let kicked = state.object(res.on_stack).is_some_and(|o| o.kicked);
            let count = 1 + if kicked { u32::from(kicked_bonus) } else { 0 };
            if let Some(equipped) = state.object(res.source).and_then(|o| o.attached_to)
                && let Some(base) = crate::layers::copiable_values(state, equipped)
            {
                // The modifications are applied once and the result copied,
                // rather than per token: they do not depend on how many
                // there are, and the doubling below must see the same base
                // every copy gets.
                let mut modified = (*base).clone();
                for m in mods {
                    apply_copy_mod_with_text(&mut modified, m, res.text);
                }
                create_token_copies(state, you, equipped, &std::sync::Arc::new(modified), count);
            }
            None
        }
        Effect::CreateTokenN { token, amount } => {
            let count = amount2(&amount, state, you, res);
            create_tokens_with_text(state, you, token, None, count, res.text);
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

fn create_tokens_with_text(
    state: &mut GameState,
    controller: PlayerId,
    token: &'static baylee_cards_dsl::TokenDef,
    size: Option<i16>,
    count: u32,
    text: TextChangeMap,
) -> Vec<ObjectId> {
    // The player a token is created under owns it (CR 111.2), and nothing
    // is created for a player who has left the game (CR 800.4b, 800.4d).
    if state.has_left(controller) {
        return Vec::new();
    }
    let count = count.saturating_mul(crate::replacement::token_multiplier(state, controller));
    (0..count)
        .map(|_| create_token_with_text(state, controller, token, size, text))
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
    let count = count.saturating_mul(crate::replacement::token_multiplier(state, controller));
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

fn create_token_with_text(
    state: &mut GameState,
    controller: PlayerId,
    token: &'static baylee_cards_dsl::TokenDef,
    size: Option<i16>,
    text: TextChangeMap,
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
    arrive(state, id);
    id
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{CardLookup, ReplacementEntry};
    use crate::text_changes::TextChangeMap;
    use baylee_cards_dsl::{Filter, ReplacementRule};
    use baylee_core::ids::CardIndex;
    use baylee_core::preset::{
        AIProfile, DeckEntry, FormatId, GamePreset, HouseRules, PrintInfo, SeatCapabilities,
        SeatController, SeatSpec,
    };

    struct RegistryLookup;
    impl CardLookup for RegistryLookup {
        fn card(&self, index: CardIndex) -> Option<&'static baylee_cards_dsl::CardDef> {
            baylee_cards::by_index(index)
        }
    }

    fn me() -> PlayerId {
        PlayerId::new(0)
    }
    fn them() -> PlayerId {
        PlayerId::new(1)
    }

    fn treasure() -> &'static baylee_cards_dsl::TokenDef {
        &baylee_cards::tokens::TREASURE
    }

    /// Seat 0 starts with one Forest on the battlefield: a token copy of a
    /// **card** is the one branch that needs a card to copy.
    fn state() -> GameState {
        let forest = baylee_cards::by_oracle_id("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
            .expect("registry contains Forest")
            .index;
        let entry = DeckEntry {
            card: forest,
            print: baylee_core::ids::PrintRef::new(0),
        };
        let seat = |bf: Vec<DeckEntry>| SeatSpec {
            controller: SeatController::Ai(AIProfile::default()),
            capabilities: SeatCapabilities::default(),
            deck: (0..60).map(|_| entry).collect(),
            sideboard: vec![],
            commanders: vec![],
            starting_life: None,
            starting_hand: None,
            starting_battlefield: bf,
            emblems: vec![],
            team: None,
        };
        let preset = GamePreset {
            format: FormatId::Freeform,
            seed: 6,
            house_rules: HouseRules::default(),
            modifiers: vec![],
            prints: vec![PrintInfo {
                scryfall_id: uuid::Uuid::nil(),
                lang: "EN".into(),
                finish: baylee_core::preset::Finish::Normal,
            }],
            seats: vec![seat(vec![entry]), seat(vec![])],
        };
        GameState::from_preset(&preset, &RegistryLookup).expect("game starts")
    }

    fn the_forest(state: &GameState) -> ObjectId {
        *state
            .zones
            .list(ZoneLocation::Battlefield)
            .first()
            .expect("seat 0 started with one")
    }

    /// "If one or more tokens would be created under your control" — a
    /// Doubling Season controlled by seat 0.
    fn doubles_tokens(state: &mut GameState, controller: PlayerId) {
        let name = state.names.intern("Doubling Season");
        let source = state.create_bare(
            controller,
            ObjectKind::Permanent,
            name,
            ZoneLocation::Battlefield,
        );
        state.replacement_rules.push(ReplacementEntry {
            source,
            controller,
            rule: ReplacementRule::DoubleTokenCreation {
                controller_filter: &Filter::ControlledByYou,
            },
        });
    }

    /// The factory is a **door** (CR 614.1): a token made beside it is
    /// invisible to every replacement that multiplies tokens. And the rule
    /// is read off whoever ends up with them — Crib Swap hands its
    /// Shapeshifter to the player whose creature it answered, so my
    /// Doubling Season must not double that one.
    #[test]
    fn the_token_factory_is_where_the_doubling_replacement_applies() {
        let mut state = state();
        doubles_tokens(&mut state, me());

        assert_eq!(
            create_tokens(&mut state, me(), treasure(), None, 1).len(),
            2
        );
        assert_eq!(
            create_tokens(&mut state, them(), treasure(), None, 1).len(),
            1,
            "the tokens are created under their control, not under mine"
        );
        assert_eq!(
            create_tokens(&mut state, me(), treasure(), None, 3).len(),
            6,
            "the multiplier applies to the whole creation"
        );
    }

    /// A token copy is a token, so the same replacement applies: Rite of
    /// Replication under a Doubling Season makes two copies. That the three
    /// copy branches read no replacement at all is why this is a second
    /// door rather than one more argument to the first.
    #[test]
    fn a_token_copy_goes_through_the_same_replacement() {
        let mut state = state();
        doubles_tokens(&mut state, me());
        let forest = the_forest(&state);
        let base = std::sync::Arc::new(
            state
                .object(forest)
                .expect("the Forest")
                .characteristics()
                .clone(),
        );

        let copies = create_token_copies(&mut state, me(), forest, &base, 1);
        assert_eq!(copies.len(), 2);
        assert_eq!(
            create_token_copies(&mut state, them(), forest, &base, 1).len(),
            1
        );
    }

    /// CR 707.2 copies the original's abilities with its characteristics,
    /// and a copy of a *card* owes the face whose printed abilities it still
    /// carries — this crate has no card registry, so the face is queued for
    /// whoever does.
    #[test]
    fn a_copy_of_a_card_queues_the_face_its_abilities_are_behind() {
        let mut state = state();
        let forest = the_forest(&state);
        let card = state
            .object(forest)
            .expect("the Forest")
            .card
            .expect("a card");
        let base = std::sync::Arc::new(
            state
                .object(forest)
                .expect("the Forest")
                .characteristics()
                .clone(),
        );
        state.pending_copied_faces.clear();

        let copies = create_token_copies(&mut state, me(), forest, &base, 2);

        assert_eq!(copies.len(), 2);
        assert_eq!(
            state.pending_copied_faces,
            vec![(copies[0], card.index, 0), (copies[1], card.index, 0)],
            "one entry per copy — a shared queue with one entry would leave \
             the second copy a blank permanent"
        );
        // A copy of a *token* has its definition instead, and owes no face.
        let treasures = create_tokens(&mut state, me(), treasure(), None, 1);
        state.pending_copied_faces.clear();
        let base = std::sync::Arc::new(
            state
                .object(treasures[0])
                .expect("a Treasure")
                .characteristics()
                .clone(),
        );
        let copy = create_token_copies(&mut state, me(), treasures[0], &base, 1);
        assert!(state.pending_copied_faces.is_empty());
        assert!(
            std::ptr::eq(
                state
                    .object(copy[0])
                    .expect("the copy")
                    .token
                    .expect("a definition"),
                treasure()
            ),
            "the copy of a Treasure is a Treasure: the definition is the \
             only record of which token this is once the characteristics \
             have been copied out of it"
        );
    }

    /// A token is not moved onto the battlefield — it is created there. But
    /// CR 603.6a asks whether a permanent *entered* and not how it got
    /// there, and the engine reads that off this one event, so a token that
    /// arrived in silence was a board Nesting Dovehawk never saw.
    ///
    /// `from` is `OutsideGame`, the only variant meaning no zone at all
    /// (CR 400.1): the leaves-, dies- and exiled-from-battlefield triggers
    /// all read that side and every one of them wants `Battlefield` there,
    /// so naming a zone the token was never in would be both a lie and a
    /// trigger.
    #[test]
    fn a_token_enters_the_battlefield_and_says_so() {
        let mut state = state();
        state.characteristics_generation = 0;
        let entries = state.journal.len();

        let tokens = create_tokens(&mut state, me(), treasure(), None, 1);
        let id = tokens[0];

        let obj = state.object(id).expect("just made it");
        assert_eq!(obj.zone, crate::zone::Zone::Battlefield);
        assert_eq!(obj.controller, me());
        assert!(
            std::ptr::eq(obj.token.expect("a definition"), treasure()),
            "what makes it a Treasure rather than a blank artifact"
        );
        assert!(
            state.zones.list(ZoneLocation::Battlefield).contains(&id),
            "and the zone list agrees with the object"
        );
        assert_eq!(
            state.characteristics_generation,
            u64::MAX,
            "a permanent that just arrived has never been projected"
        );
        assert_eq!(state.journal.len(), entries + 1);
        assert!(
            matches!(
                state.journal.entries().last().expect("an entry").event,
                GameEvent::ZoneChanged {
                    object,
                    from: crate::zone::Zone::OutsideGame,
                    to: crate::zone::Zone::Battlefield,
                    ..
                } if object == id
            ),
            "recorded {:?}",
            state.journal.entries().last().expect("an entry").event
        );
    }

    /// An ability of seat 0's with `targets`, resolving `effects` in order.
    fn resolve(state: &mut GameState, targets: &[ObjectId], effects: Vec<Effect>) {
        let mut res = Resolution {
            source: ObjectId::NO_SOURCE,
            on_stack: ObjectId::NO_SOURCE,
            controller: me(),
            effects,
            pc: 0,
            targets: SmallVec::from_slice(targets),
            second_targets: SmallVec::new(),
            x: None,
            chosen_player: None,
            target_lki: None,
            subject: crate::resolve::SubjectContext::default(),
            event_mana: None,
            retarget_left: None,
            target_players: baylee_core::ids::SeatSet::new(),
            event_object: None,
            awaiting: None,
            targeted: true,
            mana_ability: false,
            countered_source: None,
            text: TextChangeMap::IDENTITY,
        };
        assert!(matches!(run(state, &mut res), Flow::Complete));
        state.refresh_characteristics();
    }

    /// Crib Swap: "Exile target creature. Its controller creates a 1/1
    /// colorless Shapeshifter creature token with changeling."
    ///
    /// By the time the second sentence is read the creature is in exile, so
    /// "its controller" is the controller it had as it last existed on the
    /// battlefield (CR 608.2h), here the player who had taken it. The field
    /// on the exiled card is no answer: it holds whatever the last refresh
    /// left there, and a refresh that reaches into exile settles it to the
    /// card's default, which after a steal is the player it was stolen from.
    /// Any effect whose filter names another zone makes every refresh reach
    /// every zone; Past in Flames' is the one used here.
    #[test]
    fn its_controller_is_the_one_the_creature_had_on_the_battlefield() {
        static FLASHBACK_REACH: Filter = Filter::And(&[
            Filter::INSTANT_OR_SORCERY,
            Filter::InZone(baylee_cards_dsl::ZoneRef::Graveyard),
            Filter::OwnedByYou,
        ]);
        let mut state = state();
        let name = state.names.intern("Creature");
        let it = state.create_bare(
            them(),
            ObjectKind::Permanent,
            name,
            ZoneLocation::Battlefield,
        );
        let obj = state.object_mut(it).expect("fresh");
        let mut base = (*obj.base).clone();
        base.types = baylee_core::types::TypeSet::CREATURE;
        obj.base = std::sync::Arc::new(base);
        state.invalidate_projections();
        crate::resolve::gain_control(&mut state, &[(it, me())]);
        resolve(
            &mut state,
            &[],
            vec![Effect::continuous(
                &FLASHBACK_REACH,
                baylee_cards_dsl::Modifier::GrantsFlashback,
                baylee_cards_dsl::Duration::UntilEndOfTurn,
            )],
        );
        let obj = state.object(it).expect("still there");
        assert_eq!((obj.owner, obj.controller), (them(), me()), "taken");

        let shapeshifter = &baylee_cards::tokens::SHAPESHIFTER_1_1_CHANGELING;
        resolve(
            &mut state,
            &[it],
            vec![
                Effect::exile(baylee_cards_dsl::TargetSpec::Object(&Filter::CREATURE)),
                Effect::CreateTokenForTargetController {
                    token: shapeshifter,
                },
            ],
        );
        assert_eq!(
            state.object(it).map(|o| o.zone),
            Some(crate::zone::Zone::Exile)
        );
        let tokens: Vec<PlayerId> = state
            .zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .filter_map(|id| state.object(*id))
            .filter(|o| o.token.is_some_and(|t| std::ptr::eq(t, shapeshifter)))
            .map(|o| o.controller)
            .collect();
        assert_eq!(
            tokens,
            vec![me()],
            "one Shapeshifter, for the player who controlled the creature when it was exiled"
        );
    }

    /// A token is owned by the player it is created under (CR 111.2), and
    /// nothing is created for a player who has left the game (CR 800.4b,
    /// 800.4d): neither door makes one.
    #[test]
    fn no_token_is_created_for_a_player_who_has_left() {
        let mut state = state();
        let forest = the_forest(&state);
        let base = state.object(forest).expect("a Forest").base.clone();
        crate::sba::eliminate_player(&mut state, them(), crate::event::LossReason::Conceded);

        assert!(create_tokens(&mut state, them(), treasure(), None, 1).is_empty());
        assert!(create_token_copies(&mut state, them(), forest, &base, 1).is_empty());
        assert_eq!(
            state.zones.list(ZoneLocation::Battlefield)[..],
            [forest],
            "the Forest alone"
        );
    }

    #[test]
    fn per_count_token_records_its_ability_for_copying_and_later_text_changes() {
        use crate::text_changes::TextReplacement;
        use baylee_cards_dsl::TextWordKind;
        use baylee_core::color::{Color, ColorSet};
        static GREEN: Filter = Filter::HasColor(ColorSet::of(Color::Green));
        let mut state = state();
        let green = create_tokens(
            &mut state,
            me(),
            &baylee_cards::tokens::BOAR_2_2_GREEN,
            None,
            1,
        )[0];
        // The token's characteristic is a value. The green word in its
        // quoted static ability remains text and can change afterwards.
        resolve(
            &mut state,
            &[],
            vec![Effect::CreateTokenPtPerCount {
                token: &baylee_cards::tokens::CONSTRUCT_ARTIFACT_0_0,
                filter: &GREEN,
                p: 1,
                t: 1,
            }],
        );
        let construct = state
            .battlefield_seen()
            .find(|id| {
                state
                    .printed_ability_list(*id)
                    .is_some_and(|rules| !rules.runtime_statics().is_empty())
            })
            .unwrap();
        assert_eq!(
            state.object(construct).unwrap().characteristics().power,
            Some(1)
        );
        let identity = state.source_identity(construct).unwrap();
        state.text_changes.replace(
            identity,
            TextReplacement {
                kind: TextWordKind::Color,
                from: 4,
                to: 1,
            },
        );
        state.invalidate_projections();
        state.refresh_characteristics();
        assert_eq!(
            state.object(construct).unwrap().characteristics().power,
            Some(0)
        );
        assert_eq!(
            state.object(green).unwrap().characteristics().colors,
            ColorSet::of(Color::Green)
        );
        let base = crate::layers::copiable_values(&state, construct).unwrap();
        let first = create_token_copies(&mut state, me(), construct, &base, 1)[0];
        let second = create_token_copies(&mut state, me(), first, &base, 1)[0];
        let rules = state.printed_ability_list(second).unwrap();
        assert_eq!(rules.runtime_statics().len(), 1);
        assert_eq!(
            rules.runtime_statics()[0].base_text,
            TextChangeMap::IDENTITY,
            "later Sleight on the original token is not copiable"
        );
    }

    #[test]
    fn token_instruction_words_are_copiable_but_later_text_changes_are_not() {
        use crate::text_changes::{RuleContext, TextReplacement};
        use baylee_cards_dsl::{AbilityDef, CopyMod, TextWordKind, TokenDef};
        use baylee_core::color::{Color, ColorSet};
        use baylee_core::mana::ManaColor;
        use baylee_core::types::TypeSet;
        static TOKEN: TokenDef = TokenDef {
            name: "Green token",
            colors: ColorSet::of(Color::Green),
            types: TypeSet::CREATURE,
            power: Some(1),
            toughness: Some(1),
            abilities: &[baylee_cards_dsl::mana_ability!(&[Effect::mana(
                ManaColor::Green,
                1
            )])],
            ..TokenDef::DEFAULT
        };
        let mut state = state();
        let mut text = TextChangeMap::IDENTITY;
        text.replace(TextReplacement {
            kind: TextWordKind::Color,
            from: 4,
            to: 1,
        });
        let made = create_tokens_with_text(&mut state, me(), &TOKEN, None, 1, text)[0];
        let base = crate::layers::copiable_values(&state, made).unwrap();
        assert_eq!(base.colors, ColorSet::of(Color::Blue));
        assert_eq!(state.names.get(base.name), "Green token");
        let rules = state.printed_ability_list(made).unwrap();
        let AbilityDef::Activated { effects, .. } = rules.abilities[0] else {
            panic!("mana ability")
        };
        assert_eq!(
            effects,
            &[Effect::mana(ManaColor::Green, 1)],
            "a green mana symbol is not the color word green"
        );
        assert!(crate::eval::matches_with_context(
            &Filter::HasColor(ColorSet::of(Color::Green)),
            &state,
            state.object(made).unwrap(),
            me(),
            RuleContext {
                source: made,
                text: rules.base_text(0)
            },
        ));
        let identity = state.source_identity(made).unwrap();
        state.text_changes.replace(
            identity,
            TextReplacement {
                kind: TextWordKind::Color,
                from: 1,
                to: 3,
            },
        );
        let copy = create_token_copies(&mut state, me(), made, &base, 1)[0];
        let copied_rules = state.printed_ability_list(copy).unwrap();
        assert_eq!(
            copied_rules.base_text(0).color_word(Color::Green),
            Color::Blue
        );
        assert_eq!(
            state.object(copy).unwrap().base.colors,
            ColorSet::of(Color::Blue)
        );
        let mut exception = (*base).clone();
        apply_copy_mod_with_text(
            &mut exception,
            &CopyMod::SetColor(ColorSet::of(Color::Green)),
            text,
        );
        assert_eq!(exception.colors, ColorSet::of(Color::Blue));
        apply_copy_mod_with_text(&mut exception, &CopyMod::KeepColor, text);
        assert_eq!(
            exception.colors,
            ColorSet::of(Color::Blue),
            "a retained color is a value"
        );
    }
}
