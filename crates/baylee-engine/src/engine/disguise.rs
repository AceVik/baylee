//! Face-down disguise/morph/megamorph, its turn-face-up special action
//! (CR 702.37, CR 702.168), and graveyard keyword ability resolutions
//! (Unearth CR 702.84a, Encore CR 702.141a).
use super::{CardLookup, Engine, EngineError, GameEvent, ObjectId, PlayerId, Zone};
use crate::object::{GameObject, ObjectKind, Rider, Status};
use crate::zone::{ZoneLocation, ZonePosition};
use baylee_cards_dsl::{
    AbilityDef, Amount, CostPart, CounterKind, Duration, Effect, KeywordSet, Modifier, PlayerRel,
    TargetSpec, Trigger,
};
use baylee_core::{
    color::ColorSet,
    mana::ManaCost,
    types::{SubtypeSet, SupertypeSet, TypeSet},
};

pub(crate) static WARD: &[AbilityDef] = &[baylee_cards_dsl::triggered!(
    Trigger::Ward,
    &[Effect::PlayerMayPayOr {
        player: PlayerRel::ControllerOfTarget,
        mana: Amount::Fixed(2),
        effect: &Effect::CounterTargetSpellOrAbility,
    }]
)];

static UNEARTH_EXILE_END_STEP: &[Effect] = &[Effect::Exile {
    target: TargetSpec::EventObject,
}];

impl<L: CardLookup, D: crate::engine::LoopDetection> Engine<L, D> {
    pub(super) fn make_disguised(&mut self, id: ObjectId) {
        let nameless = self.state.names.intern("");
        let Some(o) = self.state.object_mut(id) else {
            return;
        };
        o.original_base = Some(std::sync::Arc::clone(&o.base));
        o.status.insert(Status::FACE_DOWN);
        let c = o.base_mut();
        c.name = nameless;
        c.mana_cost = ManaCost::ZERO;
        c.colors = ColorSet::EMPTY;
        c.types = TypeSet::CREATURE;
        c.supertypes = SupertypeSet::EMPTY;
        c.subtypes = SubtypeSet::EMPTY;
        c.keywords = KeywordSet::EMPTY;
        c.power = Some(2);
        c.toughness = Some(2);
        c.loyalty = None;
        c.produced_colors = ColorSet::EMPTY;
        c.produced_colorless = false;
        c.produced_chosen = false;
        o.cache.clear();
    }

    pub(super) fn turn_face_up_spec(
        &self,
        id: ObjectId,
    ) -> Option<(ManaCost, &'static [CostPart], bool)> {
        let o = self.state.object(id)?;
        if o.zone != Zone::Battlefield || !o.status.contains(Status::FACE_DOWN) {
            return None;
        }
        let face = self
            .lookup
            .card(o.card?.index)?
            .faces
            .get(usize::from(o.face_index))?;
        face.turn_face_up_cost()
    }

    pub(super) fn turn_face_up_parts_payable(
        &self,
        player: PlayerId,
        id: ObjectId,
        parts: &[CostPart],
    ) -> bool {
        if parts.is_empty() {
            return true;
        }
        let life: i32 = parts.iter().fold(0_i32, |acc, part| match part {
            CostPart::PayLife(n) => acc.saturating_add(i32::from(*n)),
            _ => acc,
        });
        if life > 0 && !self.state.can_pay_life(player, life) {
            return false;
        }
        let energy: u32 = parts.iter().fold(0_u32, |acc, part| match part {
            CostPart::PayEnergy(n) => acc.saturating_add(u32::from(*n)),
            _ => acc,
        });
        if energy > 0
            && self
                .state
                .players
                .get(usize::from(player.get()))
                .is_none_or(|p| u32::from(p.energy) < energy)
        {
            return false;
        }
        let lib_exile: usize = parts.iter().fold(0_usize, |acc, part| match part {
            CostPart::ExileTopOfLibrary(n) => acc.saturating_add(usize::from(*n)),
            _ => acc,
        });
        if lib_exile > 0 && self.state.zones.list(ZoneLocation::Library(player)).len() < lib_exile {
            return false;
        }
        parts.iter().all(|part| match part {
            CostPart::PayLife(_)
            | CostPart::PayEnergy(_)
            | CostPart::ExileTopOfLibrary(_)
            | CostPart::DiscardHand
            | CostPart::SacrificeAll(_) => true,
            CostPart::DiscardRandom => {
                !self.state.zones.list(ZoneLocation::Hand(player)).is_empty()
            }
            _ => !super::cost_wizard::options(&self.state, player, id, part).is_empty(),
        })
    }

