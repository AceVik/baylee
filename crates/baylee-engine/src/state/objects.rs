//! Objects: timestamps, faces and doors, attachments, the stack, face-down reveals, token cleanup, lookups and last known information.

use super::{GameState, filter_reads_board_state, modifier_reads_combat};
use crate::event::GameEvent;
use crate::object::{GameObject, ObjectKind};
use crate::zone::{Zone, ZoneLocation, ZonePosition};
use baylee_cards_dsl::CardDef;
use baylee_core::ids::ObjectId;
use std::sync::Arc;

impl GameState {
    /// The game's one clock, monotonic: every object timestamp
    /// ([`GameObject::timestamp`], CR 613.7), every continuous effect's, and
    /// the moments [`GameObject::controlled_since`] and
    /// [`Player::turn_start_timestamp`] compare for summoning sickness
    /// (CR 302.6) are read off it.
    pub fn next_timestamp(&mut self) -> u64 {
        self.timestamp += 1;
        self.timestamp
    }

    /// CR 302.6: a creature must have been controlled continuously since
    /// its controller's most recent turn began, so *any* control change
    /// makes it summoning-sick again — including the one at end of turn
    /// that hands a stolen creature back.
    ///
    /// Only [`GameObject::controlled_since`] moves. A change of control is
    /// not one of the events CR 613.7 gives an object a new timestamp for,
    /// so the order its static abilities' effects apply in stays as it was.
    #[doc(hidden)]
    pub fn restart_summoning_sickness(&mut self, id: ObjectId) {
        let ts = self.next_timestamp();
        if let Some(obj) = self.object_mut(id) {
            obj.controlled_since = ts;
        }
        self.learn_hideaway_cards(id);
    }

    /// Turns a permanent over (CR 701.27) and says so in the journal.
    ///
    /// [`Self::switch_face`] is the other half of this and stays silent,
    /// because its callers are not transforms: choosing which face of a
    /// modal card to cast or to play as a land puts a card onto the stack
    /// or the battlefield face up (CR 712.8b for the cast, CR 712.9 for
    /// the land play) — nothing turned over, and a
    /// "transformed" entry there would fire every trigger that watches for
    /// one. Anything that turns an existing permanent over comes here.
    ///
    /// Returns whether the face actually changed, which is what the
    /// daybound/nightbound fixpoint step reads: a permanent already showing
    /// the face it should show is not progress, and reporting it as such
    /// would keep the fixpoint spinning.
    ///
    /// The permanent stays the object it was, and every effect that applied
    /// to it goes on applying (CR 712.15). What its own static abilities and
    /// replacement effects did ends here, because those abilities were the
    /// face that turned away and the permanent no longer has them
    /// (CR 604.2); the next scan (`Engine::sync_static_effects`) registers
    /// what the new face prints, as [`Self::set_doors`] has it for a Room.
    /// Without this, Dowsing Dagger's "equipped creature gets +2/+1" went
    /// on applying from Lost Vale.
    ///
    /// The other face coming up without the game action is
    /// [`Self::turn_over`], which this is built on.
    pub fn transform(&mut self, id: ObjectId, def: &CardDef, face: usize) -> bool {
        // CR 701.27d / CR 712.16: only double-faced cards transform (not flip
        // cards, omens, adventures, meld cards, melded permanents, or
        // face-down or phased-out permanents); see `CardDef::can_transform`.
        if !def.can_transform() {
            return false;
        }
        if self.is_melded_permanent(id)
            || !self.object(id).is_some_and(|o| {
                o.zone == Zone::Battlefield
                    && !o.status.contains(crate::object::Status::PHASED_OUT)
                    && !o.status.contains(crate::object::Status::FACE_DOWN)
            })
        {
            return false;
        }
        let face = face.min(def.faces.len() - 1);
        if !self.turn_over(id, def, face) {
            return false;
        }
        // CR 613.7g: a new timestamp, which the new face's static abilities
        // take as the next scan registers them (CR 613.7a), so they apply
        // after every effect created while the other face was up. Who
        // controls it has not changed (CR 712.15), so `controlled_since`
        // stays and it is no more summoning-sick than it was (CR 302.6).
        let ts = self.next_timestamp();
        if let Some(obj) = self.object_mut(id) {
            obj.timestamp = ts;
        }
        self.journal.record(GameEvent::Transformed {
            object: id,
            face: face as u8,
        });
        true
    }

    /// Puts face `face` of a permanent up in place of the one it shows, and
    /// ends what the face turning away did: the continuous effects of its
    /// static abilities and its replacement effects, which the next scan
    /// (`Engine::sync_static_effects`) registers for the new face (CR 604.2).
    /// Returns whether the face changed.
    ///
    /// This is not the game action of transforming, and it records nothing
    /// and stamps nothing. Two things need it bare: [`Self::transform`],
    /// which adds the stamp and the journal entry, and a permanent that
    /// enters with its back face up (CR 712.11a, daybound at night,
    /// CR 702.145b). Such a permanent was never turned over (CR 701.27a
    /// transforms a *permanent*), so it has not transformed: a "transforms
    /// into" trigger (CR 701.27f) must not see it, the log must not say it
    /// did, and its timestamp is the one it took as it entered (CR 613.7d).
    /// It still has the front face's statics to lose, because the engine
    /// registers them for every arrival before the scan that turns it over.
    pub fn turn_over(&mut self, id: ObjectId, def: &CardDef, face: usize) -> bool {
        let face = face.min(def.faces.len() - 1);
        if self
            .object(id)
            .is_none_or(|o| o.face_index as usize == face)
        {
            return false;
        }
        self.effects.remove_where(|fx| {
            fx.source == Some(id) && fx.origin == crate::effects::EffectOrigin::Static
        });
        self.replacement_rules.retain(|r| r.source != id);
        self.switch_face(id, def, face);
        true
    }

    /// Puts face `face` of a double-faced token permanent up in place of the
    /// one it shows (`CR 701.27a`, `CR 712.7`, `CR 111.10i`). Returns whether
    /// the face changed.
    pub fn turn_over_token(&mut self, id: ObjectId, face: u8) -> bool {
        let Some((token, current_face)) = self.object(id).and_then(|o| {
            o.card
                .is_none()
                .then_some(o.token?)
                .filter(|t| t.back_face.is_some())
                .map(|t| (t, o.face_index))
        }) else {
            return false;
        };
        let face = face.min(1);
        if current_face == face {
            return false;
        }
        self.effects.remove_where(|fx| {
            fx.source == Some(id) && fx.origin == crate::effects::EffectOrigin::Static
        });
        self.replacement_rules.retain(|r| r.source != id);
        let new_base = self.token_base(token.face(face), None);
        if let Some(obj) = self.object_mut(id) {
            obj.face_index = face;
            obj.base = new_base;
            obj.cache.clear();
        }
        self.characteristics_generation = u64::MAX;
        true
    }

