//! Casts made during a resolving instruction retain that instruction's continuation.

use super::{CardLookup, Engine, EngineError, ObjectId, PlayerId};
use crate::resolve::{AwaitingOp, Flow, Resolution};

/// A nested cast has not finished the spell or ability that instructed it.
#[derive(Clone, Debug)]
pub(super) struct EffectPlay {
    resolution: Box<Resolution>,
    control: Option<(PlayerId, PlayerId, ObjectId)>,
    linked: Option<LinkedPlay>,
    awaiting_land_entry: bool,
    offered: Option<ObjectId>,
}

#[derive(Clone, Copy, Debug, Hash)]
struct LinkedPlay {
    card: baylee_core::ids::DamageSourceRef,
    face_down: bool,
    opened: bool,
}

impl EffectPlay {
    pub(super) fn control(&self) -> Option<(PlayerId, PlayerId)> {
        self.control
            .map(|(controller, player, _)| (controller, player))
    }
    pub(super) fn commanded_card(&self) -> Option<ObjectId> {
        self.control.map(|(_, _, card)| card).or(self.offered)
    }
    pub(super) fn fingerprint(&self) -> u64 {
        self.resolution
            .program_fingerprint()
            .wrapping_mul(31)
            .wrapping_add(
                self.resolution
                    .awaiting
                    .as_ref()
                    .map_or(0, AwaitingOp::search_fingerprint),
            )
            .wrapping_mul(31)
            .wrapping_add(crate::state::structural_fingerprint(&(
                self.control,
                self.linked,
                self.awaiting_land_entry,
                self.offered,
            )))
    }
}

impl<L: CardLookup> Engine<L> {
    /// Probe the same cast procedure that accepting a room offer will start.
    /// Otherwise an AI can repeatedly select an uncastable card without progress.
    pub(super) fn filter_dungeon_plays(&mut self) -> bool {
        let Some(AwaitingOp::DungeonPlay(play)) =
            self.resolution.as_ref().and_then(|r| r.awaiting.as_ref())
        else {
            return false;
        };
        let crate::choice::Pending::ChooseCards {
            player, options, ..
        } = &self.pending
        else {
            return false;
        };
        let player = *player;
        let free = play.free;
        let available: Vec<_> = options
            .iter()
            .copied()
            .filter(|&card| {
                let modes = if free {
                    self.immediate_free_options(player, card, true)
                } else {
                    self.cast_options(player, card, false)
                };
                modes.is_ok_and(|modes| !modes.is_empty())
            })
            .collect();
        if available.is_empty() {
            self.awaiting_answer = false;
            self.answer_dungeon_play(player, &[]);
            return true;
        }
        if let crate::choice::Pending::ChooseCards { options, .. } = &mut self.pending {
            *options = available;
        }
        false
    }

    pub(super) fn answer_effect_play(
        &mut self,
        player: PlayerId,
        cards: &[ObjectId],
    ) -> Option<Result<(), EngineError>> {
        let awaiting = self.resolution.as_ref()?.awaiting.as_ref()?;
        let subject = match awaiting {
            AwaitingOp::BattleDefeatCast { card } => {
                let card = *card;
                self.answer_battle_defeat(player, cards, card);
                return Some(Ok(()));
            }
            AwaitingOp::DungeonPlay(_) => {
                self.answer_dungeon_play(player, cards);
                return Some(Ok(()));
            }
            AwaitingOp::LinkedPlay { .. } => {
                self.answer_linked_play(player, cards);
                return Some(Ok(()));
            }
            AwaitingOp::MaskedCast => None,
            AwaitingOp::ControlledCard { player } => Some(*player),
            _ => return None,
        };
        let resolution = self.resolution.take()?;
        let control = subject
            .zip(cards.first().copied())
            .map(|(subject, card)| (resolution.controller, subject, card));
        if let Some((_, subject, card)) = control
            && let Some(card) = self.state.source_identity(card)
        {
            self.state
                .constrained_payments
                .push(crate::constrained_payment::ConstrainedPayment {
                    player: subject,
                    card,
                    required: baylee_core::mana::ManaPool::new(),
                });
        }
        self.effect_plays.push(EffectPlay {
            resolution: Box::new(resolution),
            control,
            linked: None,
            awaiting_land_entry: false,
            offered: None,
        });
        let result = cards.first().map_or(
            Err(EngineError::IllegalAction("casting declined")),
            |&card| {
                if let Some(subject) = subject {
                    self.play_commanded_card(subject, card)
                } else {
                    self.start_masked_cast(player, card)
                }
            },
        );
        if result.is_err() {
            self.finish_nested_cast();
        }
        Some(Ok(()))
    }

