//! Exact object references for source decisions (CR 609.7a).
//!
//! Arena handles survive zone changes; rules objects do not. References are
//! captured before a move, then retained only while a live rule refers to them.

use std::collections::{BTreeMap, BTreeSet};

mod event_readers;
mod subject_readers;

use baylee_cards_dsl::{Filter, Modifier};
use baylee_core::ids::{DamageSourceRef, EffectId, ObjectId, PlayerId, SourceChoiceId};

use crate::effects::EffectFilter;
use crate::object::{GameObject, ObjectKind, Rider};
use crate::prevention::{ShieldKind, Shielded};
use crate::state::{DelayedAction, DelayedWhen, GameState};
use crate::zone::{Zone, ZoneLocation};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Slot {
    Source,
    Event,
    Attachment,
    Target(u32),
    Second(u32),
    Tapped,
    Sacrificed,
    MovedSource,
    Linked(u32),
}

/// Targets announced before any payment can change their incarnations.
#[derive(Clone, Debug, Default, Hash)]
pub(crate) struct TargetReferences {
    pub(crate) first: Vec<DamageSourceRef>,
    pub(crate) second: Vec<DamageSourceRef>,
}

impl TargetReferences {
    pub(crate) fn fingerprint(&self) -> u64 {
        let mut hash = 0_u64;
        for group in [&self.first, &self.second] {
            hash = hash.wrapping_mul(31).wrapping_add(group.len() as u64);
            for reference in group {
                hash = hash
                    .wrapping_mul(31)
                    .wrapping_add(u64::from(reference.object.slot()));
                hash = hash
                    .wrapping_mul(31)
                    .wrapping_add(u64::from(reference.object.generation()));
                hash = hash
                    .wrapping_mul(31)
                    .wrapping_add(u64::from(reference.version));
            }
        }
        hash
    }
}

#[derive(Clone, Debug, Default, Hash)]
pub(crate) struct SourceMemory {
    pub(crate) next_choice: u64,
    pub(crate) stack: BTreeMap<DamageSourceRef, BTreeMap<Slot, DamageSourceRef>>,
    /// Announcement decisions survive with the exact spell/ability being copied.
    pub(crate) divisions: BTreeMap<DamageSourceRef, Vec<(DamageSourceRef, u32)>>,
    effects: BTreeMap<EffectId, DamageSourceRef>,
    event_readers: BTreeSet<DamageSourceRef>,
    /// Written only by a successful permanent-spell resolution.
    resolved: BTreeMap<DamageSourceRef, DamageSourceRef>,
    resolving: BTreeSet<DamageSourceRef>,
    on_resolution: BTreeMap<DamageSourceRef, (PlayerId, &'static [baylee_cards_dsl::Effect])>,
    linked: BTreeMap<DamageSourceRef, DamageSourceRef>,
}

fn identity(obj: &GameObject) -> DamageSourceRef {
    DamageSourceRef {
        object: obj.id,
        version: obj.version,
    }
}

impl GameState {
    /// Exact current or last-known incarnation, without consulting a newer one.
    #[must_use]
    pub fn source_object(&self, source: DamageSourceRef) -> Option<&GameObject> {
        self.damage_source(source.object, Some(source.version))
    }

    pub(crate) fn source_identity(&self, id: ObjectId) -> Option<DamageSourceRef> {
        self.object_or_departed(id).map(identity)
    }

    pub(crate) fn recorded_ability_source(&self, holder: ObjectId) -> Option<DamageSourceRef> {
        self.source_identity(holder)
            .and_then(|r| self.source_memory.stack.get(&r))
            .and_then(|refs| refs.get(&Slot::Source))
            .copied()
    }

    /// Stack objects publicly referring to this exact source incarnation.
    #[must_use]
    pub fn source_referenced_by(&self, source: DamageSourceRef) -> Vec<ObjectId> {
        self.source_memory
            .stack
            .iter()
            .filter_map(|(holder, refs)| {
                (self
                    .object(holder.object)
                    .is_some_and(|o| o.zone == Zone::Stack && o.version == holder.version)
                    && refs.iter().any(|(slot, r)| {
                        *r == source
                            && (*slot != Slot::Event
                                || self.source_memory.event_readers.contains(holder))
                    }))
                .then_some(holder.object)
            })
            .collect()
    }