    /// Transforms a double-faced token permanent in place (`CR 701.27a,c`,
    /// `CR 613.7g`, `CR 712.15`), keeping its object identity, counters,
    /// status, and `controlled_since`, stamping a new `timestamp`, and
    /// recording `GameEvent::Transformed`.
    pub fn transform_token(&mut self, id: ObjectId, face: u8) -> bool {
        if !self.object(id).is_some_and(|o| {
            o.zone == Zone::Battlefield
                && !o.status.contains(crate::object::Status::PHASED_OUT)
                && !o.status.contains(crate::object::Status::FACE_DOWN)
        }) {
            return false;
        }
        let face = face.min(1);
        if !self.turn_over_token(id, face) {
            return false;
        }
        let ts = self.next_timestamp();
        if let Some(obj) = self.object_mut(id) {
            obj.timestamp = ts;
        }
        self.journal
            .record(GameEvent::Transformed { object: id, face });
        true
    }

    /// Attach an Aura or Equipment to a new permanent. A new attachment
    /// gives it and its static effects a new timestamp (CR 613.7a, 613.7e).
    /// Reattaching to the same object does nothing (CR 701.3b).
    #[doc(hidden)]
    pub fn attach(&mut self, source: ObjectId, host: ObjectId) {
        let Some(object) = self.object(source) else {
            return;
        };
        if object.attached_to == Some(host) {
            return;
        }
        let on_battlefield = object.zone == Zone::Battlefield;
        if on_battlefield
            && object
                .characteristics()
                .subtypes
                .contains(baylee_core::generated::subtypes::enchantment::AURA)
            && !crate::permanent_guards::permits_new_enchantment(self, host, source)
        {
            return;
        }
        let timestamp = on_battlefield.then(|| self.next_timestamp());
        if let Some(object) = self.object_mut(source) {
            object.set_attached_player(None);
            object.attached_to = Some(host);
            if let Some(timestamp) = timestamp {
                object.timestamp = timestamp;
            }
        }
        if let Some(timestamp) = timestamp {
            self.effects.retimestamp_statics(source, timestamp);
        }
        self.invalidate_projections();
    }

    /// Attach an Aura to a player ("enchant player" / Curses, CR 303.4b). A new
    /// attachment on the battlefield gives the Aura and its static effects a new
    /// timestamp (CR 613.7a, CR 613.7e); reattaching to the same player does
    /// nothing (CR 701.3b).
    #[doc(hidden)]
    pub fn attach_to_player(&mut self, source: ObjectId, player: baylee_core::ids::PlayerId) {
        let Some(object) = self.object(source) else {
            return;
        };
        if object.attached_player() == Some(player) {
            return;
        }
        if self.has_left(player) {
            return;
        }
        let on_battlefield = object.zone == Zone::Battlefield;
        let timestamp = on_battlefield.then(|| self.next_timestamp());
        if let Some(object) = self.object_mut(source) {
            object.set_attached_player(Some(player));
            if let Some(timestamp) = timestamp {
                object.timestamp = timestamp;
            }
        }
        if let Some(timestamp) = timestamp {
            self.effects.retimestamp_statics(source, timestamp);
        }
        self.invalidate_projections();
    }

    /// Switches an object to another face of its card (MDFC cast/land
    /// play, CR 712.8b and CR 712.12): rebuilds base characteristics from
    /// the face and
    /// invalidates the layered projection cache.
    ///
    /// This is the mechanism, not the rules action — see [`Self::transform`]
    /// for the one that journals.
    pub fn switch_face(&mut self, id: ObjectId, def: &CardDef, face: usize) {
        let face = face.min(def.faces.len() - 1);
        let name = self.names.intern(def.faces[face].name);
        let base = crate::object::Characteristics::from_face(def, face, name);
        if let Some(obj) = self.object_mut(id) {
            obj.face_index = face as u8;
            obj.base = Arc::new(base);
            obj.cache.clear();
        }
        // The projection is now stale for this object; the next refresh
        // has to rebuild it even if no effect was added or removed.
        self.characteristics_generation = u64::MAX;
    }

