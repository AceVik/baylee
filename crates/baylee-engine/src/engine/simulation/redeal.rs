//! Dealing the hidden cards of a fork again.
//!
//! A hidden card is two facts: which card it is, and which object holds it.
//! Both leak. The card obviously; the object because ids are handed out in
//! deck-list order as a game is set up, so whoever knows a deck list reads
//! a card off its id. So a deal changes both: the identities of one pool are
//! sorted, shuffled on the belief stream and put into new objects, allocated
//! in zone order, and the old objects are removed from the fork.

use super::ForkRefusal;
use crate::effects::EffectFilter;
use crate::object::{CardRef, Characteristics, GameObject, ObjectKind, Rider, Status};
use crate::rng::GameRng;
use crate::state::GameState;
use crate::zone::{Zone, ZoneLocation};
use baylee_core::card_metadata::{DeckChoices, RulePrint};
use baylee_core::ids::{CardIndex, ObjectId, PlayerId, PrintRef};
use smallvec::SmallVec;
use std::sync::Arc;

/// The hidden cards of one owner that are dealt together: zones the seat
/// knows only the sizes of, any of whose cards could be in any of them.
struct Pool {
    owner: PlayerId,
    /// In the order their slots are dealt.
    zones: SmallVec<[ZoneLocation; 2]>,
}

/// What a fork deals, planned on the real game before it is copied.
pub(super) struct Deal {
    /// The seat the fork is for.
    seat: PlayerId,
    pools: Vec<Pool>,
    /// Every hidden card's id, sorted: the objects the fork removes.
    retired: Vec<ObjectId>,
}

/// What a card is, as opposed to where it is or what happened to it: all
/// that a dealt object takes from the real card whose identity it gets.
struct Identity {
    card: CardRef,
    physical_rules: Option<Arc<RulePrint>>,
    deck_choices: DeckChoices,
    /// The printed face, shared with every copy of the card.
    base: Arc<Characteristics>,
    /// Its own power and toughness, if it defines them (CR 604.3).
    cda: Option<baylee_cards_dsl::Modifier>,
    /// Whether a static ability of it works outside the battlefield.
    offboard: bool,
}

impl Identity {
    /// The order a pool's identities are dealt from before the shuffle:
    /// the card, its print, then its rules print and deck choices. Two
    /// identities with equal keys are the same card in every respect a
    /// rule reads, so their relative order — the only thing the real
    /// arrangement could still decide — changes nothing.
    fn key(&self) -> (CardIndex, PrintRef, u64, u64) {
        (
            self.card.index,
            self.card.print,
            crate::state::structural_fingerprint(&self.physical_rules),
            crate::state::structural_fingerprint(&self.deck_choices),
        )
    }
}

impl Deal {
    /// The pools `seat` cannot see into, checked for what a deal could not
    /// carry over.
    ///
    /// Each opponent's hand and library are one pool, because the seat
    /// knows how many cards each holds and nothing about which; each
    /// opponent's cards outside the game are another, kept apart because
    /// no card crosses between the two unseen. The seat's own library is a
    /// pool of its own; its hand and its sideboard it can see.
    ///
    /// # Errors
    /// [`ForkRefusal::FaceDown`], [`ForkRefusal::HiddenExile`],
    /// [`ForkRefusal::MarkedHiddenCard`],
    /// [`ForkRefusal::ReferencedHiddenCard`] or
    /// [`ForkRefusal::HiddenCommander`].
    pub(super) fn plan(state: &GameState, seat: PlayerId) -> Result<Self, ForkRefusal> {
        let mut pools = Vec::new();
        for player in &state.players {
            let owner = player.id;
            if owner == seat {
                pools.push(Pool {
                    owner,
                    zones: smallvec::smallvec![ZoneLocation::Library(owner)],
                });
            } else {
                pools.push(Pool {
                    owner,
                    zones: smallvec::smallvec![
                        ZoneLocation::Hand(owner),
                        ZoneLocation::Library(owner)
                    ],
                });
                pools.push(Pool {
                    owner,
                    zones: smallvec::smallvec![ZoneLocation::OutsideGame(owner)],
                });
            }
        }
        let mut retired: Vec<ObjectId> = pools
            .iter()
            .flat_map(|pool| pool.zones.iter())
            .flat_map(|&loc| state.zones.list(loc).iter().copied())
            .collect();
        retired.sort_unstable();
        let deal = Self {
            seat,
            pools,
            retired,
        };
        deal.check(state)?;
        Ok(deal)
    }