    pub(super) fn disguise_cost(&self, id: ObjectId) -> Option<ManaCost> {
        let controller = self.state.object(id)?.controller;
        let (mana, parts, _) = self.turn_face_up_spec(id)?;
        self.turn_face_up_parts_payable(controller, id, parts)
            .then_some(mana)
    }

    pub(super) fn turn_face_up(
        &mut self,
        player: PlayerId,
        id: ObjectId,
    ) -> Result<(), EngineError> {
        let (cost, parts, _) = self
            .turn_face_up_spec(id)
            .ok_or(EngineError::IllegalAction("no disguise cost"))?;
        if self.state.object(id).is_none_or(|o| o.controller != player) {
            return Err(EngineError::IllegalAction("not your permanent"));
        }
        if !self.turn_face_up_parts_payable(player, id, parts) {
            return Err(EngineError::IllegalAction("cannot pay turn-face-up cost"));
        }
        let origin = self
            .state
            .source_identity(id)
            .ok_or(EngineError::IllegalAction("missing cost source"))?;
        if super::casting::pay_mana_for(
            &mut self.state,
            player,
            super::casting::SpendFor::OtherFrom(origin),
            &cost,
        )
        .is_none()
        {
            return Err(EngineError::IllegalAction("cannot pay disguise cost"));
        }
        self.finish_turn_face_up(player, id, 0);
        Ok(())
    }

    pub(super) fn finish_turn_face_up(&mut self, player: PlayerId, id: ObjectId, x: u32) {
        let is_megamorph = self.turn_face_up_spec(id).is_some_and(|(_, _, mega)| mega);
        let Some(o) = self.state.object_mut(id) else {
            return;
        };
        o.status.remove(Status::FACE_DOWN);
        o.x_value = x;
        if let Some(base) = o.original_base.take() {
            o.base = base;
        }
        o.cache.clear();
        self.state.invalidate_projections();
        if is_megamorph {
            // CR 702.37b: as this permanent is turned face up, put a +1/+1
            // counter on it if its megamorph cost was paid to turn it face up.
            crate::entry::put_counters(&mut self.state, player, id, CounterKind::P1P1, 1);
        }
        self.state
            .journal
            .record(GameEvent::TurnedFaceUp { object: id });
        self.after_action(player);
    }

    /// Resolves an `AbilityDef::Unearth` ability on the stack (CR 702.84a).
    pub(super) fn resolve_unearth(
        &mut self,
        controller: PlayerId,
        source: ObjectId,
        expected_version: Option<u32>,
    ) {
        let still_in_graveyard = self.state.object(source).is_some_and(|o| {
            o.zone == Zone::Graveyard
                && o.is_card()
                && expected_version.is_none_or(|v| o.version == v)
        });
        if !still_in_graveyard {
            return;
        }
        self.state
            .entries
            .begin_event(self.state.journal.last_seq(), Some(controller));
        if let Some(obj) = self.state.object_mut(source) {
            obj.set_controller(controller);
            obj.kind = ObjectKind::Permanent;
        }
        self.state.set_tapped(source, false);
        let _ = self.state.move_object(
            source,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            crate::event::Cause::Effect,
        );
        let Some(ref_id) = crate::entry::battlefield_reference(&self.state, source) else {
            return;
        };
        let version = ref_id.version;
        let filter = crate::effects::EffectFilter::ObjectIs(source, version);
        for modifier in [
            Modifier::AddKeyword(KeywordSet::HASTE),
            Modifier::ExileInsteadOfLeavingBattlefield,
        ] {
            let ts = self.state.next_timestamp();
            self.state
                .effects
                .register(crate::effects::ContinuousEffect {
                    id: baylee_core::ids::EffectId::new(0),
                    source: Some(source),
                    controller,
                    origin: crate::effects::EffectOrigin::Resolution,
                    layer: modifier.layer(),
                    timestamp: ts,
                    duration: Duration::Indefinitely,
                    filter,
                    modifier,
                });
        }
        self.state.delayed.push(crate::state::DelayedTrigger {
            controller,
            when: crate::state::DelayedWhen::NextEndStep,
            action: crate::state::DelayedAction::TriggerAbout {
                source,
                source_version: version,
                effects: UNEARTH_EXILE_END_STEP,
                text: crate::text_changes::TextChangeMap::IDENTITY,
                object: source,
                version,
                subject: None,
            },
        });
        self.state.invalidate_projections();
    }