    /// Gives every split card off the stack its left half's characteristics,
    /// as a build from before the split boundary read it, where this one
    /// reads both halves ([`Self::rest_as_card`]). For a replay audit's hash
    /// alone (`Engine::snapshot_hash_reading_split_cards_by_left_half`), on a
    /// copy of the state: nothing in a game calls it.
    #[doc(hidden)]
    pub fn read_split_cards_by_left_half(&mut self) {
        use crate::state::CardLookup;
        let split: Vec<(ObjectId, &'static CardDef)> = self
            .arena
            .iter()
            .filter(|(_, o)| o.zone != Zone::Stack && o.is_card())
            .filter_map(|(id, o)| {
                let def = self.bases.card(o.card?.index)?;
                def.is_split_spell_card().then_some((id, def))
            })
            .collect();
        for (id, def) in split {
            if self.object(id).is_some() {
                let name = self.names.intern(def.faces[0].name);
                let left = Arc::new(crate::object::Characteristics::from_face(def, 0, name));
                if let Some(obj) = self.object_mut(id) {
                    if obj.zone == Zone::Battlefield && obj.doors.is_room() {
                        obj.original_base = Some(Arc::clone(&left));
                    } else {
                        obj.base = Arc::clone(&left);
                        obj.cache.clear();
                    }
                }
                // The shared base too, which no hash reads, so that the copy
                // is that build's state throughout.
                Arc::make_mut(&mut self.bases).cards.insert(def.index, left);
            }
        }
        // And the incarnations kept for a source named after it moved on
        // (`damage_sources`), each as it was when it left.
        for kept in &mut self.damage_sources {
            let Some(def) = kept
                .card
                .filter(|_| kept.zone != Zone::Stack)
                .and_then(|card| self.bases.card(card.index))
                .filter(|def| def.is_split_spell_card())
            else {
                continue;
            };
            let object = Arc::make_mut(kept);
            let name = object.base.name;
            object.base = Arc::new(crate::object::Characteristics::from_face(def, 0, name));
            object.cache.clear();
        }
    }

    /// Gives a card back the characteristics it has off the stack and the
    /// battlefield (CR 400.7): its front face, and for a split card whose
    /// halves are spells both halves combined (CR 709.4,
    /// `Characteristics::of_card`). The shared base the game was set up with
    /// is the same value, and is reused when there is one.
    pub fn rest_as_card(&mut self, id: ObjectId, def: &CardDef) {
        if !crate::object::reads_both_halves(def) {
            self.switch_face(id, def, 0);
            return;
        }
        let shared = self
            .object(id)
            .and_then(|o| o.card)
            .and_then(|card| self.bases.cards.get(&card.index))
            .map(Arc::clone);
        let whole = shared.unwrap_or_else(|| {
            let name = self.names.intern(def.faces[0].name);
            Arc::new(crate::object::Characteristics::of_card(def, name))
        });
        if let Some(obj) = self.object_mut(id) {
            obj.face_index = 0;
            obj.base = whole;
            obj.cache.clear();
        }
        self.characteristics_generation = u64::MAX;
    }

    /// Gives a permanent with a shared type line the unlocked designations
    /// `unlocked` names (bit 0 the left half, bit 1 the right; CR 709.5c),
    /// and the characteristics they leave it: the name, mana cost and rules
    /// text of its unlocked halves only (CR 709.5), and the shared types
    /// either way (CR 709.5a). The rules text is read off the doors
    /// (`CardDef::door_abilities`); this writes the rest.
    ///
    /// One half unlocked is that half's face ([`Self::switch_face`]). Both
    /// is the left face with both halves' mana cost, so both colours and
    /// their sum for a mana value, and both halves' keywords; the name stays
    /// the left half's, because an object here has one name and such a Room
    /// has two (CR 709.4a), and nothing in the pool reads a Room's name.
    /// Neither is the left face with no name, no mana cost, no colour and no
    /// keyword. Keywords are rules text, so each state sets them from the
    /// halves it has and never from the card-level fallback a front face
    /// reads (`CardDef::keywords_for_face`). The printed front is set aside
    /// in `original_base`, which the move off the battlefield restores: off
    /// the battlefield the card is the card again (CR 400.7).
    ///
    /// What the Room's statics and replacement rules did under the old
    /// doors ends here, and the next scan (`Engine::sync_static_effects`)
    /// registers what the new doors print. That scan only ever adds for a
    /// permanent still on the battlefield, and a Room put there uncast was
    /// scanned as its left half before it was given no doors: without this,
    /// Walk-In Closet's static outlived the door that prints it.
    pub fn set_doors(&mut self, id: ObjectId, def: &CardDef, unlocked: u8) {
        let unlocked = unlocked & 0b11;
        self.effects.remove_where(|fx| {
            fx.source == Some(id) && fx.origin == crate::effects::EffectOrigin::Static
        });
        self.replacement_rules.retain(|r| r.source != id);
        let whole = self
            .object(id)
            .and_then(|o| o.card)
            .and_then(|card| self.bases.cards.get(&card.index))
            .map_or_else(
                || {
                    let name = self.names.intern(def.faces[0].name);
                    Arc::new(crate::object::Characteristics::of_card(def, name))
                },
                Arc::clone,
            );
        self.switch_face(id, def, usize::from(unlocked == 0b10));
        let nameless = self.names.intern("");
        let Some(obj) = self.object_mut(id) else {
            return;
        };
        obj.doors = crate::object::Doors::room(unlocked);
        obj.original_base.get_or_insert(whole);
        let keywords = def
            .faces
            .iter()
            .take(2)
            .enumerate()
            .filter(|(half, _)| unlocked & (1 << half) != 0)
            .fold(baylee_cards_dsl::KeywordSet::EMPTY, |all, (_, f)| {
                all.union(f.keywords)
            });
        let c = obj.base_mut();
        c.keywords = keywords;
        match unlocked {
            0b11 => {
                let cost = def.faces[0].mana_cost.combine(&def.faces[1].mana_cost);
                c.mana_cost = cost;
                c.colors = cost
                    .colors()
                    .union(def.faces[0].color_indicator)
                    .union(def.faces[1].color_indicator);
            }
            0 => {
                c.name = nameless;
                c.mana_cost = baylee_core::mana::ManaCost::ZERO;
                c.colors = baylee_core::color::ColorSet::EMPTY;
            }
            _ => {}
        }
    }

    /// Forces the next [`GameState::refresh_characteristics`] to rebuild
    /// every projection, even though the effect table did not change.
    ///
    /// The generation compare that guards the refresh tracks *effects*, and
    /// counters are the other input the projection reads (CR 613.4c for
    /// +1/+1 and -1/-1, CR 122.1b for keyword counters). Changing a counter
    /// on a board where nothing else moved therefore leaves the projection
    /// stale — which is how an amass token, a 0/0 that is only alive because
    /// of the counter placed on it a moment later, used to die to state-based
    /// actions before anything ever recomputed its toughness. Anything that
    /// writes a counter, or puts a new permanent on the battlefield or a new
    /// spell on the stack ([`Self::put_new_spell_on_stack`]), calls this.
    pub const fn invalidate_projections(&mut self) {
        self.characteristics_generation = u64::MAX;
    }

    /// Puts `id`, a spell that has never been in a zone (a copy of a spell,
    /// CR 707.10), on top of the stack, and has the projection take it in.
    ///
    /// A spell on the stack is projected like a permanent (a creature spell
    /// under Maskwood Nexus is every creature type), and `move_object` is
    /// what invalidates for a spell that is cast. A copy is not moved: it is
    /// made there. Without the invalidation the copy answered its copied
    /// printed values until some unrelated change refreshed the board, and
    /// for everything that read it in between — a cast trigger, "target Ally
    /// spell" — the Nexus did not apply to it.
    pub fn put_new_spell_on_stack(&mut self, id: ObjectId) {
        self.zones
            .insert(id, ZoneLocation::Stack, ZonePosition::Top, true);
        self.invalidate_projections();
    }

    /// Whether this incarnation still awaits its event-driven face-up replacement.
    #[must_use]
    pub(crate) fn awaits_masked_reveal(&self, id: ObjectId) -> bool {
        self.object(id).is_some_and(|object| {
            object.zone == Zone::Battlefield
                && object.status.contains(crate::object::Status::FACE_DOWN)
                && object.riders.contains(&crate::object::Rider::Masked)
        })
    }

    /// Turn the same permanent face up; this is not a zone change or entry.
    pub(crate) fn reveal_masked(&mut self, id: ObjectId) -> bool {
        if !self.awaits_masked_reveal(id) {
            return false;
        }
        let object = self.object_mut(id).expect("checked permanent");
        object.status.remove(crate::object::Status::FACE_DOWN);
        if let Some(original) = object.original_base.take() {
            object.base = original;
        }
        object.cache.clear();
        self.invalidate_projections();
        self.journal.record(GameEvent::TurnedFaceUp { object: id });
        true
    }

    /// Manifests `id` under `controller`'s control as a face-down 2/2 colorless
    /// nameless creature permanent with no text or mana cost (CR 701.40a).
    ///
    /// If a rule or replacement prohibits the face-down permanent from entering
    /// the battlefield, nothing happens and `id` stays in its current zone
    /// unchanged (CR 701.40f).
    pub(crate) fn manifest_card(
        &mut self,
        controller: baylee_core::ids::PlayerId,
        id: ObjectId,
    ) -> bool {
        let Some(o) = self.object(id) else {
            return false;
        };
        if !o.is_card() {
            return false;
        }
        let prev_base = Arc::clone(&o.base);
        let prev_original_base = o.original_base.clone();
        let prev_status = o.status;
        let prev_controller = o.controller;
        let prev_kind = o.kind;
        let prev_face_index = o.face_index;
        let had_manifested = o.riders.contains(&crate::object::Rider::Manifested);
        let nameless = self.names.intern("");
        if let Some(obj) = self.object_mut(id) {
            obj.original_base = prev_original_base
                .clone()
                .or_else(|| Some(Arc::clone(&prev_base)));
            obj.status.insert(crate::object::Status::FACE_DOWN);
            if !had_manifested {
                obj.riders.push(crate::object::Rider::Manifested);
            }
            obj.kind = ObjectKind::Permanent;
            obj.set_controller(controller);
            obj.face_index = 0;
            let c = obj.base_mut();
            c.name = nameless;
            c.mana_cost = baylee_core::mana::ManaCost::ZERO;
            c.colors = baylee_core::color::ColorSet::EMPTY;
            c.types = baylee_core::types::TypeSet::CREATURE;
            c.supertypes = baylee_core::types::SupertypeSet::EMPTY;
            c.subtypes = baylee_core::types::SubtypeSet::EMPTY;
            c.keywords = baylee_cards_dsl::KeywordSet::EMPTY;
            c.power = Some(2);
            c.toughness = Some(2);
            c.loyalty = None;
            c.produced_colors = baylee_core::color::ColorSet::EMPTY;
            c.produced_colorless = false;
            c.produced_chosen = false;
            obj.cache.clear();
        }
        self.set_tapped(id, false);
        let moved = self.move_object(
            id,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            crate::event::Cause::Effect,
        );
        let entered = moved.is_ok()
            && (self.object(id).is_some_and(|o| o.zone == Zone::Battlefield)
                || self.entries.contains(id));
        if !entered {
            if let Some(obj) = self.object_mut(id) {
                obj.base = prev_base;
                obj.original_base = prev_original_base;
                obj.status = prev_status;
                obj.set_controller(prev_controller);
                obj.kind = prev_kind;
                obj.face_index = prev_face_index;
                if !had_manifested {
                    obj.riders
                        .retain(|r| *r != crate::object::Rider::Manifested);
                }
                obj.cache.clear();
            }
            self.invalidate_projections();
            return false;
        }
        self.invalidate_projections();
        true
    }

    /// Flips a Kamigawa flip permanent in place (`CR 710.2–710.4`, `CR 613.7g`).
    ///
    /// Flipping is a one-way status change on the battlefield: once flipped,
    /// the permanent cannot flip again or become unflipped until it leaves the
    /// battlefield (`CR 710.4`). Keeps object identity, counters, damage,
    /// attachments, controller, and `controlled_since`; stamps a new timestamp.
    pub fn flip_permanent(&mut self, id: ObjectId, def: &CardDef) -> bool {
        if !def.is_flip_card() {
            return false;
        }
        let Some((is_copy, until_eot)) = self.object(id).and_then(|o| {
            if o.zone != Zone::Battlefield
                || o.status.contains(crate::object::Status::PHASED_OUT)
                || o.status.contains(crate::object::Status::FLIPPED)
                || o.status.contains(crate::object::Status::FACE_DOWN)
            {
                return None;
            }
            if o.own_abilities.is_none() {
                o.card
                    .is_some_and(|c| c.index == def.index)
                    .then_some((false, false))
            } else {
                // CR 707.2 / CR 710.2: a permanent copying a flip card acquires
                // its alternative (flipped) characteristics and flips to face 1
                // while preserving its underlying `original_base`.
                o.printed_face()
                    .is_some_and(|pf| pf.card() == def.index)
                    .then_some((true, o.own_abilities_until_eot))
            }
        }) else {
            return false;
        };
        self.effects.remove_where(|fx| {
            fx.source == Some(id) && fx.origin == crate::effects::EffectOrigin::Static
        });
        self.replacement_rules.retain(|r| r.source != id);
        if is_copy {
            let flipped_name = self.names.intern(def.faces[1].name);
            let flipped_base = Arc::new(crate::object::Characteristics::from_face(
                def,
                1,
                flipped_name,
            ));
            let acquisition = self.next_timestamp();
            let flipped_abilities = crate::object::AbilityList {
                acquisition,
                abilities: crate::copiable_abilities::AbilityDefs::Static(
                    def.abilities_for_face(1),
                ),
                printed: crate::object::PrintedFace::new(def.index, 1),
                token: None,
            };
            if until_eot {
                let copy_effect_ids: Vec<_> = self
                    .effects
                    .iter()
                    .filter(|fx| {
                        fx.layer == baylee_cards_dsl::Layer::Copy
                            && matches!(
                                fx.filter,
                                crate::effects::EffectFilter::ObjectIs(target, _) if target == id
                            )
                    })
                    .map(|fx| fx.id)
                    .collect();
                for (fx_id, snapshot) in &mut self.copy_snapshots {
                    if copy_effect_ids.contains(fx_id) {
                        *snapshot = Arc::clone(&flipped_base);
                    }
                }
            }
            if let Some(obj) = self.object_mut(id) {
                if !until_eot {
                    obj.base = flipped_base;
                }
                obj.take_abilities(flipped_abilities);
                obj.own_abilities_until_eot = until_eot;
                obj.cache.clear();
            }
        } else {
            self.switch_face(id, def, 1);
        }
        let ts = self.next_timestamp();
        if let Some(obj) = self.object_mut(id) {
            obj.status.insert(crate::object::Status::FLIPPED);
            obj.timestamp = ts;
        }
        if !def.faces[1]
            .types
            .contains(baylee_core::types::TypeSet::CREATURE)
        {
            self.combat.remove_from_combat(id);
        }
        self.invalidate_projections();
        true
    }

    /// Whether `id` is a melded permanent (`CR 701.42`, `CR 712.1`) rather than
    /// a mutated permanent (`CR 727.2`). Both carry `Rider::MergedComponent`, so
    /// melded permanents are distinguished by their combined melded back
    /// (`card.is_some() && token.is_some()`) or any meld-card component.
    #[must_use]
    pub(crate) fn is_melded_permanent(&self, id: ObjectId) -> bool {
        use crate::state::CardLookup as _;
        let Some(o) = self.object(id) else {
            return false;
        };
        let mut has_merged = false;
        for rider in &o.riders {
            if let crate::object::Rider::MergedComponent(comp_id) = *rider {
                has_merged = true;
                if self
                    .object(comp_id)
                    .and_then(|comp| comp.card)
                    .and_then(|c| self.bases.card(c.index))
                    .is_some_and(CardDef::is_meld_card)
                {
                    return true;
                }
            }
        }
        has_merged
            && ((o.card.is_some() && o.token.is_some())
                || o.card
                    .and_then(|c| self.bases.card(c.index))
                    .is_some_and(CardDef::is_meld_card))
    }

    /// Whether `id` is a double-faced card or token, a melded permanent, or a
    /// merged permanent containing any double-faced card or token component
    /// (`CR 712.1`, `CR 712.16`, `CR 727.2`).
    #[must_use]
    pub(crate) fn is_double_faced_object(&self, id: ObjectId) -> bool {
        use crate::state::CardLookup as _;
        let Some(o) = self.object(id) else {
            return false;
        };
        if self.is_melded_permanent(id) {
            return true;
        }
        if o.token.is_some_and(|t| t.back_face.is_some()) {
            return true;
        }
        if o.card
            .and_then(|c| self.bases.card(c.index))
            .is_some_and(CardDef::is_double_faced)
        {
            return true;
        }
        o.riders.iter().any(|rider| {
            let crate::object::Rider::MergedComponent(comp_id) = *rider else {
                return false;
            };
            self.object(comp_id).is_some_and(|comp| {
                comp.token.is_some_and(|t| t.back_face.is_some())
                    || comp
                        .card
                        .and_then(|c| self.bases.card(c.index))
                        .is_some_and(CardDef::is_double_faced)
            })
        })
    }

    /// Whether `id` is an eligible permanent to turn face down (`CR 708.2`,
    /// `CR 712.16`, `CR 727.2`). Double-faced cards (transforming DFCs, MDFCs,
    /// meld cards), double-faced tokens, melded permanents, and merged
    /// permanents containing any double-faced component cannot turn face down;
    /// mutated permanents composed solely of single-faced cards can.
    #[must_use]
    pub fn can_turn_face_down(&self, id: ObjectId) -> bool {
        let Some(o) = self.object(id) else {
            return false;
        };
        if o.zone != Zone::Battlefield
            || o.status.contains(crate::object::Status::PHASED_OUT)
            || o.status.contains(crate::object::Status::FACE_DOWN)
        {
            return false;
        }
        !self.is_double_faced_object(id)
    }

    /// Turns a face-up permanent on the battlefield face down with `spec`
    /// characteristics (`CR 708.2`, `CR 712.16`, `CR 613.7g`).
    ///
    /// Keeps object identity, counters, damage, controller, `controlled_since`,
    /// and any other status (including `Status::FLIPPED`); stamps a new
    /// timestamp and adds `Rider::TurnedFaceDown` so Disguise Ward {2} is
    /// never granted. If the permanent already has `original_base` (e.g. a
    /// permanent copy), its displaced copied base is preserved in
    /// `face_down_copied_bases` so turning it back face up restores the copy
    /// while leaving the battlefield still restores the printed card.
    pub fn turn_permanent_face_down(
        &mut self,
        id: ObjectId,
        spec: baylee_cards_dsl::FaceDownSpec,
    ) -> bool {
        if !self.can_turn_face_down(id) {
            return false;
        }
        self.effects.remove_where(|fx| {
            fx.source == Some(id) && fx.origin == crate::effects::EffectOrigin::Static
        });
        self.replacement_rules.retain(|r| r.source != id);

        let mut face_down_base = self.bare_base(super::NAMELESS);
        {
            let c = Arc::make_mut(&mut face_down_base);
            c.types = spec.types;
            c.subtypes = baylee_core::types::SubtypeSet::from_slice(spec.subtypes);
            c.power = spec.power;
            c.toughness = spec.toughness;
        }
        let Some((version, has_original, current_base)) = self
            .object(id)
            .map(|o| (o.version, o.original_base.is_some(), Arc::clone(&o.base)))
        else {
            return false;
        };
        let displaced_original = if has_original {
            self.face_down_copied_bases.retain(|(r, _)| r.object != id);
            self.face_down_copied_bases.push((
                baylee_core::ids::DamageSourceRef {
                    object: id,
                    version,
                },
                current_base,
            ));
            None
        } else {
            Some(current_base)
        };
        let ts = self.next_timestamp();
        let Some(object) = self.object_mut(id) else {
            return false;
        };
        if let Some(original) = displaced_original {
            object.original_base = Some(original);
        }
        object.base = face_down_base;
        object.status.insert(crate::object::Status::FACE_DOWN);
        if !object
            .riders
            .contains(&crate::object::Rider::TurnedFaceDown)
        {
            object.riders.push(crate::object::Rider::TurnedFaceDown);
        }
        object.timestamp = ts;
        object.cache.clear();
        if !spec.types.contains(baylee_core::types::TypeSet::CREATURE) {
            self.combat.remove_from_combat(id);
        }
        self.invalidate_projections();
        true
    }

    /// Turns a face-down permanent on the battlefield face up by effect
    /// (CR 708.8). If it is represented by an instant or sorcery card, its
    /// controller reveals it and leaves it face down without firing
    /// `TurnedFaceUp` triggers (CR 701.40g).
    pub(crate) fn turn_permanent_face_up(&mut self, id: ObjectId) -> bool {
        let Some(o) = self.object(id) else {
            return false;
        };
        if o.zone != Zone::Battlefield
            || o.status.contains(crate::object::Status::PHASED_OUT)
            || !o.status.contains(crate::object::Status::FACE_DOWN)
        {
            return false;
        }
        let version = o.version;
        let controller = o.controller;
        let copied_base_pos = self
            .face_down_copied_bases
            .iter()
            .position(|(r, _)| r.object == id && r.version == version);
        let non_permanent_spell = copied_base_pos
            .map(|pos| &self.face_down_copied_bases[pos].1)
            .or(o.original_base.as_ref())
            .is_some_and(|b| {
                b.types.intersects(
                    baylee_core::types::TypeSet::INSTANT
                        .union(baylee_core::types::TypeSet::SORCERY),
                )
            });
        if non_permanent_spell {
            self.journal.record(GameEvent::Revealed {
                player: controller,
                cards: vec![id],
            });
            return false;
        }
        let copied_base = copied_base_pos.map(|pos| self.face_down_copied_bases.swap_remove(pos).1);
        let ts = self.next_timestamp();
        let object = self.object_mut(id).expect("checked permanent");
        object.status.remove(crate::object::Status::FACE_DOWN);
        object.riders.retain(|r| {
            !matches!(
                r,
                crate::object::Rider::Manifested
                    | crate::object::Rider::Masked
                    | crate::object::Rider::TurnedFaceDown
            )
        });
        if let Some(original) = copied_base.or_else(|| object.original_base.take()) {
            object.base = original;
        }
        object.timestamp = ts;
        object.cache.clear();
        self.invalidate_projections();
        self.journal.record(GameEvent::TurnedFaceUp { object: id });
        true
    }

    /// Tells the projection that the board moved under it — something tapped,
    /// something began or stopped attacking.
    ///
    /// The gate rather than a bare [`Self::invalidate_projections`]:
    /// `refresh_characteristics` makes the same scan for cross-zone effects,
    /// and a full re-projection per land tap is the cost this refuses. On a
    /// board where no effect reads tap or attack status — nearly every board
    /// — this is one walk of a short table and nothing else.
    pub fn board_state_changed(&mut self) {
        if self.effects.may_read_board_state()
            && self.effects.iter().any(|fx| {
                matches!(fx.filter, crate::effects::EffectFilter::Dsl(f)
                    | crate::effects::EffectFilter::DslAbout(f, ..) if filter_reads_board_state(f))
                    || modifier_reads_combat(fx.modifier)
            })
        {
            self.invalidate_projections();
        }
    }

    /// Whether [`crate::zone::Zones::stack_projectable`] still agrees with
    /// the stack: same objects, abilities excluded.
    ///
    /// Read-only, and only ever called from a `debug_assert!` — it walks
    /// the stack, which is precisely the walk the subset exists to avoid.
    #[must_use]
    pub fn stack_projection_set_is_consistent(&self) -> bool {
        let mut expected: Vec<ObjectId> = self
            .zones
            .list(ZoneLocation::Stack)
            .iter()
            .copied()
            .filter(|id| {
                self.object(*id)
                    .is_some_and(|o| o.kind != ObjectKind::AbilityOnStack)
            })
            .collect();
        let mut actual = self.zones.stack_projectable().to_vec();
        expected.sort_unstable();
        actual.sort_unstable();
        expected == actual
    }

    /// Notes that `id` may now be a token outside the battlefield, so the
    /// next state-based-action pass looks at it (CR 704.5d).
    ///
    /// Deliberately over-inclusive: the caller checks only the two cheap
    /// facts it already has in hand (card-less, destination is neither the
    /// battlefield nor the stack) and [`crate::sba`] applies the real rule.
    /// A false positive costs one arena lookup; a false negative would be
    /// a token that never ceases to exist.
    pub(crate) fn watch_token_cleanup(&mut self, id: ObjectId) {
        self.token_cleanup.push(id);
    }

    /// Takes the pending cleanup candidates, leaving the buffer empty and
    /// allocated for reuse.
    pub(crate) fn take_token_cleanup(&mut self) -> Vec<ObjectId> {
        std::mem::take(&mut self.token_cleanup)
    }

    /// Returns the drained buffer so the next pass allocates nothing.
    pub(crate) fn return_token_cleanup(&mut self, mut buffer: Vec<ObjectId>) {
        if self.token_cleanup.is_empty() {
            buffer.clear();
            self.token_cleanup = buffer;
        }
    }

    /// Effective words of one ability, including token-defined quoted text.
    #[must_use]
    pub fn ability_text(&self, id: ObjectId, index: u32) -> crate::text_changes::TextChangeMap {
        let Some(object) = self.object(id) else {
            return crate::text_changes::TextChangeMap::IDENTITY;
        };
        let changed = self.text_changes.get(baylee_core::ids::DamageSourceRef {
            object: id,
            version: object.version,
        });
        if object.kind == ObjectKind::AbilityOnStack {
            return changed;
        }
        object
            .printed_abilities(self.bases.as_ref())
            .base_text(index as usize)
            .then(changed)
    }

    /// Whether the text of the ability `on_stack` has someone other than its
    /// controller choose its targets (`TargetReq::chosen_by_event_player`,
    /// `chosen_by_opponent`): Confusion in the Ranks, The Abyss, Preacher.
    /// An ability that is not on the stack, or a modal one still announcing
    /// its mode, has nobody's choice to read and answers `false`.
    #[must_use]
    pub fn targets_chosen_by_another(&self, on_stack: ObjectId) -> bool {
        use baylee_cards_dsl::{AbilityDef, TargetChooser, TargetReq};
        let foreign = |req: Option<TargetReq>| {
            req.is_some_and(|req| {
                matches!(
                    req.chooser,
                    TargetChooser::Opponent | TargetChooser::EventPlayer
                )
            })
        };
        let Some(object) = self.object(on_stack) else {
            return false;
        };
        if object.kind != ObjectKind::AbilityOnStack {
            return false;
        }
        if foreign(object.target_req) || foreign(object.second_target_req()) {
            return true;
        }
        let Some(loc) = object.ability else {
            return false;
        };
        if loc.index == baylee_core::ids::AbilityRef::SYNTHETIC
            || crate::choice::granted_slot(loc.index).is_some()
        {
            return false;
        }
        let abilities = object.own_abilities.as_ref().map_or_else(
            || {
                self.object(loc.source)
                    .map_or(crate::copiable_abilities::AbilityDefs::EMPTY, |source| {
                        source.printed_abilities(self.bases.as_ref())
                    })
            },
            crate::copiable_abilities::AbilityDefs::from,
        );
        let announcing = object.mode_index.is_none()
            && matches!(
                abilities.get(loc.index as usize),
                Some(AbilityDef::ModalTriggered { .. } | AbilityDef::ModalActivated { .. })
            );
        !announcing
            && (foreign(crate::object::ability_target_req(
                abilities.clone(),
                loc.index,
                object.mode_index,
            )) || foreign(crate::object::ability_second_target_req(
                abilities,
                loc.index,
                object.mode_index,
            )))
    }

    /// The current copiable rules of an admitted object, without a second registry.
    #[must_use]
    pub fn printed_ability_list(&self, id: ObjectId) -> Option<crate::object::AbilityList> {
        self.object(id)
            .map(|object| object.printed_ability_list(self.bases.as_ref()))
    }

    /// Object access.
    #[must_use]
    pub fn object(&self, id: ObjectId) -> Option<&GameObject> {
        self.arena.get(id)
    }

    /// The object an event was about, even if it has ceased to exist.
    ///
    /// The narrow door onto [`Self::ceased`], and it is deliberately not
    /// [`Self::object`]: an object that has ceased to exist is not in the
    /// game, and every rule that asks "is this here?" has to keep getting no
    /// for an answer. The callers are the ones asking a different question —
    /// "what was this, at the moment of the event?" — which is the trigger
    /// scan and nothing else (CR 111.7, CR 603.10a).
    #[must_use]
    pub fn object_or_departed(&self, id: ObjectId) -> Option<&GameObject> {
        self.arena
            .get(id)
            .or_else(|| self.ceased.iter().find(|o| o.id == id))
    }

    /// The characteristic-defining P/T `obj` applies itself, which it does
    /// wherever it is but on the battlefield ([`Self::printed_pt_cda`]).
    #[must_use]
    pub fn off_battlefield_pt_cda(&self, obj: &GameObject) -> Option<baylee_cards_dsl::Modifier> {
        if obj.zone == Zone::Battlefield {
            return None;
        }
        self.printed_pt_cda
            .iter()
            .find(|(card, _)| *card == obj.id)
            .map(|(_, modifier)| *modifier)
    }

    pub(super) fn defines_pt_off_battlefield(&self, obj: &GameObject) -> bool {
        self.off_battlefield_pt_cda(obj).is_some()
    }

    /// What `id` was as it last left the battlefield, if that is the last
    /// move it made ([`Self::ltb_characteristics`]).
    #[must_use]
    pub fn last_known_characteristics(
        &self,
        id: ObjectId,
    ) -> Option<&crate::object::Characteristics> {
        self.ltb_characteristics
            .iter()
            .find(|(other, _)| *other == id)
            .map(|(_, was)| was)
    }

    /// The exact incarnation dealing damage, current or retained as LKI.
    pub(crate) fn damage_source(&self, id: ObjectId, version: Option<u32>) -> Option<&GameObject> {
        self.object_or_departed(id)
            .filter(|obj| version.is_none_or(|v| v == obj.version))
            .or_else(|| {
                if self.source_memory.cache.damage_source_len == self.damage_sources.len()
                    && let Some(v) = version
                {
                    self.source_memory
                        .cache
                        .damage_source_index
                        .get(&baylee_core::ids::DamageSourceRef {
                            object: id,
                            version: v,
                        })
                        .and_then(|&idx| self.damage_sources.get(idx))
                        .map(|obj| &**obj)
                } else {
                    self.damage_sources
                        .iter()
                        .rev()
                        .find(|obj| obj.id == id && version == Some(obj.version))
                        .map(|obj| &**obj)
                }
            })
    }

    /// The battlefield as rules see it: phased-out permanents are treated
    /// as though they don't exist (CR 702.26b).
    #[must_use]
    pub fn battlefield_view(&self) -> Vec<ObjectId> {
        self.battlefield_seen().collect()
    }

    /// [`Self::battlefield_view`] without the allocation, for a walk that
    /// only counts or filters.
    pub fn battlefield_seen(&self) -> impl Iterator<Item = ObjectId> + '_ {
        // phasing: the one walk that filters phased-out permanents for
        // everybody else.
        self.zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .copied()
            .filter(|id| {
                self.object(*id)
                    .is_some_and(|o| !o.status.contains(crate::object::Status::PHASED_OUT))
            })
    }