    fn answer_battle_defeat(
        &mut self,
        player: PlayerId,
        cards: &[ObjectId],
        card: baylee_core::ids::DamageSourceRef,
    ) {
        let resolution = self.resolution.take().expect("battle defeat resolution");
        self.effect_plays.push(EffectPlay {
            resolution: Box::new(resolution),
            control: None,
            linked: None,
            awaiting_land_entry: false,
            offered: None,
        });
        if cards.first() != Some(&card.object)
            || self.start_transformed_free_cast(player, card).is_err()
        {
            self.finish_nested_cast();
        }
    }

    fn answer_linked_play(&mut self, player: PlayerId, cards: &[ObjectId]) {
        let mut resolution = self.resolution.take().expect("linked play resolution");
        let Some(AwaitingOp::LinkedPlay {
            cards: remaining,
            copy,
        }) = resolution.awaiting.as_mut()
        else {
            unreachable!("linked play continuation")
        };
        let chosen = cards
            .first()
            .and_then(|id| remaining.iter().find(|r| r.object == *id))
            .copied();
        if let Some(chosen) = chosen {
            remaining.retain(|r| *r != chosen);
        } else {
            remaining.clear();
        }
        let chosen = chosen.and_then(|original| {
            if *copy {
                self.copy_exiled_card(original)
            } else {
                Some(original)
            }
        });
        let linked = chosen
            .filter(|r| {
                self.state
                    .object(r.object)
                    .is_some_and(|o| o.zone == crate::zone::Zone::Exile && o.version == r.version)
            })
            .map(|card| {
                let face_down = self.state.reveal_exiled_card(card.object);
                let object = self.state.object_mut(card.object).expect("chosen exile");
                let rider = crate::object::Rider::PlayableFromExileFor(player);
                let opened = !object.riders.contains(&rider);
                if opened {
                    object.riders.push(rider);
                }
                LinkedPlay {
                    card,
                    face_down,
                    opened,
                }
            });
        self.effect_plays.push(EffectPlay {
            resolution: Box::new(resolution),
            control: None,
            linked,
            awaiting_land_entry: false,
            offered: None,
        });
        if let Some(linked) = linked {
            if self
                .start_linked_free_play(player, linked.card.object)
                .is_err()
            {
                self.finish_nested_cast();
            }
        } else {
            self.finish_nested_cast();
        }
    }

    fn answer_dungeon_play(&mut self, player: PlayerId, cards: &[ObjectId]) {
        let mut resolution = self.resolution.take().expect("dungeon play resolution");
        let Some(AwaitingOp::DungeonPlay(play)) = resolution.awaiting.as_mut() else {
            unreachable!("dungeon play")
        };
        let chosen = cards
            .first()
            .and_then(|id| play.cards.iter().find(|r| r.object == *id))
            .copied();
        let free = play.free;
        if chosen.is_none() {
            play.cards.clear();
        }
        let linked = chosen
            .filter(|r| {
                self.state.object(r.object).is_some_and(|o| {
                    o.version == r.version
                        && o.zone
                            == if free {
                                crate::zone::Zone::Hand
                            } else {
                                crate::zone::Zone::Exile
                            }
                })
            })
            .filter(|_| !free)
            .map(|card| {
                let object = self.state.object_mut(card.object).expect("offered exile");
                let rider = crate::object::Rider::PlayableFromExileFor(player);
                let opened = !object.riders.contains(&rider);
                if opened {
                    object.riders.push(rider);
                }
                LinkedPlay {
                    card,
                    face_down: false,
                    opened,
                }
            });
        self.effect_plays.push(EffectPlay {
            resolution: Box::new(resolution),
            control: None,
            linked,
            awaiting_land_entry: false,
            offered: chosen.map(|r| r.object),
        });
        let result = chosen.map_or(Err(EngineError::IllegalAction("play declined")), |r| {
            if free {
                self.start_immediate_free_play(player, r.object, true)
            } else {
                self.start_cast_wizard(player, r.object)
            }
        });
        if result.is_err() {
            self.finish_nested_cast();
        }
    }