    pub(crate) fn next_source_choice(&mut self) -> SourceChoiceId {
        let id = self.source_memory.next_choice;
        self.source_memory.next_choice =
            id.checked_add(1).expect("source choice identity exhausted");
        SourceChoiceId::new(id)
    }

    /// Capture before removing a referenced object from its zone or arena.
    pub(crate) fn remember_damage_source(&mut self, id: ObjectId) {
        self.capture_source_references();
        if let Some(obj) = self.object(id)
            && !self
                .damage_sources
                .iter()
                .any(|old| identity(old) == identity(obj))
        {
            self.damage_sources.push(obj.clone());
        }
    }

    /// Bare target fields are recorded before their referenced object changes
    /// zones. Explicit source/event riders already carry their incarnation.
    pub(crate) fn capture_source_references(&mut self) {
        for (_, card) in self.arena.iter().filter(|(_, o)| o.zone == Zone::Exile) {
            let card_ref = identity(card);
            if let Some(host) = card.riders.iter().find_map(|r| match r {
                Rider::Linked { host, .. } => Some(*host),
                _ => None,
            }) && !self.source_memory.linked.contains_key(&card_ref)
                && let Some(host_ref) = self.source_identity(host)
            {
                self.source_memory.linked.insert(card_ref, host_ref);
            }
        }
        for id in self.zones.list(ZoneLocation::Stack).clone() {
            let Some(obj) = self.object(id) else { continue };
            let holder = identity(obj);
            let mut refs = self
                .source_memory
                .stack
                .get(&holder)
                .cloned()
                .unwrap_or_default();
            let mut slots = Vec::new();
            if let Some(ability) = obj.ability {
                let version = obj.riders.iter().find_map(|r| match r {
                    Rider::AbilitySourceVersion(v) => Some(*v),
                    _ => None,
                });
                slots.push((Slot::Source, ability.source, version));
            }
            if let Some(event) = obj.event_object {
                let version = obj.riders.iter().find_map(|r| match r {
                    Rider::EventObjectIdentity(v, _) => Some(*v),
                    _ => None,
                });
                slots.push((Slot::Event, event, version));
            }
            if let Some(host) = obj.riders.iter().find_map(|r| match r {
                Rider::SourceAttachmentLki(id) => Some(*id),
                _ => None,
            }) {
                let version = obj.riders.iter().find_map(|r| match r {
                    Rider::SourceAttachmentVersion(v) => Some(*v),
                    _ => None,
                });
                slots.push((Slot::Attachment, host, version));
            }
            for (i, &target) in obj.targets.iter().enumerate() {
                slots.push((
                    Slot::Target(u32::try_from(i).expect("target index")),
                    target,
                    None,
                ));
            }
            {
                for (i, &target) in obj.second_targets().iter().enumerate() {
                    slots.push((
                        Slot::Second(u32::try_from(i).expect("target index")),
                        target,
                        None,
                    ));
                }
            }
            if let Some((id, version)) = obj.paid.as_ref().and_then(|p| p.tapped) {
                slots.push((Slot::Tapped, id, Some(version)));
            }
            if let Some((id, version)) = obj.paid.as_ref().and_then(|p| p.sacrificed) {
                slots.push((Slot::Sacrificed, id, Some(version)));
            }
            refs.retain(|slot, _| {
                matches!(slot, Slot::Linked(_) | Slot::MovedSource)
                    || slots.iter().any(|(s, _, _)| s == slot)
            });
            for (slot, object, version) in slots {
                let reference = version
                    .map(|version| DamageSourceRef { object, version })
                    .or_else(|| refs.get(&slot).copied().filter(|r| r.object == object))
                    .or_else(|| self.source_identity(object));
                if let Some(reference) = reference {
                    refs.insert(slot, reference);
                }
            }
            if let Some((_, shares)) = self.divided.iter().find(|(id, _)| *id == holder.object) {
                self.source_memory.divisions.insert(holder, shares.clone());
            }
            self.source_memory.stack.insert(holder, refs);
        }
        for effect in self.effects.iter().filter(|e| is_prevention(&e.modifier)) {
            if let Some(source) = effect.source
                && !self.source_memory.effects.contains_key(&effect.id)
                && let Some(reference) = self.source_identity(source)
            {
                self.source_memory.effects.insert(effect.id, reference);
            }
        }
    }

    pub(crate) fn remember_link(&mut self, card: ObjectId, source: DamageSourceRef) {
        if let Some(card) = self.source_identity(card) {
            self.source_memory.linked.insert(card, source);
        }
    }

