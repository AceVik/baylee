//! Face-down disguise and its face-up special action (CR 702.168).
use super::{CardLookup, Engine, EngineError, GameEvent, ObjectId, PlayerId, Zone};
use crate::object::Status;
use baylee_cards_dsl::{AbilityDef, Amount, Effect, KeywordSet, PlayerRel, Trigger};
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

impl<L: CardLookup> Engine<L> {
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

    pub(super) fn disguise_cost(&self, id: ObjectId) -> Option<ManaCost> {
        let o = self.state.object(id)?;
        if o.zone != Zone::Battlefield || !o.status.contains(Status::FACE_DOWN) {
            return None;
        }
        self.lookup
            .card(o.card?.index)?
            .faces
            .get(usize::from(o.face_index))?
            .disguise
    }

    pub(super) fn turn_face_up(
        &mut self,
        player: PlayerId,
        id: ObjectId,
    ) -> Result<(), EngineError> {
        let cost = self
            .disguise_cost(id)
            .ok_or(EngineError::IllegalAction("no disguise cost"))?;
        if self.state.object(id).is_none_or(|o| o.controller != player) {
            return Err(EngineError::IllegalAction("not your permanent"));
        }
        if super::casting::pay_mana_for(
            &mut self.state,
            player,
            super::casting::SpendFor::Other,
            &cost,
        )
        .is_none()
        {
            return Err(EngineError::IllegalAction("cannot pay disguise cost"));
        }
        let o = self.state.object_mut(id).expect("checked permanent");
        o.status.remove(Status::FACE_DOWN);
        if let Some(base) = o.original_base.take() {
            o.base = base;
        }
        o.cache.clear();
        self.state.invalidate_projections();
        self.state
            .journal
            .record(GameEvent::TurnedFaceUp { object: id });
        self.after_action(player);
        Ok(())
    }
}