    /// The ids the fork no longer has, sorted.
    pub(super) fn retired(&self) -> &[ObjectId] {
        &self.retired
    }

    fn is_retired(&self, id: ObjectId) -> bool {
        self.retired.binary_search(&id).is_ok()
    }

    fn check(&self, state: &GameState) -> Result<(), ForkRefusal> {
        for &id in &self.retired {
            let object = state
                .object(id)
                .ok_or(ForkRefusal::MarkedHiddenCard("an id with no object"))?;
            plain(object).map_err(ForkRefusal::MarkedHiddenCard)?;
        }
        for (id, object) in state.arena.iter() {
            if object.status.contains(Status::FACE_DOWN) {
                return Err(if object.zone == Zone::Exile {
                    ForkRefusal::HiddenExile
                } else {
                    ForkRefusal::FaceDown
                });
            }
            if !self.is_retired(id) && references(object).into_iter().any(|r| self.is_retired(r)) {
                return Err(ForkRefusal::ReferencedHiddenCard("an object"));
            }
        }
        if state
            .commanders
            .iter()
            .flatten()
            .any(|commander| self.is_retired(commander.object))
        {
            return Err(ForkRefusal::HiddenCommander);
        }
        if state
            .play_permissions
            .iter()
            .any(|permission| self.is_retired(permission.card))
        {
            return Err(ForkRefusal::ReferencedHiddenCard("a play permission"));
        }
        if state
            .discovered
            .iter()
            .any(|&(_, card, _)| self.is_retired(card))
        {
            return Err(ForkRefusal::ReferencedHiddenCard("a discovered card"));
        }
        // An effect about one exact incarnation that is now a hidden card.
        // One about an earlier incarnation already applies to nothing
        // (CR 400.7), in the fork as in the game.
        let hidden_now = |id: ObjectId, version: u32| {
            self.is_retired(id) && state.object(id).is_some_and(|o| o.version == version)
        };
        if state.effects.iter().any(|effect| match effect.filter {
            EffectFilter::ObjectIs(id, version) | EffectFilter::DslAbout(_, id, version) => {
                hidden_now(id, version)
            }
            EffectFilter::Dsl(_) | EffectFilter::DslControlledBy(..) => false,
        }) {
            return Err(ForkRefusal::ReferencedHiddenCard("a continuous effect"));
        }
        Ok(())
    }

    /// Deals every pool into `state`, a copy of the game the deal was
    /// planned on, and removes the cards it dealt from.
    pub(super) fn apply(&self, state: &mut GameState, belief: &mut GameRng) {
        for pool in &self.pools {
            let mut identities: Vec<Identity> = pool
                .zones
                .iter()
                .flat_map(|&loc| state.zones.list(loc).iter())
                .map(|&id| identity(state, id))
                .collect();
            identities.sort_by_cached_key(Identity::key);
            belief.shuffle(&mut identities);
            let mut dealt = identities.into_iter();
            for &loc in &pool.zones {
                let size = state.zones.list(loc).len();
                let ids: Vec<ObjectId> = dealt
                    .by_ref()
                    .take(size)
                    .map(|identity| insert(state, pool.owner, loc, identity))
                    .collect();
                debug_assert_eq!(ids.len(), size, "one identity per slot");
                *state.zones.list_mut(loc) = ids;
            }
        }
        for &id in &self.retired {
            state.arena.remove(id);
        }
        self.forget_retired(state);
        self.forget_hidden_incarnations(state);
    }

    /// Forgets what the game noted about the removed objects: derived lists
    /// a new object takes its own place in, and per-turn notes made about a
    /// card while it was hidden — what was drawn this turn, what a Wand of
    /// Ith looked at. Every one names an id that no longer exists in the
    /// fork, so forgetting it changes no rule there; keeping it would keep
    /// the real card's id, and with it the card.
    fn forget_retired(&self, state: &mut GameState) {
        let gone = |id: &ObjectId| self.is_retired(*id);
        state.printed_pt_cda.retain(|(id, _)| !gone(id));
        state.offboard_static_candidates.retain(|id| !gone(id));
        state.per_turn.drawn.retain(|(id, _)| !gone(id));
        state.per_turn.instruction_drawn.retain(|(id, _)| !gone(id));
        state
            .per_turn
            .wand_of_ith_targets
            .retain(|(_, id)| !gone(id));
        state
            .ability_fires
            .retain(|(source, _), _| !gone(&source.object));
    }

