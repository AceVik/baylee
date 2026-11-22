//! Wave 10b spell-casting mechanics (`CR 702.47` Splice, `CR 702.113` Awaken, `CR 702.78` Conspire).

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::object::{Characteristics, GameObject, Rider};
use baylee_cards_dsl::{CardDef, Cost, Effect, Filter, KeywordSet};

pub(crate) static AWAKEN_LAND_REQ: TargetReq =
    TargetReq::one(TargetSpec::Object(&Filter::YOUR_LAND));

pub(crate) fn face_awaken(def: &CardDef, face_idx: usize) -> Option<(u16, Cost)> {
    let idx = face_idx.min(def.faces.len().saturating_sub(1));
    def.abilities_for_face(idx)
        .iter()
        .find_map(|ability| match ability {
            AbilityDef::Awaken { count, cost } => Some((*count, *cost)),
            _ => None,
        })
}

pub(crate) fn awaken_has_legal_land(
    state: &crate::state::GameState,
    player: PlayerId,
    spell: ObjectId,
) -> bool {
    !eval::target_options(&AWAKEN_LAND_REQ.spec, state, player, spell).is_empty()
}

pub(crate) fn splice_ability_for(
    def: &'static CardDef,
    face_idx: usize,
) -> Option<(&'static Filter, Cost)> {
    let idx = face_idx.min(def.faces.len().saturating_sub(1));
    def.abilities_for_face(idx)
        .iter()
        .find_map(|ability| match ability {
            AbilityDef::Splice { onto, cost } => Some((*onto, *cost)),
            _ => None,
        })
}

pub(crate) fn spell_face_parts(
    def: &'static CardDef,
    face_idx: usize,
) -> Option<(&'static [Effect], Option<TargetReq>, Option<TargetReq>)> {
    let idx = face_idx.min(def.faces.len().saturating_sub(1));
    def.abilities_for_face(idx)
        .iter()
        .find_map(|ability| match ability {
            AbilityDef::Spell {
                effects,
                targets,
                second_targets,
                ..
            } => Some((*effects, *targets, *second_targets)),
            _ => None,
        })
}

fn push_base_clauses(
    out: &mut Vec<(usize, bool, TargetReq)>,
    def: &'static CardDef,
    face_idx: usize,
    option: Option<CastModeKind>,
    kicked: bool,
) {
    let abilities = def.abilities_for_face(face_idx);
    match option {
        Some(CastModeKind::Modes(set)) => {
            out.extend(casting::modal_target_clauses(abilities, set));
        }
        Some(CastModeKind::RepeatedModes(packed)) => {
            out.extend(casting::repeated_modal_target_clauses(abilities, packed));
        }
        Some(CastModeKind::Mode(i)) => {
            if let Some(mode) = abilities.iter().find_map(|a| a.modal_spell()?.0.get(i)) {
                if let Some(r) = mode.targets {
                    out.push((i, false, r));
                }
                if let Some(r) = mode.second_targets {
                    out.push((i, true, r));
                }
            }
        }
        Some(CastModeKind::Fused) => {
            for f in 0..def.faces.len().min(2) {
                if let Some((_, t1, t2)) = spell_face_parts(def, f) {
                    if let Some(r) = t1 {
                        out.push((f, false, r));
                    }
                    if let Some(r) = t2 {
                        out.push((f, true, r));
                    }
                }
            }
        }
        _ => {
            if kicked && let Some(req) = def.faces[face_idx].kicked_targets {
                out.push((0, false, req));
                return;
            }
            if let Some((_, t1, t2)) = spell_face_parts(def, face_idx) {
                if let Some(r) = t1 {
                    out.push((0, false, r));
                }
                if let Some(r) = t2 {
                    out.push((0, true, r));
                }
            }
        }
    }
}

