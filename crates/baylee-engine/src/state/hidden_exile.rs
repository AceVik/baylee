//! Face-down exile and permanent, private inspection rights (CR 406.3, 702.75a).

use super::{GameState, NAMELESS};
use crate::object::{Rider, Status};
use crate::zone::{Zone, ZoneLocation};
use baylee_core::ids::{DamageSourceRef, ObjectId, PlayerId, SeatSet};
use std::sync::Arc;

impl GameState {
    #[doc(hidden)]
    pub fn hide_exiled_card(
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
        let Some(object) = self
            .object(card)
            .filter(|o| o.zone == Zone::Exile && !o.status.contains(Status::FACE_DOWN))
        else {
            return;
        };
        let original = Arc::clone(&object.base);
        let blank = self.bare_base(NAMELESS);
        let object = self.object_mut(card).expect("exiled card exists");
        object.original_base = Some(original);
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

    /// Exile `card` face up (if not already in exile) and mark it as plotted by
    /// `player` on the current turn (`CR 702.170a`, `CR 702.170c`).
    ///
    /// Per `CR 702.170c`, only cards can become plotted: tokens or copies of
    /// spells/abilities exiled this way cease to exist without becoming plotted.
    pub(crate) fn plot_card_into_exile(&mut self, player: PlayerId, card: ObjectId) -> bool {
        let Some((owner, zone)) = self.object(card).map(|o| (o.owner, o.zone)) else {
            return false;
        };
        if zone != Zone::Exile {
            let _ = self.move_object(
                card,
                ZoneLocation::Exile(owner),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            );
        }
        let _ = self.reveal_exiled_card(card);
        let turn = self.turn.number;
        let Some(object) = self.object_mut(card) else {
            return false;
        };
        if object.zone != Zone::Exile
            || !object.is_card()
            || object.riders.contains(&Rider::SpellCopy)
            || object.riders.contains(&Rider::AbilityCopy)
        {
            return false;
        }
        object
            .riders
            .retain(|r| !matches!(r, Rider::Plotted | Rider::PlottedOnTurn(_)));
        object.riders.push(Rider::Plotted);
        object.riders.push(Rider::PlottedOnTurn(turn));
        self.journal.record(crate::event::GameEvent::CardPlotted {
            object: card,
            player,
        });
        true
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

    /// Records [`crate::event::GameEvent::PlayerScried`] when an active
    /// ability, granted trigger, off-battlefield trigger, or delayed trigger
    /// watches [`baylee_cards_dsl::Trigger::Scried`].
    ///
    /// Skipping unobserved scry events preserves `trigger_scan_seq` (and
    /// therefore `snapshot_hash` and simulation RNG seeds) on existing games
    /// that scry with no scry observer in play.
    pub(crate) fn record_player_scried(&mut self, player: PlayerId, count: u8) {
        if count == 0 || !self.has_scry_observer() {
            return;
        }
        self.journal
            .record(crate::event::GameEvent::PlayerScried { player, count });
    }

    fn has_scry_observer(&self) -> bool {
        let ability_watches = |ab: &baylee_cards_dsl::AbilityDef| match ab {
            baylee_cards_dsl::AbilityDef::Triggered { trigger, .. } => {
                matches!(trigger, baylee_cards_dsl::Trigger::Scried { .. })
            }
            baylee_cards_dsl::AbilityDef::Static(sa) => matches!(
                sa.modifier,
                baylee_cards_dsl::Modifier::GrantTriggered {
                    trigger: baylee_cards_dsl::Trigger::Scried { .. },
                    ..
                }
            ),
            _ => false,
        };
        if self.effects.iter().any(|fx| {
            matches!(
                fx.modifier,
                baylee_cards_dsl::Modifier::GrantTriggered {
                    trigger: baylee_cards_dsl::Trigger::Scried { .. },
                    ..
                }
            )
        }) || self.delayed.iter().any(|d| {
            matches!(
                d.when,
                crate::state::DelayedWhen::OnEvent {
                    trigger: baylee_cards_dsl::Trigger::Scried { .. },
                    ..
                }
            )
        }) {
            return true;
        }
        if self.battlefield_seen().any(|id| {
            self.object(id)
                .is_some_and(|o| o.abilities(&*self.bases).iter().any(ability_watches))
        }) {
            return true;
        }
        (0..self.players.len()).any(|seat| {
            let p = PlayerId::new(u8::try_from(seat).expect("seat fits"));
            [
                ZoneLocation::Command(p),
                ZoneLocation::Graveyard(p),
                ZoneLocation::Exile(p),
            ]
            .into_iter()
            .flat_map(|loc| self.zones.list(loc).iter().copied())
            .any(|id| {
                self.object(id)
                    .is_some_and(|o| o.abilities(&*self.bases).iter().any(ability_watches))
            })
        })
    }
}