    /// Forgets the incarnations the game kept of objects as they left a
    /// zone the seat cannot see.
    ///
    /// Every move keeps a copy of the object as it was before it
    /// (`GameState::remember_damage_source`), so that a rule can still name
    /// that incarnation after the object has moved again; the copies are
    /// pruned at the end of the turn. A copy taken in a library, outside the
    /// game or in another player's hand is the card as it was while hidden
    /// — a draw keeps one of every card drawn — and no rule names a damage
    /// source, a shield's source or an ability's source there.
    fn forget_hidden_incarnations(&self, state: &mut GameState) {
        let seat = self.seat;
        state.damage_sources.retain(|object| match object.zone {
            Zone::Library | Zone::OutsideGame => false,
            Zone::Hand => object.owner == seat,
            Zone::Battlefield | Zone::Graveyard | Zone::Exile | Zone::Stack | Zone::Command => true,
        });
        let cache = &mut state.source_memory.cache;
        cache.damage_source_index.clear();
        for (index, object) in state.damage_sources.iter().enumerate() {
            cache.damage_source_index.insert(
                baylee_core::ids::DamageSourceRef {
                    object: object.id,
                    version: object.version,
                },
                index,
            );
        }
        cache.damage_source_len = state.damage_sources.len();
    }
}

/// The identity the real card behind `id` holds.
fn identity(state: &GameState, id: ObjectId) -> Identity {
    let object = state.object(id).expect("a planned hidden card exists");
    Identity {
        card: object.card.expect("a planned hidden card is card-backed"),
        physical_rules: object.physical_rules.clone(),
        deck_choices: object.deck_choices,
        base: Arc::clone(&object.base),
        cda: state
            .printed_pt_cda
            .iter()
            .find(|(card, _)| *card == id)
            .map(|(_, modifier)| *modifier),
        offboard: state.offboard_static_candidates.contains(&id),
    }
}

/// A new object holding `identity` in `loc`: a card as its deck put it
/// there, with no history of moves, so its version, timestamps and id say
/// nothing about the real card.
fn insert(
    state: &mut GameState,
    owner: PlayerId,
    loc: ZoneLocation,
    identity: Identity,
) -> ObjectId {
    let Identity {
        card,
        physical_rules,
        deck_choices,
        base,
        cda,
        offboard,
    } = identity;
    let id = state.arena.insert_with(|id| {
        let mut object = GameObject::new_card(id, owner, card, base);
        object.physical_rules = physical_rules;
        object.deck_choices = deck_choices;
        object.zone = loc.zone();
        object.zone_owner = loc.player();
        object
    });
    if let Some(modifier) = cda {
        state.printed_pt_cda.push((id, modifier));
    }
    if offboard {
        state.offboard_static_candidates.push(id);
    }
    id
}

/// Whether a hidden card is nothing but its identity in its zone, so that a
/// dealt card of the same identity would answer every rule the same way.
///
/// Refused: anything a rule reads of a card in a hand, a library or outside
/// the game — a reveal that binds it, counters, a status, an attachment, a
/// copy's base, a face other than the front. Ignored: what an earlier
/// incarnation left on the object and no rule reads after a move (the
/// targets, X and modes of a spell it was, its cast history), and its
/// version and timestamps, which a dealt card gets afresh.
#[allow(clippy::too_many_lines)] // one line per object field: the list is the guard
fn plain(object: &GameObject) -> Result<(), &'static str> {
    let GameObject {
        id: _,
        owner,
        controller,
        base_controller,
        zone: _,
        zone_owner,
        kind,
        card,
        physical_rules: _,
        deck_choices: _,
        base: _,
        cache: _,
        counters,
        damage: _,
        deathtouched: _,
        regeneration_shields: _,
        status,
        attached_to,
        timestamp: _,
        controlled_since: _,
        version: _,
        riders,
        targets: _,
        target_req: _,
        second: _,
        original_base,
        ability,
        source_power_lki: _,
        paid: _,
        x_value: _,
        kicked: _,
        replicated: _,
        alt_cast: _,
        prototyped,
        chosen_player: _,
        target_players: _,
        mode_index: _,
        modes: _,
        chosen_subtype: _,
        chosen_color: _,
        chosen_name: _,
        doors,
        face_index,
        own_abilities,
        own_abilities_until_eot: _,
        own_origin: _,
        token,
        pending_face_change,
        event_object: _,
        cast_history: _,
    } = object;
    let marks = [
        (
            *kind != ObjectKind::Card || card.is_none() || token.is_some() || ability.is_some(),
            "not a card",
        ),
        (
            controller != owner || base_controller != owner || *zone_owner != Some(*owner),
            "held by another player",
        ),
        (!counters.is_empty(), "counters"),
        (status.bits() != 0, "a status"),
        (attached_to.is_some(), "an attachment"),
        (!riders.is_empty(), "riders"),
        (original_base.is_some() || own_abilities.is_some(), "a copy"),
        (
            *face_index != 0 || *prototyped || pending_face_change.is_some(),
            "a face other than its front",
        ),
        (*doors != crate::object::Doors::NONE, "a room's doors"),
    ];
    match marks.iter().find(|(marked, _)| *marked) {
        Some(&(_, what)) => Err(what),
        None => Ok(()),
    }
}