    /// Mutable object access.
    #[must_use]
    pub fn object_mut(&mut self, id: ObjectId) -> Option<&mut GameObject> {
        self.source_memory.invalidate_object(id);
        self.arena.get_mut(id)
    }

    /// The semantic wording applicable to this continuous effect's definition.
    /// Layer-six grants follow their grantor, never the receiving object's text.
    #[must_use]
    pub fn effect_text(
        &self,
        effect: &crate::effects::ContinuousEffect,
    ) -> crate::text_changes::TextChangeMap {
        if let Some((_, origin)) = self
            .effect_text_overrides
            .iter()
            .find(|(id, _)| *id == effect.id)
        {
            return origin.resolve(&self.text_changes);
        }
        if effect.origin == crate::effects::EffectOrigin::Static
            && let Some(source) = effect
                .source
                .and_then(|source| self.source_identity(source))
        {
            return self.text_changes.get(source);
        }
        crate::text_changes::TextChangeMap::IDENTITY
    }

    /// Active soulbond partner of `id`, if both permanents are currently
    /// non-phased-out creatures on the battlefield under the same player's
    /// control with reciprocal [`crate::object::Rider::PairedWith`] riders
    /// (CR 702.95b, CR 702.95d, CR 702.95e, CR 702.26b).
    #[must_use]
    pub fn paired_partner(&self, id: ObjectId) -> Option<ObjectId> {
        let a = self.object(id)?;
        if a.zone != Zone::Battlefield
            || a.status.contains(crate::object::Status::PHASED_OUT)
            || !a
                .characteristics()
                .types
                .contains(baylee_core::types::TypeSet::CREATURE)
        {
            return None;
        }
        let (partner_id, partner_version) = a.paired_with_raw()?;
        if partner_id == id {
            return None;
        }
        let b = self.object(partner_id)?;
        if b.zone != Zone::Battlefield
            || b.status.contains(crate::object::Status::PHASED_OUT)
            || b.version != partner_version
            || b.controller != a.controller
            || !b
                .characteristics()
                .types
                .contains(baylee_core::types::TypeSet::CREATURE)
            || b.paired_with_raw() != Some((id, a.version))
        {
            return None;
        }
        Some(partner_id)
    }

