//! The two phasing events (CR 702.26): phasing out and phasing in, each with
//! what goes with it.
//!
//! A permanent phasing out takes the Auras, Equipment and Fortifications
//! attached to it along, "indirectly" (CR 702.26g), and one of those phases
//! in only with the permanent it is attached to. An object named directly
//! that is also attached to something phasing out phases out only
//! indirectly (CR 702.26h). Neither event attaches or unattaches anything
//! (CR 702.26d, 702.26j): `attached_to` is left as it was, and a directly
//! phased-out Aura whose object has gone meets the attachment state-based
//! actions once it is back (CR 702.26i, 704.5m).
//!
//! Both events invalidate the projection. A phased-out permanent is not
//! projected (`GameState::refresh_characteristics`), so its characteristics
//! and its controller stay what they were as it phased out: nothing affects
//! it (CR 702.26b), and it phases in during the untap step of the player who
//! controlled it then (CR 502.1). Phasing in is where it is projected again.

use crate::event::GameEvent;
use crate::object::Status;
use crate::state::GameState;
use baylee_core::ids::ObjectId;

impl GameState {
    /// Phases out every permanent of `direct` that is on the battlefield and
    /// phased in, and with each everything attached to it, transitively (an
    /// Aura on an Equipment on a creature).
    pub(crate) fn phase_out(&mut self, direct: &[ObjectId]) {
        let named: Vec<ObjectId> = direct
            .iter()
            .copied()
            .filter(|&id| {
                self.object(id).is_some_and(|o| {
                    o.zone == crate::zone::Zone::Battlefield
                        && !o.status.contains(Status::PHASED_OUT)
                })
            })
            .collect();
        // The whole set is read before any status changes: a host marked
        // first would hide what is attached to it from the walk.
        let mut indirect: Vec<ObjectId> = Vec::new();
        let mut hosts = named.clone();
        let mut next = 0;
        while let Some(&host) = hosts.get(next) {
            next += 1;
            for id in self.battlefield_seen() {
                if self.object(id).is_some_and(|o| o.attached_to == Some(host))
                    && !indirect.contains(&id)
                {
                    indirect.push(id);
                    hosts.push(id);
                }
            }
        }
        let phasing = named
            .iter()
            .filter(|id| !indirect.contains(id))
            .map(|&id| (id, false))
            .chain(indirect.iter().map(|&id| (id, true)));
        for (id, indirectly) in phasing.collect::<Vec<_>>() {
            if let Some(obj) = self.object_mut(id) {
                obj.status.insert(Status::PHASED_OUT);
                if indirectly {
                    obj.status.insert(Status::PHASED_OUT_INDIRECTLY);
                }
            }
            // CR 702.26b, 506.4: removed from combat without changing
            // zones; an attacker it blocked stays blocked.
            self.combat.remove_from_combat(id);
            self.journal.record(GameEvent::PhaseChanged {
                object: id,
                phased_out: true,
            });
        }
        self.invalidate_projections();
    }

    /// Gives each phased-out permanent whose controller has left the game
    /// the controller layer 2 gives it now.
    ///
    /// A phased-out permanent is not projected, so its controller is the one
    /// it phased out under. When that player leaves, the effects that gave
    /// them control of it end (CR 800.4a), and "game rules may cause a
    /// phased-out permanent to leave the game or to be exiled once a player
    /// leaves the game" (CR 702.26n): what they control by default is exiled
    /// with the rest, and what was only theirs through an effect is not. So
    /// the controller is read again here, before the exile asks it, and the
    /// permanent phases in at the untap step of the player it belongs to
    /// now. CR 702.26n has it phase in "during the next untap step after
    /// that player's next turn would have begun", which can be a round
    /// later; the engine does not keep that turn.
    pub(crate) fn release_from_the_departed(&mut self) {
        let stranded: Vec<ObjectId> = self
            .zones
            // phasing: the walk is for the permanents that are phased out.
            .list(crate::zone::ZoneLocation::Battlefield)
            .iter()
            .copied()
            .filter(|&id| {
                self.object(id).is_some_and(|o| {
                    o.status.contains(Status::PHASED_OUT) && self.has_left(o.controller)
                })
            })
            .collect();
        if stranded.is_empty() {
            return;
        }
        let plan = crate::layers::LayerPlan::build(&self.effects);
        for id in stranded {
            let Some(obj) = self.object(id) else {
                continue;
            };
            let controller = crate::layers::recompute_with(self, obj, &plan).controller;
            if obj.controller != controller {
                if let Some(obj) = self.object_mut(id) {
                    obj.controller = controller;
                }
                self.restart_summoning_sickness(id);
            }
        }
    }

    /// Phases `id` in, and with it everything that phased out indirectly
    /// attached to it, transitively.
    pub(crate) fn phase_in(&mut self, id: ObjectId) {
        let mut coming = vec![id];
        let mut next = 0;
        while let Some(&host) = coming.get(next) {
            next += 1;
            if let Some(obj) = self.object_mut(host) {
                obj.status.remove(Status::PHASED_OUT);
                obj.status.remove(Status::PHASED_OUT_INDIRECTLY);
            }
            self.journal.record(GameEvent::PhaseChanged {
                object: host,
                phased_out: false,
            });
            // phasing: the walk is for what is still phased out.
            for &other in self.zones.list(crate::zone::ZoneLocation::Battlefield) {
                if self.object(other).is_some_and(|o| {
                    o.attached_to == Some(host) && o.status.contains(Status::PHASED_OUT_INDIRECTLY)
                }) && !coming.contains(&other)
                {
                    coming.push(other);
                }
            }
        }
        self.invalidate_projections();
    }
}