/// Every object a public object points at: its targets, the object its
/// trigger is about, what it is attached to, and what its riders name.
fn references(object: &GameObject) -> SmallVec<[ObjectId; 4]> {
    let mut named: SmallVec<[ObjectId; 4]> = object.targets.iter().copied().collect();
    named.extend(object.event_object);
    named.extend(object.attached_to);
    if let Some(second) = &object.second {
        named.extend(second.targets.iter().copied());
    }
    named.extend(object.riders.iter().filter_map(rider_object));
    named
}

/// The object a rider names, if it names one.
const fn rider_object(rider: &Rider) -> Option<ObjectId> {
    match *rider {
        Rider::DelayedReturnObject(source)
        | Rider::EventSubject(source)
        | Rider::EventAbility(source)
        | Rider::CreatedBy(source)
        | Rider::ReanimatedCreature {
            creature: source, ..
        }
        | Rider::LinkedExile { source, .. } => Some(source.object),
        Rider::SourceAttachmentLki(object)
        | Rider::Linked { host: object, .. }
        | Rider::PhasedOutUntilHostLeaves { host: object, .. }
        | Rider::ExiledWith { host: object, .. }
        | Rider::MergedComponent(object)
        | Rider::MergedTop(object)
        | Rider::MergedInto(object) => Some(object),
        Rider::Masked
        | Rider::TriggerSourceVersion(_)
        | Rider::AbilitySourceVersion(_)
        | Rider::GrantedActivated
        | Rider::SourceAuraSuccessor(_)
        | Rider::SourceAttachmentVersion(_)
        | Rider::AttachmentHostLeft
        | Rider::EventObjectIdentity(..)
        | Rider::EventManaValue(_)
        | Rider::CounterSourceVersion(_)
        | Rider::Rebound
        | Rider::Adventure(_)
        | Rider::Foretold
        | Rider::Plotted
        | Rider::Suspend
        | Rider::Flashback
        | Rider::ExileInsteadOfGraveyard
        | Rider::Uncounterable
        | Rider::EntryP1P1(_)
        | Rider::EntryFinality
        | Rider::Prepared
        | Rider::Saddled
        | Rider::PrepareSpellFace(_)
        | Rider::PlayableFromExileFor(_)
        | Rider::SpellCopy
        | Rider::AbilityCopy
        | Rider::Dashed
        | Rider::EventPlayer(_)
        | Rider::EventAmount(_)
        | Rider::EventDeparture(..)
        | Rider::ChosenOpponent(_)
        | Rider::BattleProtector(_)
        | Rider::Escaped
        | Rider::ExhaustedAbility(_)
        | Rider::EntrySacrifices(_)
        | Rider::SecondChosenSubtype(_)
        | Rider::ChosenNumber { .. }
        | Rider::ChosenName { .. }
        | Rider::RulesAcquisition(_)
        | Rider::HideawayHost(_)
        | Rider::ExileVisibility { .. }
        | Rider::EntryCopyApplied
        | Rider::EntryTappingApplied
        | Rider::RevealedInHandCantPlayUntilTurn
        | Rider::NotedCounter(..)
        | Rider::Mutating
        | Rider::MergedToken(_) => None,
    }
}