    /// Create the exact spell copy a linked-copy instruction casts, leaving
    /// the original and its exile link untouched.
    fn copy_exiled_card(
        &mut self,
        original: baylee_core::ids::DamageSourceRef,
    ) -> Option<baylee_core::ids::DamageSourceRef> {
        let mut copy = self
            .state
            .object(original.object)
            .filter(|object| {
                object.zone == crate::zone::Zone::Exile && object.version == original.version
            })?
            .clone();
        copy.riders.clear();
        self.state.timestamp += 1;
        copy.timestamp = self.state.timestamp;
        copy.controlled_since = self.state.timestamp;
        let owner = copy.owner;
        let id = self.state.arena.insert_with(|id| {
            copy.id = id;
            copy
        });
        self.state.zones.insert(
            id,
            crate::zone::ZoneLocation::Exile(owner),
            crate::zone::ZonePosition::Top,
            true,
        );
        let object = self.state.object_mut(id)?;
        object.zone = crate::zone::Zone::Exile;
        object.zone_owner = Some(owner);
        object.kind = crate::object::ObjectKind::Card;
        object.riders.push(crate::object::Rider::SpellCopy);
        let version = object.version;
        self.state.watch_token_cleanup(id);
        Some(baylee_core::ids::DamageSourceRef {
            object: id,
            version,
        })
    }

    fn play_commanded_card(&mut self, player: PlayerId, card: ObjectId) -> Result<(), EngineError> {
        self.start_cast_wizard(player, card)
    }

    /// A land play is not complete until its entry replacements have finished.
    pub(super) fn suspend_nested_land_entry(&mut self) {
        if let Some(parent) = self.effect_plays.last_mut() {
            parent.awaiting_land_entry = true;
        }
    }

    pub(super) fn resume_nested_land_entry(&mut self) -> bool {
        if self
            .effect_plays
            .last()
            .is_some_and(|parent| parent.awaiting_land_entry)
        {
            self.finish_nested_cast();
            true
        } else {
            false
        }
    }

    /// Called before ordinary priority/SBA processing when the nested cast ends.
    pub(super) fn finish_nested_cast(&mut self) -> bool {
        let Some(mut parent) = self.effect_plays.pop() else {
            return false;
        };
        if parent.control.is_some() {
            self.state.constrained_payments.pop();
        }
        if let Some((controller, player, card)) = parent.control
            && self
                .state
                .object(card)
                .is_some_and(|object| object.zone == crate::zone::Zone::Stack)
        {
            self.remember_controlled_spell(card, controller, player);
        }
        if let Some(linked) = parent.linked
            && self.state.object(linked.card.object).is_some_and(|o| {
                o.zone == crate::zone::Zone::Exile && o.version == linked.card.version
            })
        {
            if linked.face_down {
                self.state.conceal_exiled_card(linked.card.object);
            }
            if linked.opened
                && let Some(object) = self.state.object_mut(linked.card.object)
            {
                let rider =
                    crate::object::Rider::PlayableFromExileFor(parent.resolution.controller);
                object.riders.retain(|r| *r != rider);
            }
        }
        if let Some(AwaitingOp::DungeonPlay(play)) = parent.resolution.awaiting.as_ref() {
            let mut play = play.clone();
            // A rejected/cancelled cast does not consume this permission.
            // Only a card that actually left the offered incarnation was played.
            let played = parent.offered.is_some_and(|id| {
                play.cards.iter().find(|r| r.object == id).is_some_and(|r| {
                    self.state.object(id).is_none_or(|o| {
                        o.version != r.version
                            || o.zone
                                != if play.free {
                                    crate::zone::Zone::Hand
                                } else {
                                    crate::zone::Zone::Exile
                                }
                    })
                })
            });
            if played {
                if play.free {
                    play.cards.clear();
                } else {
                    play.cards.retain(|r| Some(r.object) != parent.offered);
                }
            }
            parent.resolution.awaiting = None;
            if let Some(question) =
                crate::resolve::dungeon::continue_play(&self.state, &mut parent.resolution, play)
            {
                self.resolution = Some(*parent.resolution);
                self.pending = question;
                self.awaiting_answer = true;
                return true;
            }
        }
        if let Some(AwaitingOp::LinkedPlay { cards, copy }) = parent.resolution.awaiting.take()
            && let Some(question) = crate::resolve::hidden_exile::continue_play(
                &self.state,
                &mut parent.resolution,
                cards,
                copy,
            )
        {
            self.resolution = Some(*parent.resolution);
            self.pending = question;
            self.awaiting_answer = true;
            return true;
        }
        parent.resolution.awaiting = None;
        parent.resolution.pc += 1;
        self.awaiting_answer = false;
        match crate::resolve::run(&mut self.state, &mut parent.resolution) {
            Flow::Complete => self.finish_resolution(&parent.resolution),
            Flow::Enter => {
                self.resolution = Some(*parent.resolution);
                self.awaiting_answer = false;
            }
            Flow::Wait(pending) => {
                self.resolution = Some(*parent.resolution);
                self.pending = pending;
                self.awaiting_answer = true;
            }
        }
        true
    }
}