    /// Whether `id` (at optional `version`) is currently a non-phased-out,
    /// unpaired creature on the battlefield controlled by `player`.
    #[must_use]
    pub(crate) fn is_unpaired_creature_controlled_by(
        &self,
        id: ObjectId,
        version: Option<u32>,
        player: baylee_core::ids::PlayerId,
    ) -> bool {
        let Some(obj) = self.object(id) else {
            return false;
        };
        obj.zone == Zone::Battlefield
            && !obj.status.contains(crate::object::Status::PHASED_OUT)
            && version.is_none_or(|v| obj.version == v)
            && obj.controller == player
            && obj
                .characteristics()
                .types
                .contains(baylee_core::types::TypeSet::CREATURE)
            && self.paired_partner(id).is_none()
    }

    /// Whether `a` and `b` can currently become paired by `player`'s soulbond
    /// ability (CR 702.95a, CR 702.95c, CR 702.95d).
    #[must_use]
    pub(crate) fn can_soulbond_pair(
        &self,
        player: baylee_core::ids::PlayerId,
        a: ObjectId,
        a_version: Option<u32>,
        b: ObjectId,
        b_version: Option<u32>,
    ) -> bool {
        a != b
            && self.is_unpaired_creature_controlled_by(a, a_version, player)
            && self.is_unpaired_creature_controlled_by(b, b_version, player)
    }

