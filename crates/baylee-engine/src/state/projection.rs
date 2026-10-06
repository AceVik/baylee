//! The layer refresh: every object's characteristics and controller, projected.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary
use super::*;

impl GameState {
    /// Hot path: one generation compare. When stale, permanents and stack
    /// objects are re-projected through the layer system (CR 613).
    ///
    /// # Panics
    /// Internal invariant violations (zone objects always exist).
    pub fn refresh_characteristics(&mut self) {
        crate::resolve::linked_counters::expire(self);
        // Both halves of the stack shortcut below fail silently — an id
        // left in the subset is projected after its object is gone, a spell
        // missing from it quietly stops being affected by anthems — and the
        // sites that fill it are spread over five files. Checked here, on
        // every engine step rather than only when the effect set moved, so
        // that every test on every code path is a test of the invariant.
        debug_assert!(
            self.stack_projection_set_is_consistent(),
            "stack_projectable drifted from the stack"
        );
        if self.characteristics_generation == self.effects.generation {
            return;
        }
        let generation = self.effects.generation;
        // Bucket and prepare the object-independent dependency orders
        // once for the whole pass. Conditional animation's type-layer
        // order is decided during projection (see `layers::LayerPlan`).
        let plan = crate::layers::LayerPlan::build(&self.effects);
        // Cross-zone effects (Maskwood Nexus & co.) reach into library,
        // hand, graveyard — then every object must be projected, not only
        // battlefield + stack.
        let cross_zone = self
            .effects
            .iter()
            .any(|fx| matches!(fx.filter, crate::effects::EffectFilter::Dsl(f) if filter_reaches_other_zones(f)));
        // Scratch buffers live in the state so a refresh — which runs
        // after every single effect-set change — allocates nothing.
        let mut ids = std::mem::take(&mut self.projection_ids);
        ids.clear();
        // Cached off-board projections must be revisited after the last
        // cross-zone effect disappears (#120).
        //
        // A phased-out permanent is not projected at all: it can't be
        // affected by anything (CR 702.26b), so what it was as it phased out
        // is what it stays, and so is its controller, the player it phases
        // in under (CR 502.1). `phase_in` invalidates, and the next refresh
        // takes it back in.
        if cross_zone || self.projected_cross_zone {
            ids.extend(
                self.arena
                    .iter()
                    .filter(|(_, o)| !o.status.contains(crate::object::Status::PHASED_OUT))
                    .map(|(id, _)| id),
            );
        } else {
            // The stack contributes only its *spells*. An ability on the
            // stack has no characteristic a layer can touch, and this pass
            // runs once per engine step — walking six figures of Ally
            // triggers to establish that, every time a counter moves, is
            // the difference between a long game and no game at all.
            ids.extend(
                self.battlefield_seen()
                    .chain(self.zones.stack_projectable().iter().copied()),
            );
            // And the cards that define their own power and toughness,
            // wherever else they are (CR 604.3).
            ids.extend(self.printed_pt_cda.iter().map(|(id, _)| *id).filter(|id| {
                self.object(*id)
                    .is_some_and(|o| !matches!(o.zone, Zone::Battlefield | Zone::Stack))
            }));
        }
        // Layer 2 decides the controller every later layer reads, of this
        // object and of every other one (CR 613.1b before 613.1c–f), and a
        // static ability's "you" is whoever controls its source now
        // (CR 109.5). A walk projects one object at a time, so what it read
        // of a controller it had not reached yet, or of the object it was
        // projecting, was the last refresh's: a creature just taken was not
        // pumped by its taker's anthem. So: point each static at its
        // source's controller, walk, and walk again while a walk moved a
        // controller. The walk that moves none read exactly the controllers
        // it wrote. That is one walk when no control changed and two when
        // one did; a static that gives control of something (Control Magic)
        // whose source changed hands is a third (CR 613.8a). The bound only
        // stops a dependency loop, which CR 613.8b settles by timestamp and
        // this by stopping where it is.
        let mut was: Vec<(ObjectId, PlayerId)> = Vec::new();
        // Empty, and so unallocated, on every board where nothing counts.
        let mut readers: Vec<ObjectId> = Vec::new();
        let mut settled = false;
        for _ in 0..plan.control_effects() + 2 {
            self.follow_static_sources();
            readers.clear();
            if !self.project_all(&ids, &plan, &mut was, &mut readers) {
                settled = true;
                break;
            }
        }
        debug_assert!(
            !settled || self.statics_follow_their_sources(),
            "a settled refresh left a static ability's controller behind its source's"
        );
        // The same for what an object counts (CR 613.1: the layers in their
        // order, so a count in layer 7 sees every type layer 4 gave). Ashaya
        // counts the lands you control, and an Elf the walk had not reached
        // was still the last refresh's Elf and not yet this one's Forest, so
        // Ashaya came out one short and stayed so. The board is finished
        // now: project what counted again, and again while that moved a
        // count another counter reads. Each pass settles at least one more
        // counter; the bound stops counters that count each other round in
        // a circle.
        for _ in 0..readers.len() {
            if !self.project_readers(&readers, &plan, &mut was) {
                break;
            }
        }
        // CR 302.6 wants control held continuously since the turn began. A
        // permanent a walk moved and a later one moved back never changed
        // hands, so the comparison is with the controller before the first.
        for (id, before) in was {
            if self.object(id).is_some_and(|o| o.controller != before) {
                self.restart_summoning_sickness(id);
            }
        }
        ids.clear();
        self.projection_ids = ids;
        self.projected_cross_zone = cross_zone;
        self.characteristics_generation = generation;
    }

