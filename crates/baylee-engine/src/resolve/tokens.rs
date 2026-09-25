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
            // CR 701.47a: choose an *Army* you control — not a creature of the
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
                && let Some(base) = crate::layers::copiable_values(state, id)
            {
                create_token_copies(state, you, id, &base, 1);
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
                    apply_copy_mod(&mut modified, m);
                }
                create_token_copies(state, you, equipped, &std::sync::Arc::new(modified), count);
            }
            None
        }
        Effect::CreateTokenN { token, amount } => {
            let count = amount2(&amount, state, you, res);
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
                    origin: crate::effects::EffectOrigin::Static,
                    layer: baylee_cards_dsl::Layer::PtModify,
                    timestamp: ts,
                    duration: baylee_cards_dsl::Duration::WhileSourceOnBattlefield,
                    filter: crate::effects::EffectFilter::object(state, id),
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
        // All three are about the object rather than about the
        // characteristics this function is handed. A counter is put on by
        // the caller (CR 614.1c), and an ability is not a `Characteristics`
        // field at all — `progress::apply_copy_choice` keeps the copier's
        // statics by registering them as the copy's own continuous effects,
        // and this token door reaches no effect table. A granted ability
        // (`CopyMod::Grant`, CR 707.9a) is the same case: a token or spell
        // copy made "except it has …" would lose it here, and nothing in the
        // pool is one.
        baylee_cards_dsl::CopyMod::AddCounter(_, _)
        | baylee_cards_dsl::CopyMod::KeepOtherAbilities
        | baylee_cards_dsl::CopyMod::Grant(_) => {}
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
    // The player a token is created under owns it (CR 111.2), and nothing
    // is created for a player who has left the game (CR 800.4b, 800.4d).
    if state.has_left(controller) {
        return Vec::new();
    }
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
    // As `create_tokens` (CR 800.4b, 800.4d).
    if state.has_left(controller) {
        return Vec::new();
    }
    let (own, token, face) = state.object(original).map_or((None, None, None), |o| {
        (
            o.own_abilities.map(|abilities| crate::object::AbilityList {
                abilities,
                printed: o.own_face,
            }),
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
                if let Some(own) = own {
                    obj.take_abilities(own);
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
    arrive(state, id);
    id
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{CardLookup, ReplacementEntry};
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
}