impl<L: CardLookup, D: crate::engine::LoopDetection> Engine<L, D> {
    fn prospective_wizard_spell(&self, wizard: &CastWizard) -> Option<GameObject> {
        let obj = self.state.object(wizard.card)?;
        let def = obj.card.and_then(|c| self.lookup.card(c.index))?;
        let face_idx = wizard.face_index().min(def.faces.len().saturating_sub(1));
        let mut spell = obj.clone();
        if wizard.option != Some(CastModeKind::Fused) {
            spell.base =
                std::sync::Arc::new(Characteristics::from_face(def, face_idx, obj.base.name));
            spell.face_index = u8::try_from(face_idx).unwrap_or(0);
        }
        spell.kind = ObjectKind::Spell;
        spell.zone = crate::zone::Zone::Stack;
        spell.set_controller(wizard.player);
        let plan = crate::layers::LayerPlan::build(&self.state.effects);
        let projected = crate::layers::recompute_with(&self.state, &spell, &plan);
        let base = spell.base.clone();
        spell.cache.store(projected.characteristics, &base);
        Some(spell)
    }

    fn req_has_legal_targets(&self, wizard: &CastWizard, req: TargetReq) -> bool {
        let (min, max) = req.bounds(wizard.x);
        if min == 0 || max == 0 {
            return true;
        }
        let objs = self.wizard_target_options(wizard, &req.spec);
        let players = eval::target_player_options_from(
            &self.state,
            &req.spec,
            wizard.player,
            eval::live_context(&self.state, wizard.card),
        );
        objs.len() + players.len() >= min as usize
    }

    pub(super) fn ask_splice(&mut self, wizard: &CastWizard) -> Option<Pending> {
        if wizard.masked
            || wizard.option == Some(CastModeKind::Disguise)
            || wizard.spliced.len() >= 8
        {
            return None;
        }
        let spell = self.prospective_wizard_spell(wizard)?;
        let hand = self
            .state
            .zones
            .list(ZoneLocation::Hand(wizard.player))
            .clone();
        let mut options = Vec::new();
        for card_id in hand {
            if card_id == wizard.card || wizard.spliced.iter().any(|(id, ..)| *id == card_id) {
                continue;
            }
            let Some(card_obj) = self.state.object(card_id) else {
                continue;
            };
            if card_obj.characteristics().rules_text_lost
                || card_obj.characteristics().abilities_lost.is_some()
            {
                continue;
            }
            let Some(card_ref) = card_obj.card else {
                continue;
            };
            let Some(def) = self.lookup.card(card_ref.index) else {
                continue;
            };
            let face_idx = usize::from(card_obj.face_index);
            let Some((onto, cost)) = splice_ability_for(def, face_idx) else {
                continue;
            };
            let Some((_, t1, t2)) = spell_face_parts(def, face_idx) else {
                continue;
            };
            if !eval::matches_with_context(
                onto,
                &self.state,
                &spell,
                wizard.player,
                eval::live_context(&self.state, card_id),
            ) {
                continue;
            }
            if t1.is_some_and(|r| !self.req_has_legal_targets(wizard, r))
                || t2.is_some_and(|r| !self.req_has_legal_targets(wizard, r))
            {
                continue;
            }
            let mut probe = wizard.clone();
            probe
                .spliced
                .push((card_id, card_ref.index, card_obj.face_index, cost));
            if self.announced_cost_payable(&probe) {
                options.push(card_id);
            }
        }
        (!options.is_empty()).then_some(Pending::ChooseCards {
            player: wizard.player,
            options,
            min: 0,
            max: 1,
            prompt: ChoicePrompt::Splice,
            total: None,
        })
    }

    pub(super) fn ask_conspire(&mut self, wizard: &CastWizard) -> Option<Pending> {
        if wizard.masked || wizard.option == Some(CastModeKind::Disguise) {
            return None;
        }
        let spell = self.prospective_wizard_spell(wizard)?;
        if !spell
            .characteristics()
            .keywords
            .contains(KeywordSet::CONSPIRE)
        {
            return None;
        }
        let total = crate::layers::keyword_instances(&self.state, &spell, KeywordSet::CONSPIRE);
        if u32::from(wizard.conspire_cursor) >= total {
            return None;
        }
        let mut probe = wizard.clone();
        probe.conspired = probe.conspired.saturating_add(1);
        if !self.announced_cost_payable(&probe) {
            return None;
        }
        let source = self.state.object(wizard.card).and_then(|object| {
            object.card.map(|card| {
                baylee_core::ids::AbilityRef::new(
                    card.index,
                    baylee_core::ids::AbilityRef::ADDITIONAL_COST,
                )
            })
        });
        Some(Pending::YesNo {
            player: wizard.player,
            prompt: YesNoPrompt::Conspire,
            source,
        })
    }