    /// Breaks any soulbond pairing involving `id` on both sides (CR 702.95e),
    /// returning `true` if a rider was removed.
    pub(crate) fn unpair_creature(&mut self, id: ObjectId) -> bool {
        let direct_partner = self
            .object(id)
            .and_then(crate::object::GameObject::paired_with_raw)
            .map(|(partner, _)| partner);
        let mut to_clear = smallvec::SmallVec::<[ObjectId; 2]>::new();
        if self.object(id).is_some_and(|o| {
            o.riders
                .iter()
                .any(|r| matches!(r, crate::object::Rider::PairedWith(..)))
        }) {
            to_clear.push(id);
        }
        if let Some(partner) = direct_partner
            && !to_clear.contains(&partner)
        {
            to_clear.push(partner);
        }
        // phasing: unpairing also clears PairedWith riders on phased-out partners (CR 702.95e).
        for &bf_id in self.zones.list(ZoneLocation::Battlefield) {
            if !to_clear.contains(&bf_id)
                && self.object(bf_id).is_some_and(|o| {
                    o.riders
                        .iter()
                        .any(|r| matches!(*r, crate::object::Rider::PairedWith(p, _) if p == id))
                })
            {
                to_clear.push(bf_id);
            }
        }
        let mut changed = false;
        for clear_id in to_clear {
            if let Some(obj) = self.object_mut(clear_id) {
                let before = obj.riders.len();
                obj.riders
                    .retain(|r| !matches!(r, crate::object::Rider::PairedWith(..)));
                changed |= obj.riders.len() != before;
            }
        }
        if changed {
            self.invalidate_projections();
        }
        changed
    }