    /// One walk of the refresh: projects `ids` through every layer and
    /// writes each controller, noting in `was` the controller an object had
    /// before a walk first moved it, and in `readers` every object whose
    /// projection counted others (`layers::Projection::read_board`).
    /// Whether any controller moved.
    fn project_all(
        &mut self,
        ids: &[ObjectId],
        plan: &crate::layers::LayerPlan,
        was: &mut Vec<(ObjectId, PlayerId)>,
        readers: &mut Vec<ObjectId>,
    ) -> bool {
        let mut moved = false;
        for &id in ids {
            let Some(obj) = self.object(id) else {
                continue;
            };
            // Every branch asks before it writes: an object whose projection
            // and controller did not move is left unwritten, so its arena
            // chunk stays shared with the answer's checkpoint (`arena`).
            if crate::layers::needs_projection(self, plan, obj)
                || self.defines_pt_off_battlefield(obj)
            {
                let projection = crate::layers::recompute_with(self, obj, plan);
                if projection.read_board {
                    readers.push(id);
                }
                if obj.controller == projection.controller
                    && obj.cache.holds(&projection.characteristics, &obj.base)
                {
                    continue;
                }
                let obj = self.object_mut(id).expect("zone object exists");
                moved |= settle_controller(obj, projection.controller, was);
                // `cache` and `base` are disjoint fields, so this is one
                // mutable borrow and one shared borrow of the same object.
                let crate::object::GameObject { cache, base, .. } = obj;
                cache.store(projection.characteristics, base);
            } else {
                // Nothing can change this object's characteristics, so the
                // base is the projection. Dropping the cache is not just
                // cheaper than recomputing it — it is what keeps an
                // untouched board's per-object projection memory at zero.
                if obj.cache.is_clear() && obj.controller == obj.base_controller {
                    continue;
                }
                let obj = self.object_mut(id).expect("checked above");
                obj.cache.clear();
                // No effects means no layer 2 either: whoever the base
                // says controls it does, which is how a "gain control
                // until end of turn" hands the permanent back.
                let base = obj.base_controller;
                moved |= settle_controller(obj, base, was);
            }
        }
        moved
    }

    /// Projects again the objects a walk found counting others, now that
    /// every one of those others is this refresh's. Whether any of them
    /// came out different, which is what another counter may have read.
    fn project_readers(
        &mut self,
        readers: &[ObjectId],
        plan: &crate::layers::LayerPlan,
        was: &mut Vec<(ObjectId, PlayerId)>,
    ) -> bool {
        let mut changed = false;
        for &id in readers {
            let Some(obj) = self.object(id) else {
                continue;
            };
            let projection = crate::layers::recompute_with(self, obj, plan);
            // As in `project_all`: nothing moved, nothing written.
            if obj.controller == projection.controller
                && obj.cache.holds(&projection.characteristics, &obj.base)
            {
                continue;
            }
            let obj = self.object_mut(id).expect("zone object exists");
            changed |= settle_controller(obj, projection.controller, was);
            changed |= *obj.characteristics() != projection.characteristics;
            let crate::object::GameObject { cache, base, .. } = obj;
            cache.store(projection.characteristics, base);
        }
        changed
    }

    /// Gives each static ability's effect, and each replacement rule, the
    /// controller its source has now (CR 109.5).
    ///
    /// Only for a source on the battlefield. One that has left keeps the
    /// controller it last had there until `sync_static_effects` drops what
    /// it registered: its last-known information, so that a rule consulted
    /// after its source moved in the middle of one event does not change
    /// sides to the owner.
    fn follow_static_sources(&mut self) {
        let Self {
            effects,
            replacement_rules,
            arena,
            ..
        } = self;
        let now = |source: ObjectId| {
            arena
                .get(source)
                .filter(|o| o.zone == Zone::Battlefield)
                .map(|o| o.controller)
        };
        effects.follow_sources(now);
        for entry in replacement_rules.iter_mut() {
            if let Some(controller) = now(entry.source) {
                entry.controller = controller;
            }
        }
    }

    /// Whether every static ability's effect and replacement rule names the
    /// player who controls its source on the battlefield now: what a
    /// settled refresh leaves behind.
    fn statics_follow_their_sources(&self) -> bool {
        let now = |source: ObjectId| {
            self.object(source)
                .filter(|o| o.zone == Zone::Battlefield)
                .map(|o| o.controller)
        };
        self.effects.iter().all(|fx| {
            !fx.origin.is_static() || fx.source.and_then(now).is_none_or(|c| c == fx.controller)
        }) && self
            .replacement_rules
            .iter()
            .all(|entry| now(entry.source).is_none_or(|c| c == entry.controller))
    }
}

/// Writes the controller a walk of the refresh projected, noting in `was`
/// the one the object had before a walk first moved it. Whether it moved.
fn settle_controller(
    obj: &mut GameObject,
    controller: PlayerId,
    was: &mut Vec<(ObjectId, PlayerId)>,
) -> bool {
    if obj.controller == controller {
        return false;
    }
    if !was.iter().any(|(moved, _)| *moved == obj.id) {
        was.push((obj.id, obj.controller));
    }
    obj.controller = controller;
    true
}
