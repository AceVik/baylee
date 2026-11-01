//! Face-down exile and permanent, private inspection rights (CR 406.3, 702.75a).

use super::{GameState, NAMELESS};
use crate::object::{Rider, Status};
use crate::zone::{Zone, ZoneLocation};
use baylee_core::ids::{DamageSourceRef, ObjectId, PlayerId, SeatSet};
use std::sync::Arc;

impl GameState {
    pub(crate) fn hide_exiled_card(
        &mut self,
        card: ObjectId,
        source: DamageSourceRef,
        acquisition: u64,
        seen: SeatSet,
        follow_source: bool,
    ) {
        if !self.object(card).is_some_and(|o| o.zone == Zone::Exile) {
            return;
        }
        let blank = self.bare_base(NAMELESS);
        let object = self.object_mut(card).expect("exiled card exists");
        object.original_base = Some(Arc::clone(&object.base));
        object.base = blank;
        object.cache.clear();
        object.status.insert(Status::FACE_DOWN);
        object.riders.push(Rider::LinkedExile {
            source,
            acquisition,
        });
        object.riders.push(Rider::ExileVisibility {
            seen,
            follow_source,
        });
        if follow_source
            && let Some(host) = self
                .object_mut(source.object)
                .filter(|o| o.zone == Zone::Battlefield && o.version == source.version)
        {
            host.riders.retain(|r| !matches!(r, Rider::HideawayHost(_)));
            host.riders.push(Rider::HideawayHost(source.version));
        }
        self.learn_hideaway_cards(source.object);
        self.invalidate_projections();
    }

    /// Reveal immediately before announcing a cast or land play (CR 406.3a).
    pub(crate) fn reveal_exiled_card(&mut self, card: ObjectId) -> bool {
        let Some(object) = self.object_mut(card).filter(|o| o.zone == Zone::Exile) else {
            return false;
        };
        if !object.status.contains(Status::FACE_DOWN) {
            return false;
        }
        if let Some(base) = object.original_base.take() {
            object.base = base;
        }
        object.status.remove(Status::FACE_DOWN);
        object.cache.clear();
        self.invalidate_projections();
        self.refresh_characteristics();
        true
    }

    /// A play that could not legally be completed leaves the original card hidden.
    pub(crate) fn conceal_exiled_card(&mut self, card: ObjectId) {
        let blank = self.bare_base(NAMELESS);
        let Some(object) = self.object_mut(card).filter(|o| o.zone == Zone::Exile) else {
            return;
        };
        object.original_base = Some(Arc::clone(&object.base));
        object.base = blank;
        object.face_index = 0;
        object.status.insert(Status::FACE_DOWN);
        object.cache.clear();
        self.invalidate_projections();
    }

    /// Mark this exact exiled incarnation as foretold and privately visible
    /// to its owner. No counters or linked host are involved (CR 702.143a).
    pub(crate) fn foretell_exiled_card(&mut self, card: ObjectId, owner: PlayerId, turn: u32) {
        self.conceal_exiled_card(card);
        let object = self.object_mut(card).expect("foretold card exists");
        object.riders.push(Rider::Foretold { turn });
        object.riders.push(Rider::ExileVisibility {
            seen: std::iter::once(owner).collect(),
            follow_source: false,
        });
    }

    /// Called when control actually changes, and when a hideaway host phases in.
    /// Only exile is visited; ordinary projection passes do not scan for rights.
    pub(crate) fn learn_hideaway_cards(&mut self, host: ObjectId) {
        let Some(object) = self.object(host).filter(|o| {
            o.zone == Zone::Battlefield
                && !o.status.contains(Status::PHASED_OUT)
                && o.riders.contains(&Rider::HideawayHost(o.version))
        }) else {
            return;
        };
        let source = DamageSourceRef {
            object: host,
            version: object.version,
        };
        let controller = object.controller;
        let cards: Vec<_> = (0..self.players.len())
            .flat_map(|i| self.zones.list(ZoneLocation::Exile(PlayerId::new(u8::try_from(i).expect("seat fits")))))
            .copied()
            .filter(|&id| self.object(id).is_some_and(|o| {
                o.riders.iter().any(|r| matches!(r, Rider::LinkedExile { source: s, .. } if *s == source))
                    && o.riders.iter().any(|r| matches!(r, Rider::ExileVisibility { seen, follow_source: true } if !seen.contains(controller)))
            }))
            .collect();
        for card in cards {
            let object = self.object_mut(card).expect("exiled card exists");
            for rider in &mut object.riders {
                if let Rider::ExileVisibility {
                    seen,
                    follow_source: true,
                } = rider
                {
                    seen.insert(controller);
                }
            }
        }
    }
}