    /// Pairs `a` and `b` via soulbond if still legal (CR 702.95a–d), refreshing
    /// projections immediately so continuous grants on the pair take effect.
    pub(crate) fn pair_creatures(
        &mut self,
        player: baylee_core::ids::PlayerId,
        a: ObjectId,
        a_version: Option<u32>,
        b: ObjectId,
        b_version: Option<u32>,
    ) -> bool {
        if !self.can_soulbond_pair(player, a, a_version, b, b_version) {
            return false;
        }
        let _ = self.unpair_creature(a);
        let _ = self.unpair_creature(b);
        let (Some(va), Some(vb)) = (
            self.object(a).map(|o| o.version),
            self.object(b).map(|o| o.version),
        ) else {
            return false;
        };
        if let Some(oa) = self.object_mut(a) {
            oa.riders.push(crate::object::Rider::PairedWith(b, vb));
        }
        if let Some(ob) = self.object_mut(b) {
            ob.riders.push(crate::object::Rider::PairedWith(a, va));
        }
        self.invalidate_projections();
        self.refresh_characteristics();
        true
    }

    /// Unpairs any battlefield permanents whose soulbond pairing has ceased to
    /// hold because either stopped being a creature, left the battlefield, or
    /// changed controller (CR 702.95e). Leaves pairings with a phased-out
    /// permanent dormant while phased out until it phases in (CR 702.26b).
    pub(crate) fn prune_broken_soulbond_pairs(&mut self) -> bool {
        let mut broken = smallvec::SmallVec::<[ObjectId; 2]>::new();
        // phasing: walks raw battlefield while keeping phased-out pairings dormant (CR 702.26b, CR 702.95e).
        for &id in self.zones.list(ZoneLocation::Battlefield) {
            let Some(a) = self.object(id) else {
                continue;
            };
            if a.status.contains(crate::object::Status::PHASED_OUT) {
                continue;
            }
            let Some((partner_id, partner_version)) = a.paired_with_raw() else {
                continue;
            };
            let valid = partner_id != id
                && a.characteristics()
                    .types
                    .contains(baylee_core::types::TypeSet::CREATURE)
                && self.object(partner_id).is_some_and(|b| {
                    b.zone == Zone::Battlefield
                        && b.version == partner_version
                        && (b.status.contains(crate::object::Status::PHASED_OUT)
                            || (b.controller == a.controller
                                && b.characteristics()
                                    .types
                                    .contains(baylee_core::types::TypeSet::CREATURE)
                                && b.paired_with_raw() == Some((id, a.version))))
                });
            if !valid && !broken.contains(&id) {
                broken.push(id);
            }
        }
        let mut changed = false;
        for id in broken {
            changed |= self.unpair_creature(id);
        }
        changed
    }
}