    /// The destination actually recorded by this source's own departure trigger.
    /// CR 400.7e grants lookup of that version, never the current arena occupant.
    pub(crate) fn own_departure_successor(&self, holder: ObjectId) -> Option<DamageSourceRef> {
        let obj = self.object(holder)?;
        let source = obj.ability?.source;
        if obj.event_object != Some(source)
            || !obj
                .riders
                .iter()
                .any(|r| matches!(r, Rider::EventDeparture(..)))
        {
            return None;
        }
        let old = obj.riders.iter().find_map(|r| match r {
            Rider::AbilitySourceVersion(v) => Some(*v),
            _ => None,
        })?;
        let version = obj.riders.iter().find_map(|r| match r {
            Rider::EventObjectIdentity(v, _) => Some(*v),
            _ => None,
        })?;
        // Both identities came from the trigger's recorded departure. The
        // arithmetic checks that association; it never infers a current one.
        if old.checked_add(1) != Some(version) {
            return None;
        }
        let reference = DamageSourceRef {
            object: source,
            version,
        };
        self.source_object(reference)
            .filter(|object| !object.zone.is_hidden_by_default())
            .map(|_| reference)
    }

    pub(crate) fn capture_rule_references(&mut self, lookup: &impl crate::state::CardLookup) {
        use baylee_cards_dsl::AbilityDef as A;
        let mut work = Vec::new();
        for &id in self.zones.list(ZoneLocation::Stack) {
            let Some(obj) = self.object(id) else { continue };
            for (index, ability) in obj.printed_abilities(lookup).iter().enumerate() {
                if obj
                    .ability
                    .is_some_and(|loc| usize::try_from(loc.index).ok() != Some(index))
                {
                    continue;
                }
                match ability {
                    A::Spell { effects, .. }
                    | A::Activated { effects, .. }
                    | A::ActivatedConditional { effects, .. }
                    | A::Triggered { effects, .. }
                    | A::Loyalty { effects, .. }
                    | A::SagaChapter { effects, .. } => work.push((id, *effects)),
                    A::ModalSpell { modes, .. } | A::ModalTriggered { modes, .. } => {
                        for (index, mode) in modes.iter().enumerate() {
                            if obj.mode_index == u8::try_from(index).ok()
                                || (index < 8 && obj.modes & (1 << index) != 0)
                            {
                                work.push((id, mode.effects));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        for (id, effects) in work {
            self.capture_linked_references(id, effects);
        }
        self.capture_source_references();
    }

    pub(crate) fn capture_linked_references(
        &mut self,
        holder: ObjectId,
        effects: &'static [baylee_cards_dsl::Effect],
    ) {
        use baylee_cards_dsl::Effect;
        if (event_readers::refers_to_event(effects)
            || (subject_readers::moves_subject(effects)
                && self.own_departure_successor(holder).is_some()))
            && let Some(holder) = self.source_identity(holder)
        {
            self.source_memory.event_readers.insert(holder);
        }
        if let Some(obj) = self.object(holder)
            && subject_readers::refers_to_subject(effects, obj.target_req.is_none())
            && let Some(subject) = obj.paid.as_ref().and_then(|p| p.source_after_cost)
            && let Some(holder) = self.source_identity(holder)
        {
            self.source_memory
                .stack
                .entry(holder)
                .or_default()
                .insert(Slot::MovedSource, subject);
        }
        let mut refers = false;
        Effect::walk(effects, &mut 0, &mut |effect| {
            refers |= matches!(
                effect,
                Effect::ReturnLinkedToBattlefield | Effect::CreateTokenFromLinked { .. }
            );
        });
        if !refers {
            return;
        }
        self.capture_source_references();
        let Some(holder) = self.source_identity(holder) else {
            return;
        };
        let Some(source) = self
            .source_memory
            .stack
            .get(&holder)
            .and_then(|refs| refs.get(&Slot::Source))
            .copied()
        else {
            return;
        };
        let linked: Vec<_> =
            self.arena
                .iter()
                .filter_map(|(_, obj)| {
                    (obj.zone == Zone::Exile
                        && self.source_memory.linked.get(&identity(obj)) == Some(&source)
                        && obj.riders.iter().any(
                            |r| matches!(r, Rider::Linked { host, .. } if *host == source.object),
                        ))
                    .then_some(identity(obj))
                })
                .collect();
        let refs = self.source_memory.stack.entry(holder).or_default();
        for (index, reference) in linked.into_iter().enumerate() {
            refs.entry(Slot::Linked(u32::try_from(index).expect("linked index")))
                .or_insert(reference);
        }
    }

    pub(crate) fn recorded_stack_target(
        &self,
        holder: ObjectId,
        index: u32,
    ) -> Option<DamageSourceRef> {
        self.recorded_target_reference(holder, false, index)
    }

    /// Exact object announced for one slot of a stack object's target group.
    #[must_use]
    pub fn recorded_target_reference(
        &self,
        holder: ObjectId,
        second: bool,
        index: u32,
    ) -> Option<DamageSourceRef> {
        let slot = if second {
            Slot::Second(index)
        } else {
            Slot::Target(index)
        };
        self.source_identity(holder)
            .and_then(|r| self.source_memory.stack.get(&r))
            .and_then(|refs| refs.get(&slot))
            .copied()
    }

    pub(crate) fn targets_current_object(&self, holder: ObjectId, target: ObjectId) -> bool {
        let Some(current) = self.source_identity(target) else {
            return false;
        };
        self.source_identity(holder)
            .and_then(|holder| self.source_memory.stack.get(&holder))
            .is_some_and(|refs| {
                refs.iter().any(|(slot, reference)| {
                    matches!(slot, Slot::Target(_) | Slot::Second(_)) && *reference == current
                })
            })
    }

    pub(crate) fn copy_source_references(&mut self, original: DamageSourceRef, copy: ObjectId) {
        if self
            .source_object(original)
            .is_some_and(|object| object.kind == crate::object::ObjectKind::AbilityOnStack)
            && let Some(reference) = self.source_identity(copy)
        {
            self.text_changes
                .set(reference, self.text_changes.get(original));
        }

        self.capture_source_references();
        let Some(to) = self.source_identity(copy) else {
            return;
        };
        if let Some(refs) = self.source_memory.stack.get(&original).cloned() {
            self.source_memory.stack.insert(to, refs);
        }
        if self.source_memory.event_readers.contains(&original) {
            self.source_memory.event_readers.insert(to);
        }
    }

    pub(crate) fn capture_target_group(&self, objects: &[ObjectId]) -> Vec<DamageSourceRef> {
        objects
            .iter()
            .map(|&id| self.source_identity(id).expect("offered target exists"))
            .collect()
    }

    pub(crate) fn bind_target_references(&mut self, holder: ObjectId, targets: &TargetReferences) {
        let Some(holder) = self.source_identity(holder) else {
            return;
        };
        let refs = self.source_memory.stack.entry(holder).or_default();
        refs.retain(|slot, _| !matches!(slot, Slot::Target(_) | Slot::Second(_)));
        for (group, objects) in [(false, &targets.first), (true, &targets.second)] {
            for (index, &reference) in objects.iter().enumerate() {
                let index = u32::try_from(index).expect("target slot");
                refs.insert(
                    if group {
                        Slot::Second(index)
                    } else {
                        Slot::Target(index)
                    },
                    reference,
                );
            }
        }
    }

    pub(crate) fn begin_permanent_resolution(&mut self, id: ObjectId) {
        self.remember_damage_source(id);
        if let Some(source) = self.source_identity(id) {
            self.source_memory.resolving.insert(source);
        }
    }

    pub(crate) fn delay_for_resolved_permanent(
        &mut self,
        spell: ObjectId,
        controller: PlayerId,
        effects: &'static [baylee_cards_dsl::Effect],
    ) {
        if let Some(spell) = self.source_identity(spell) {
            self.source_memory
                .on_resolution
                .insert(spell, (controller, effects));
        }
    }

    pub(crate) fn source_moved(&mut self, previous: DamageSourceRef) {
        if self
            .source_object(previous)
            .is_some_and(|o| o.zone == Zone::Stack)
        {
            // The active table is for current stack objects; the captured
            // division remains in SourceMemory for a later spell copy.
            self.divided.retain(|(id, _)| *id != previous.object);
        }
        if self.source_memory.resolving.remove(&previous) {
            self.resolved_source(previous);
        }
        if let Some((controller, effects)) = self.source_memory.on_resolution.remove(&previous)
            && let Some(permanent) = self.source_memory.resolved.get(&previous).copied()
        {
            self.delayed.push(crate::state::DelayedTrigger {
                controller,
                when: DelayedWhen::NextEndStep,
                action: DelayedAction::TriggerAbout {
                    source: previous.object,
                    source_version: previous.version,
                    effects,
                    // Dash defines this return instruction; its keyword
                    // rules contain no replaceable color or land words.
                    text: crate::text_changes::TextChangeMap::IDENTITY,
                    object: permanent.object,
                    version: permanent.version,
                },
            });
        }
    }

    pub(crate) fn resolved_source(&mut self, spell: DamageSourceRef) {
        if let Some(permanent) = self.source_identity(spell.object)
            && self
                .object(spell.object)
                .is_some_and(|o| o.zone == Zone::Battlefield)
        {
            self.source_memory.resolved.insert(spell, permanent);
            self.text_changes.carry(spell, permanent);
        }
    }

    pub(crate) fn is_resolved_source(
        &self,
        spell: DamageSourceRef,
        permanent: DamageSourceRef,
    ) -> bool {
        self.source_memory.resolved.get(&spell) == Some(&permanent)
    }

    /// All eligibility is structural: retained snapshots alone never grant it.
    pub(crate) fn eligible_damage_sources(&mut self) -> BTreeSet<DamageSourceRef> {
        self.capture_source_references();
        let mut out = BTreeSet::new();
        for (_, obj) in self.arena.iter() {
            if (obj.zone == Zone::Battlefield
                && !obj.status.contains(crate::object::Status::PHASED_OUT))
                || (obj.zone == Zone::Stack && obj.kind == ObjectKind::Spell)
                || (obj.zone == Zone::Command
                    && !obj.status.contains(crate::object::Status::FACE_DOWN))
            {
                out.insert(identity(obj));
            }
        }
        for (holder, refs) in &self.source_memory.stack {
            if self
                .object(holder.object)
                .is_some_and(|o| o.zone == Zone::Stack && o.version == holder.version)
            {
                out.extend(refs.iter().filter_map(|(slot, reference)| {
                    (*slot != Slot::Event || self.source_memory.event_readers.contains(holder))
                        .then_some(*reference)
                }));
            }
        }
        for (_, shield, _) in self.shields.identified() {
            if let Shielded::Object(object, version) = shield.protects {
                out.insert(DamageSourceRef { object, version });
            }
            if let ShieldKind::NextFrom { source, .. }
            | ShieldKind::RedirectNextFrom { source, .. } = shield.kind
            {
                out.insert(DamageSourceRef {
                    object: source.id,
                    version: source.version,
                });
                if let Some(permanent) = self.source_memory.resolved.get(&DamageSourceRef {
                    object: source.id,
                    version: source.version,
                }) {
                    out.insert(*permanent);
                }
            }
        }
        for effect in self.effects.iter().filter(|e| is_prevention(&e.modifier)) {
            if let EffectFilter::ObjectIs(object, version) = effect.filter {
                out.insert(DamageSourceRef { object, version });
            }
            if let Some(source) = self.source_memory.effects.get(&effect.id) {
                out.insert(*source);
            }
        }
        for replacement in &self.replacement_rules {
            if let Some(source) = self.source_identity(replacement.source) {
                out.insert(source);
            }
        }
        for delayed in &self.delayed {
            if let DelayedWhen::DiesOrIsExiled { card, version, .. }
            | DelayedWhen::LeavesBattlefield { card, version, .. } = delayed.when
            {
                out.insert(DamageSourceRef {
                    object: card,
                    version,
                });
            }
            delayed_references(&delayed.action, &mut out);
        }
        out
    }

    pub(crate) fn prune_damage_sources(&mut self) {
        let mut retained = self.eligible_damage_sources();
        // Presentation of a permission does not make its source eligible.
        for grant in &self.granted_actions {
            retained.insert(grant.offer.source);
            if let crate::choice::GrantedActionKind::PreventNextDamage {
                target: baylee_core::ids::TargetRef::Object(reference),
                ..
            } = grant.offer.effect
            {
                retained.insert(reference);
            }
        }
        // A referenced departed spell may itself refer to targets which are
        // copied by its surviving trigger. Retain this transitive closure.
        loop {
            let before = retained.len();
            for (holder, refs) in &self.source_memory.stack {
                if retained.contains(holder) {
                    retained.extend(refs.values().copied());
                }
            }
            if retained.len() == before {
                break;
            }
        }
        self.damage_sources
            .retain(|o| retained.contains(&identity(o)));
        self.source_memory.stack.retain(|holder, _| {
            retained.contains(holder)
                || self
                    .arena
                    .get(holder.object)
                    .is_some_and(|o| o.zone == Zone::Stack && o.version == holder.version)
        });
        self.source_memory
            .event_readers
            .retain(|holder| self.source_memory.stack.contains_key(holder));
        self.source_memory
            .divisions
            .retain(|holder, _| self.source_memory.stack.contains_key(holder));
        self.source_memory
            .effects
            .retain(|id, _| self.effects.contains(*id));
        self.source_memory
            .resolved
            .retain(|spell, _| retained.contains(spell));
        self.source_memory.linked.retain(|card, _| {
            self.arena
                .get(card.object)
                .is_some_and(|o| o.zone == Zone::Exile && o.version == card.version)
        });
        self.effect_text_overrides
            .retain(|(id, _)| self.effects.contains(*id));
        self.copy_snapshots
            .retain(|(id, _)| self.effects.contains(*id));
        // Wording provenance is a reader, not damage-source eligibility.
        // Keep departures until trigger collection has frozen their text.
        for (_, origin) in &self.effect_text_overrides {
            match origin {
                crate::text_changes::TextOrigin::Live(source)
                | crate::text_changes::TextOrigin::Ability { source, .. } => {
                    retained.insert(*source);
                }
                crate::text_changes::TextOrigin::Frozen(_) => {}
            }
        }
        retained.extend(
            self.ltb_versions
                .iter()
                .map(|&(object, version)| DamageSourceRef { object, version }),
        );
        retained.extend(self.ceased.iter().map(identity));
        self.text_changes.retain(|reference| {
            retained.contains(&reference)
                || self
                    .arena
                    .get(reference.object)
                    .is_some_and(|object| object.version == reference.version)
        });
    }
}

fn is_prevention(modifier: &Modifier) -> bool {
    matches!(
        modifier,
        Modifier::PreventDamageToIt
            | Modifier::PreventDamageFromIt
            | Modifier::RedirectDamageToYou(_)
            | Modifier::CountersPreventDamage(_)
    )
}

fn delayed_references(action: &DelayedAction, out: &mut BTreeSet<DamageSourceRef>) {
    match *action {
        DelayedAction::Trigger {
            source,
            source_version,
            ..
        } => {
            out.insert(DamageSourceRef {
                object: source,
                version: source_version,
            });
        }
        DelayedAction::TriggerAbout {
            source,
            source_version,
            object,
            version,
            ..
        } => {
            out.insert(DamageSourceRef {
                object: source,
                version: source_version,
            });
            out.insert(DamageSourceRef { object, version });
        }
        DelayedAction::LinkedCounterCleanup {
            source: object,
            version,
            ..
        }
        | DelayedAction::CastFromExileWithoutPaying {
            card: object,
            version,
        }
        | DelayedAction::CastFreeOrBottom {
            card: object,
            version,
        }
        | DelayedAction::CastPaying {
            card: object,
            version,
            ..
        }
        | DelayedAction::PayCostOrSacrifice {
            card: object,
            version,
            ..
        }
        | DelayedAction::Transform {
            card: object,
            version,
            ..
        }
        | DelayedAction::Sacrifice {
            card: object,
            version,
        }
        | DelayedAction::ReturnToBattlefield {
            card: object,
            version,
        }
        | DelayedAction::CastDiscovered {
            card: object,
            version,
        } => {
            out.insert(DamageSourceRef { object, version });
        }
        DelayedAction::PayCostOrLose { .. } | DelayedAction::AddMana { .. } => {}
    }
}

pub(crate) fn options(
    state: &mut GameState,
    filter: &'static Filter,
    you: PlayerId,
    this: ObjectId,
) -> Vec<DamageSourceRef> {
    options_with_context(state, filter, you, crate::eval::live_context(state, this))
}

pub(crate) fn options_with_context(
    state: &mut GameState,
    filter: &'static Filter,
    you: PlayerId,
    context: crate::text_changes::RuleContext,
) -> Vec<DamageSourceRef> {
    state
        .eligible_damage_sources()
        .into_iter()
        .filter(|&source| {
            state.source_object(source).is_some_and(|obj| {
                crate::eval::matches_projected_with_context(
                    filter,
                    state,
                    obj,
                    obj.characteristics(),
                    you,
                    context,
                )
            })
        })
        .collect()
}

#[cfg(test)]
mod tests;