    pub(crate) fn extended_wizard_clauses(
        &self,
        wizard: &CastWizard,
    ) -> Option<Vec<(usize, bool, TargetReq)>> {
        if wizard.masked
            || (wizard.option != Some(CastModeKind::Awaken) && wizard.spliced.is_empty())
        {
            return None;
        }
        let def = self
            .state
            .object(wizard.card)
            .and_then(|o| o.card)
            .and_then(|c| self.lookup.card(c.index))?;
        let face_idx = wizard.face_index().min(def.faces.len().saturating_sub(1));
        let mut out = Vec::new();
        push_base_clauses(&mut out, def, face_idx, wizard.option, wizard.kicked);
        let mut seg = 100_usize;
        if wizard.option == Some(CastModeKind::Awaken) {
            out.push((seg, false, AWAKEN_LAND_REQ));
            seg += 1;
        }
        for &(_, card_idx, face, _) in &wizard.spliced {
            if let Some(s_def) = self.lookup.card(card_idx)
                && let Some((_, t1, t2)) = spell_face_parts(s_def, usize::from(face))
            {
                if let Some(r) = t1 {
                    out.push((seg, false, r));
                }
                if let Some(r) = t2 {
                    out.push((seg, true, r));
                }
                seg += 1;
            }
        }
        Some(out)
    }

    pub(crate) fn extended_stack_clauses(
        &self,
        obj: &GameObject,
    ) -> Option<Vec<(usize, bool, TargetReq)>> {
        let awakened = obj.riders.iter().any(|r| matches!(r, Rider::Awakened(_)));
        let has_spliced = obj
            .riders
            .iter()
            .any(|r| matches!(r, Rider::Spliced { .. }));
        if !awakened && !has_spliced {
            return None;
        }
        let def = obj.card.and_then(|c| self.lookup.card(c.index))?;
        let face_idx = usize::from(obj.face_index).min(def.faces.len().saturating_sub(1));
        let option = if obj.riders.contains(&Rider::Fused) {
            Some(CastModeKind::Fused)
        } else if let Some(packed) = obj.repeated_modes() {
            Some(CastModeKind::RepeatedModes(packed))
        } else if obj.modes != 0 {
            Some(CastModeKind::Modes(obj.modes))
        } else {
            obj.mode_index.map(|i| CastModeKind::Mode(usize::from(i)))
        };
        let mut out = Vec::new();
        push_base_clauses(&mut out, def, face_idx, option, obj.kicked());
        let mut seg = 100_usize;
        if awakened {
            out.push((seg, false, AWAKEN_LAND_REQ));
            seg += 1;
        }
        for rider in &obj.riders {
            if let Rider::Spliced { card, face } = *rider
                && let Some(s_def) = self.lookup.card(card)
                && let Some((_, t1, t2)) = spell_face_parts(s_def, usize::from(face))
            {
                if let Some(r) = t1 {
                    out.push((seg, false, r));
                }
                if let Some(r) = t2 {
                    out.push((seg, true, r));
                }
                seg += 1;
            }
        }
        Some(out)
    }

    #[allow(clippy::type_complexity)]
    pub(crate) fn extended_stack_program(
        &self,
        obj: &GameObject,
    ) -> Option<(
        Vec<Effect>,
        bool,
        Option<usize>,
        Option<crate::resolve::ModalTargetStep>,
        Vec<crate::resolve::ModalTargetStep>,
    )> {
        let awaken_count = obj.riders.iter().find_map(|r| match *r {
            Rider::Awakened(count) => Some(count),
            _ => None,
        });
        let has_spliced = obj
            .riders
            .iter()
            .any(|r| matches!(r, Rider::Spliced { .. }));
        if awaken_count.is_none() && !has_spliced {
            return None;
        }
        let def = obj.card.and_then(|c| self.lookup.card(c.index))?;
        let mut segments: Vec<(Vec<Effect>, bool, bool)> = Vec::new();
        push_base_segments(&mut segments, obj, def);
        if let Some(count) = awaken_count {
            segments.push((vec![Effect::AwakenLand { count }], true, false));
        }
        for rider in &obj.riders {
            if let Rider::Spliced { card, face } = *rider
                && let Some(s_def) = self.lookup.card(card)
                && let Some((fx, t1, t2)) = spell_face_parts(s_def, usize::from(face))
            {
                segments.push((crate::resolve::flatten(fx), t1.is_some(), t2.is_some()));
            }
        }
        Some(assemble_segment_steps(obj, segments))
    }
}