    /// Resolves an `AbilityDef::Encore` ability on the stack (CR 702.141a).
    pub(super) fn resolve_encore(&mut self, controller: PlayerId, source: ObjectId) {
        if self.state.has_left(controller) {
            return;
        }
        let Some(base) = crate::layers::copiable_values(&self.state, source).or_else(|| {
            self.state
                .ltb_characteristics
                .iter()
                .find(|(id, _)| *id == source)
                .map(|(_, c)| std::sync::Arc::new(c.clone()))
        }) else {
            return;
        };
        let opponents: Vec<PlayerId> = self
            .state
            .players
            .iter()
            .filter(|p| self.state.is_opponent(p.id, controller) && !p.has_lost())
            .map(|p| p.id)
            .collect();
        if opponents.is_empty() {
            return;
        }
        self.state
            .entries
            .begin_event(self.state.journal.last_seq(), Some(controller));
        let own = self
            .state
            .object(source)
            .map(|o| o.printed_ability_list(&self.lookup))
            .or_else(|| self.state.printed_ability_list(source));
        let face = self
            .state
            .object(source)
            .and_then(|o| o.card.map(|c| (c.index, o.face_index)));
        for opponent in opponents {
            let Some(count) =
                crate::replacement::token_multiplier(&self.state, controller, base.types)
            else {
                self.state.numeric_failure = Some("token creation exceeds u32 objects");
                return;
            };
            for _ in 0..count {
                self.create_encore_token(controller, opponent, &base, own.as_ref(), face);
            }
        }
    }

    fn create_encore_token(
        &mut self,
        controller: PlayerId,
        opponent: PlayerId,
        base: &std::sync::Arc<crate::object::Characteristics>,
        own: Option<&crate::object::AbilityList>,
        face: Option<(baylee_core::ids::CardIndex, u8)>,
    ) {
        let ts = self.state.next_timestamp();
        let id = self.state.arena.insert_with(|oid| {
            let mut obj =
                GameObject::new_bare(oid, controller, ObjectKind::Permanent, base.clone());
            obj.timestamp = ts;
            obj.controlled_since = ts;
            if let Some(own) = own {
                obj.take_abilities(own.clone());
            }
            obj.riders.push(Rider::MustAttackPlayerThisTurn(opponent));
            obj.zone = Zone::OutsideGame;
            obj
        });
        if let Some((card, face_idx)) = face {
            self.state.pending_copied_faces.push((id, card, face_idx));
        }
        if !crate::entry::defer(
            &mut self.state,
            id,
            ZonePosition::Top,
            crate::event::Cause::Effect,
        ) {
            self.state
                .zones
                .insert(id, ZoneLocation::Battlefield, ZonePosition::Top, true);
            if let Some(obj) = self.state.object_mut(id) {
                obj.zone = Zone::Battlefield;
            }
            self.state.per_turn.entered_battlefield.push(id);
            self.state.note_entry_history(id);
            self.state.journal.record(GameEvent::ZoneChanged {
                object: id,
                from: Zone::OutsideGame,
                to: Zone::Battlefield,
                cause: crate::event::Cause::Effect,
                place: None,
            });
        }
        let Some(ref_id) = crate::entry::battlefield_reference(&self.state, id) else {
            return;
        };
        let version = ref_id.version;
        let filter = crate::effects::EffectFilter::ObjectIs(id, version);
        let ets = self.state.next_timestamp();
        self.state
            .effects
            .register(crate::effects::ContinuousEffect {
                id: baylee_core::ids::EffectId::new(0),
                source: Some(id),
                controller,
                origin: crate::effects::EffectOrigin::Resolution,
                layer: baylee_cards_dsl::Layer::Ability,
                timestamp: ets,
                duration: Duration::Indefinitely,
                filter,
                modifier: Modifier::AddKeyword(KeywordSet::HASTE),
            });
        self.state.delayed.push(crate::state::DelayedTrigger {
            controller,
            when: crate::state::DelayedWhen::NextEndStep,
            action: crate::state::DelayedAction::Sacrifice { card: id, version },
        });
        self.state.invalidate_projections();
    }
}
