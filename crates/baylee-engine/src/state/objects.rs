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
    pub(crate) fn restart_summoning_sickness(&mut self, id: ObjectId) {
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
    /// or the battlefield face up (CR 712.11b for the cast, CR 712.12 for
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
    /// to it goes on applying (CR 712.18). What its own static abilities and
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
        let face = face.min(def.faces.len() - 1);
        if !self.turn_over(id, def, face) {
            return false;
        }
        // CR 613.7g: a new timestamp, which the new face's static abilities
        // take as the next scan registers them (CR 613.7a), so they apply
        // after every effect created while the other face was up. Who
        // controls it has not changed (CR 712.18), so `controlled_since`
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
    /// enters with its back face up (CR 712.14a, daybound at night,
    /// CR 702.145b). Such a permanent was never turned over (CR 701.27a
    /// transforms a *permanent*), so it has not transformed: a "transforms
    /// into" trigger (CR 701.27e) must not see it, the log must not say it
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

    /// Attach an Aura or Equipment to a new permanent. A new attachment
    /// gives it and its static effects a new timestamp (CR 613.7a, 613.7e).
    /// Reattaching to the same object does nothing (CR 701.3b).
    pub(crate) fn attach(&mut self, source: ObjectId, host: ObjectId) {
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

    /// Switches an object to another face of its card (MDFC cast/land
    /// play, CR 712.11b and CR 712.12): rebuilds base characteristics from
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
        let front = {
            let name = self.names.intern(def.faces[0].name);
            Arc::new(crate::object::Characteristics::from_face(def, 0, name))
        };
        self.switch_face(id, def, usize::from(unlocked == 0b10));
        let nameless = self.names.intern("");
        let Some(obj) = self.object_mut(id) else {
            return;
        };
        obj.doors = crate::object::Doors::room(unlocked);
        obj.original_base.get_or_insert(front);
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
}