fn push_base_segments(
    segments: &mut Vec<(Vec<Effect>, bool, bool)>,
    obj: &GameObject,
    def: &'static CardDef,
) {
    let face_idx = usize::from(obj.face_index).min(def.faces.len().saturating_sub(1));
    let abilities = def.abilities_for_face(face_idx);
    if obj.riders.contains(&Rider::Fused) {
        for f in 0..def.faces.len().min(2) {
            if let Some((fx, t1, t2)) = spell_face_parts(def, f) {
                segments.push((crate::resolve::flatten(fx), t1.is_some(), t2.is_some()));
            }
        }
        return;
    }
    if let Some(modes) = abilities.iter().find_map(|a| a.modal_spell().map(|t| t.0)) {
        if let Some(packed) = obj.repeated_modes() {
            for (_, m) in casting::chosen_repeated_modes(modes, packed) {
                segments.push((
                    crate::resolve::flatten(m.effects),
                    m.targets.is_some(),
                    m.second_targets.is_some(),
                ));
            }
        } else if obj.modes != 0 {
            for (_, m) in casting::chosen_modes(modes, obj.modes) {
                segments.push((
                    crate::resolve::flatten(m.effects),
                    m.targets.is_some(),
                    m.second_targets.is_some(),
                ));
            }
        } else if let Some(m) = obj.mode_index.and_then(|i| modes.get(usize::from(i))) {
            segments.push((
                crate::resolve::flatten(m.effects),
                m.targets.is_some(),
                m.second_targets.is_some(),
            ));
        }
        return;
    }
    if let Some((fx, t1, t2)) = spell_face_parts(def, face_idx) {
        let has_t1 = t1.is_some() || (obj.kicked() && def.faces[face_idx].kicked_targets.is_some());
        segments.push((crate::resolve::flatten(fx), has_t1, t2.is_some()));
    }
}

#[allow(clippy::type_complexity)]
fn assemble_segment_steps(
    obj: &GameObject,
    segments: Vec<(Vec<Effect>, bool, bool)>,
) -> (
    Vec<Effect>,
    bool,
    Option<usize>,
    Option<crate::resolve::ModalTargetStep>,
    Vec<crate::resolve::ModalTargetStep>,
) {
    let first_players = eval::first_target_players(obj);
    let mut stream: Vec<(SmallVec<[ObjectId; 2]>, baylee_core::ids::SeatSet)> =
        Vec::with_capacity(2 + obj.extra_target_instances().len());
    stream.push((obj.targets.clone(), first_players));
    stream.push((
        SmallVec::from_slice(obj.second_targets()),
        obj.second_target_players(),
    ));
    for inst in obj.extra_target_instances() {
        stream.push((inst.targets.clone(), inst.players));
    }
    let mut cursor = 0_usize;
    let mut take = || {
        let item = stream
            .get(cursor)
            .cloned()
            .unwrap_or_else(|| (SmallVec::new(), baylee_core::ids::SeatSet::new()));
        cursor += 1;
        item
    };
    let any_targeted = segments.iter().any(|(_, t1, t2)| *t1 || *t2);
    let mut program = Vec::new();
    let mut initial = None;
    let mut staged = Vec::new();
    for (fx, has_t1, has_t2) in segments {
        if any_targeted {
            let (targets, target_players) = if has_t1 {
                take()
            } else {
                (SmallVec::new(), baylee_core::ids::SeatSet::new())
            };
            let (sec_objs, second_players) = if has_t2 {
                take()
            } else {
                (SmallVec::new(), baylee_core::ids::SeatSet::new())
            };
            let step = crate::resolve::ModalTargetStep {
                left_from_end: program.len(),
                targets,
                target_players,
                second_targets: sec_objs.into_iter().collect(),
                second_players,
            };
            if initial.is_none() {
                initial = Some(step);
            } else {
                staged.push(step);
            }
        }
        program.extend(fx);
    }
    let total_len = program.len();
    for step in &mut staged {
        step.left_from_end = total_len - step.left_from_end;
    }
    let targeted = initial.is_some();
    (program, targeted, None, initial, staged)
}
