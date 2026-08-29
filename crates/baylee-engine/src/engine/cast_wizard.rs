//! The casting wizard: multi-step spell casting (CR 601.2a–h).
//!
//! Modes/alternative costs → X → targets → kicker → pitch choices →
//! payment. Each step suspends into a pending request; the wizard resumes
//! on the answer. Everything ends in one `SpellCast` event — atomic from
//! the outside.

use super::{
    AbilityDef, CardLookup, Cause, Engine, EngineError, GameEvent, ObjectId, ObjectKind, Pending,
    PlayerId, SmallVec, Zone, ZoneLocation, ZonePosition, eval, mana_pay,
};
use crate::choice::{CastModeDesc, CastModeKind, ChoicePrompt, YesNoPrompt};
use baylee_cards_dsl::{AltCondition, CostPart, SpellMode, TargetReq, TargetSpec};
use baylee_core::mana::ManaCost;

/// Where the wizard currently is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum WizardStage {
    /// Choosing a cast mode (skip when only one option).
    ChooseMode,
    /// Choosing X.
    XValue,
    /// Choosing targets.
    Targets,
    /// Choosing a target player.
    ChoosePlayer,
    /// Kicker yes/no.
    Kicker,
    /// Pitch choice (exile-from-hand).
    PitchChoice,
    /// Ready to pay and cast.
    Done,
}

/// A spell being cast step by step.
#[derive(Clone, Debug)]
pub(crate) struct CastWizard {
    /// The card being cast.
    pub card: ObjectId,
    /// The casting player.
    pub player: PlayerId,
    /// Chosen cast option.
    pub option: Option<CastModeKind>,
    /// Chosen targets.
    pub targets: SmallVec<[ObjectId; 2]>,
    /// Chosen target player, if any.
    pub chosen_player: Option<PlayerId>,
    /// Chosen X.
    pub x: u32,
    /// Whether the kicker was taken.
    pub kicked: bool,
    /// Cards chosen for pitch (exile-from-hand).
    pub pitch: SmallVec<[ObjectId; 2]>,
    /// Current stage.
    pub stage: WizardStage,
    /// Options computed at start (kept for the Done stage).
    pub options: Vec<CastModeDesc>,
    /// Whether this cast is free (rebound, suspend finish).
    pub free: bool,
}

impl<L: CardLookup> Engine<L> {
    /// Starts the casting wizard for `card` (validated castable).
    pub(crate) fn start_cast_wizard(
        &mut self,
        player: PlayerId,
        card: ObjectId,
    ) -> Result<(), EngineError> {
        let options = self.cast_options(player, card)?;
        let mut wizard = CastWizard {
            card,
            player,
            option: None,
            targets: SmallVec::new(),
            chosen_player: None,
            x: 0,
            kicked: false,
            pitch: SmallVec::new(),
            stage: WizardStage::ChooseMode,
            options,
            free: false,
        };
        if wizard.options.len() == 1 {
            wizard.option = Some(wizard.options[0].kind);
            wizard.stage = WizardStage::XValue;
        }
        self.cast_wizard = Some(wizard);
        self.advance_cast_wizard()
    }

    /// Starts a miracle cast (CR 702.94): timing-free, at the miracle
    /// cost; targets and other choices still run through the wizard.
    pub(crate) fn start_miracle_cast(
        &mut self,
        player: PlayerId,
        card: ObjectId,
    ) -> Result<(), EngineError> {
        let cost = self
            .state
            .object(card)
            .and_then(|o| o.card)
            .and_then(|c| self.lookup.card(c.index))
            .and_then(|def| def.faces[0].miracle)
            .ok_or(EngineError::IllegalAction("not a miracle card"))?;
        let options = vec![CastModeDesc {
            index: 0,
            kind: CastModeKind::Miracle,
            cost,
        }];
        let mut wizard = CastWizard {
            card,
            player,
            option: Some(CastModeKind::Miracle),
            targets: SmallVec::new(),
            chosen_player: None,
            x: 0,
            kicked: false,
            pitch: SmallVec::new(),
            stage: WizardStage::Targets,
            options,
            free: false,
        };
        let _ = &mut wizard;
        self.cast_wizard = Some(wizard);
        self.advance_cast_wizard()
    }

    /// Starts a free cast (rebound at upkeep, suspend finish): no payment,
    /// but targets and other choices still run through the wizard.
    pub(crate) fn start_free_cast(
        &mut self,
        player: PlayerId,
        card: ObjectId,
    ) -> Result<(), EngineError> {
        let mut wizard = CastWizard {
            card,
            player,
            option: Some(CastModeKind::Normal),
            targets: SmallVec::new(),
            chosen_player: None,
            x: 0,
            kicked: false,
            pitch: SmallVec::new(),
            stage: WizardStage::Targets,
            options: Vec::new(),
            free: true,
        };
        let _ = &mut wizard;
        self.cast_wizard = Some(wizard);
        self.advance_cast_wizard()
    }

    /// All legal ways to cast `card` right now.
    fn cast_options(
        &self,
        player: PlayerId,
        card: ObjectId,
    ) -> Result<Vec<CastModeDesc>, EngineError> {
        let obj = self
            .state
            .object(card)
            .ok_or(EngineError::IllegalAction("no such card"))?;
        let card_ref = obj
            .card
            .ok_or(EngineError::IllegalAction("not card-backed"))?;
        let def = self
            .lookup
            .card(card_ref.index)
            .ok_or(EngineError::IllegalAction("unknown card"))?;
        let face = &def.faces[0];
        let pool = &self.state.players[player.get() as usize].mana_pool;
        let mut options = Vec::new();
        // Normal cost (X probed with 0; the real check happens at payment).
        if mana_pay::can_pay(pool, &face.mana_cost.with_x(0)) {
            options.push(CastModeDesc {
                index: 0,
                kind: CastModeKind::Normal,
                cost: face.mana_cost,
            });
        }
        // Alternative costs (pitch, evoke, conditional free).
        for (i, alt) in face.alternative_costs.iter().enumerate() {
            let condition_ok = match alt.condition {
                AltCondition::Always => true,
                AltCondition::NotYourTurn => self.state.turn.active != player,
                AltCondition::CommanderControlled => self.has_commander_on_battlefield(player),
            };
            if !condition_ok || !self.can_afford(player, card, &alt.cost) {
                continue;
            }
            options.push(CastModeDesc {
                index: (options.len()) as u8,
                kind: CastModeKind::Alternative(i),
                cost: alt.cost.mana,
            });
        }
        // MDFC: castable non-front faces (non-land backs, CR 712.4).
        for (i, back) in def.faces.iter().enumerate().skip(1) {
            if back.types.contains(baylee_core::types::TypeSet::LAND) || !back.castable_from_hand {
                continue; // land faces are played; disturb backs come from the graveyard
            }
            if mana_pay::can_pay(pool, &back.mana_cost.with_x(0)) {
                options.push(CastModeDesc {
                    index: (options.len()) as u8,
                    kind: CastModeKind::Face(i),
                    cost: back.mana_cost,
                });
            }
        }
        // Modal spells (overload & friends): one option per mode.
        for ability in def.abilities {
            let AbilityDef::ModalSpell { modes } = ability else {
                continue;
            };
            for (i, mode) in modes.iter().enumerate() {
                let cost = mode.cost_override.unwrap_or(face.mana_cost);
                if mana_pay::can_pay(pool, &cost.with_x(0)) {
                    options.push(CastModeDesc {
                        index: (options.len()) as u8,
                        kind: CastModeKind::Mode(i),
                        cost,
                    });
                }
            }
        }
        if options.is_empty() {
            return Err(EngineError::IllegalAction("no way to cast this spell"));
        }
        Ok(options)
    }

    fn has_commander_on_battlefield(&self, player: PlayerId) -> bool {
        self.state
            .zones
            .list(ZoneLocation::Command(player))
            .iter()
            .any(|id| {
                self.state
                    .object(*id)
                    .is_some_and(|o| o.zone == Zone::Battlefield && o.controller == player)
            })
    }

    /// Drives the wizard forward until it needs an answer or finishes.
    #[allow(clippy::too_many_lines)] // the wizard is a flat stage machine; extraction would obscure it
    pub(crate) fn advance_cast_wizard(&mut self) -> Result<(), EngineError> {
        let Some(wizard) = self.cast_wizard.clone() else {
            return Ok(());
        };
        match wizard.stage {
            WizardStage::ChooseMode => {
                self.pending = Pending::ChooseCastMode {
                    player: wizard.player,
                    options: wizard.options.clone(),
                };
                self.awaiting_answer = true;
                Ok(())
            }
            WizardStage::XValue => {
                let cost = wizard_cost(&wizard);
                // X is asked when the cost has a variable OR a mandatory
                // part scales with it (Toxic Deluge's pay-X-life).
                let needs_x = cost.has_variable()
                    || self
                        .wizard_face(&wizard)
                        .mandatory_additional_costs
                        .contains(&CostPart::PayLifeX);
                if needs_x {
                    self.pending = Pending::ChooseNumber {
                        player: wizard.player,
                        min: 0,
                        max: 50,
                    };
                    self.awaiting_answer = true;
                    return Ok(());
                }
                let mut wizard = wizard;
                wizard.stage = WizardStage::Targets;
                self.cast_wizard = Some(wizard);
                self.advance_cast_wizard()
            }
            WizardStage::Targets => {
                let Some(req) = self.wizard_target_req(&wizard) else {
                    let mut wizard = wizard;
                    wizard.stage = WizardStage::Kicker;
                    self.cast_wizard = Some(wizard);
                    return self.advance_cast_wizard();
                };
                let (mut min, mut max) = (req.min, req.max);
                if req.count_is_x {
                    min = wizard.x as u8;
                    max = wizard.x as u8;
                }
                let spec = req.spec;
                if matches!(spec, TargetSpec::AnyPlayer) {
                    let mut wizard = wizard;
                    wizard.stage = WizardStage::ChoosePlayer;
                    self.cast_wizard = Some(wizard);
                    return self.advance_cast_wizard();
                }
                if max == 0 {
                    // Zero required targets (e.g. X = 0): skip targeting.
                    let mut wizard = wizard;
                    wizard.stage = WizardStage::Kicker;
                    self.cast_wizard = Some(wizard);
                    return self.advance_cast_wizard();
                }
                let options = eval::target_options(&spec, &self.state, wizard.player, wizard.card);
                if options.len() < min as usize {
                    self.cast_wizard = None;
                    return Err(EngineError::IllegalAction("not enough legal targets"));
                }
                self.pending = Pending::ChooseTargets {
                    player: wizard.player,
                    options,
                    min,
                    max,
                };
                self.awaiting_answer = true;
                Ok(())
            }
            WizardStage::ChoosePlayer => {
                let options: Vec<PlayerId> = self
                    .state
                    .players
                    .iter()
                    .filter(|p| !p.has_lost)
                    .map(|p| p.id)
                    .collect();
                self.pending = Pending::ChoosePlayer {
                    player: wizard.player,
                    options,
                };
                self.awaiting_answer = true;
                Ok(())
            }
            WizardStage::Kicker => {
                let face = self.wizard_face(&wizard);
                if face.additional_costs.is_empty() {
                    let mut wizard = wizard;
                    wizard.stage = WizardStage::PitchChoice;
                    self.cast_wizard = Some(wizard);
                    return self.advance_cast_wizard();
                }
                self.pending = Pending::YesNo {
                    player: wizard.player,
                    prompt: YesNoPrompt::Kicker,
                };
                self.awaiting_answer = true;
                Ok(())
            }
            WizardStage::PitchChoice => {
                let filter = self.wizard_pitch_filter(&wizard);
                if let Some(filter) = filter {
                    let options: Vec<ObjectId> = self
                        .state
                        .zones
                        .list(ZoneLocation::Hand(wizard.player))
                        .iter()
                        .copied()
                        .filter(|id| {
                            *id != wizard.card
                                && self.state.object(*id).is_some_and(|o| {
                                    eval::matches(
                                        filter,
                                        &self.state,
                                        o,
                                        wizard.player,
                                        wizard.card,
                                    )
                                })
                        })
                        .collect();
                    if options.is_empty() {
                        self.cast_wizard = None;
                        return Err(EngineError::IllegalAction(
                            "no card to exile for the pitch cost",
                        ));
                    }
                    self.pending = Pending::ChooseCards {
                        player: wizard.player,
                        options,
                        min: 1,
                        max: 1,
                        prompt: ChoicePrompt::Generic,
                    };
                    self.awaiting_answer = true;
                    return Ok(());
                }
                let mut wizard = wizard;
                wizard.stage = WizardStage::Done;
                self.cast_wizard = Some(wizard);
                self.advance_cast_wizard()
            }
            WizardStage::Done => self.finish_cast(&wizard),
        }
    }

    fn wizard_face(&self, wizard: &CastWizard) -> &'static baylee_cards_dsl::FaceDef {
        let card = self
            .state
            .object(wizard.card)
            .expect("wizard card exists")
            .card
            .expect("wizard card is card-backed");
        let def = self.lookup.card(card.index).expect("wizard card known");
        let face_index = match wizard.option {
            Some(CastModeKind::Face(i)) => i.min(def.faces.len() - 1),
            _ => 0,
        };
        &def.faces[face_index]
    }

    fn wizard_target_req(&self, wizard: &CastWizard) -> Option<TargetReq> {
        let def = self
            .state
            .object(wizard.card)
            .and_then(|o| o.card)
            .and_then(|c| self.lookup.card(c.index))
            .expect("wizard card known");
        let face_index = match wizard.option {
            Some(CastModeKind::Face(i)) => i.min(def.faces.len() - 1),
            _ => 0,
        };
        let abilities = def.abilities_for_face(face_index);
        match wizard.option {
            Some(CastModeKind::Mode(i)) => abilities.iter().find_map(|a| match a {
                AbilityDef::ModalSpell { modes } => modes
                    .get(i)
                    .and_then(|m: &SpellMode| m.target.map(TargetReq::one)),
                _ => None,
            }),
            _ => abilities.iter().find_map(|a| match a {
                AbilityDef::Spell { targets, .. } => *targets,
                _ => None,
            }),
        }
    }

    fn wizard_pitch_filter(
        &self,
        wizard: &CastWizard,
    ) -> Option<&'static baylee_cards_dsl::Filter> {
        let Some(CastModeKind::Alternative(i)) = wizard.option else {
            return None;
        };
        let face = self.wizard_face(wizard);
        face.alternative_costs.get(i).and_then(|alt| {
            alt.cost.parts.iter().find_map(|p| match p {
                CostPart::ExileFromHand(f) => Some(*f),
                _ => None,
            })
        })
    }

    /// Pays everything and puts the spell on the stack.
    #[allow(clippy::too_many_lines)] // payment is a flat checklist; extraction would obscure it
    fn finish_cast(&mut self, wizard: &CastWizard) -> Result<(), EngineError> {
        let face = self.wizard_face(wizard);
        // Total mana: option cost (with X) + kicker mana when taken.
        let mut total = wizard_cost(wizard);
        if wizard.kicked {
            for add in face.additional_costs {
                total = total.combine(&add.mana);
            }
        }
        let player = wizard.player;
        if !wizard.free
            && !mana_pay::pay(
                &mut self.state.players[player.get() as usize].mana_pool,
                &total,
            )
        {
            self.cast_wizard = None;
            return Err(EngineError::IllegalAction("cannot pay the total cost"));
        }
        // Non-mana parts of the chosen alternative cost (pay life etc.).
        if let Some(CastModeKind::Alternative(i)) = wizard.option {
            let alt = &face.alternative_costs[i];
            for part in alt.cost.parts {
                match part {
                    CostPart::PayLife(n) => {
                        let p = &mut self.state.players[player.get() as usize];
                        let old = p.life;
                        p.life -= i32::from(*n);
                        let new = p.life;
                        self.state.journal.record(GameEvent::LifeChanged {
                            player,
                            old,
                            new,
                            cause: Cause::Cost,
                        });
                    }
                    CostPart::ExileFromHand(_) => {
                        for card in &wizard.pitch {
                            let owner = self.state.object(*card).map_or(player, |o| o.owner);
                            self.state.move_object(
                                *card,
                                ZoneLocation::Exile(owner),
                                ZonePosition::Top,
                                Cause::Cost,
                            )?;
                        }
                    }
                    _ => {}
                }
            }
        }
        // Mandatory additional cost parts (e.g. Toxic Deluge's pay X life).
        for part in face.mandatory_additional_costs {
            match part {
                CostPart::PayLifeX => {
                    let p = &mut self.state.players[player.get() as usize];
                    let old = p.life;
                    p.life -= wizard.x as i32;
                    let new = p.life;
                    self.state.journal.record(GameEvent::LifeChanged {
                        player,
                        old,
                        new,
                        cause: Cause::Cost,
                    });
                }
                CostPart::PayLife(n) => {
                    let p = &mut self.state.players[player.get() as usize];
                    let old = p.life;
                    p.life -= i32::from(*n);
                    let new = p.life;
                    self.state.journal.record(GameEvent::LifeChanged {
                        player,
                        old,
                        new,
                        cause: Cause::Cost,
                    });
                }
                _ => {}
            }
        }
        // Move the card to the stack as a spell.
        let card = wizard.card;
        {
            let obj = self.state.object_mut(card).expect("wizard card exists");
            obj.kind = ObjectKind::Spell;
            obj.controller = player;
            obj.targets.clone_from(&wizard.targets);
            obj.x_value = wizard.x;
            obj.kicked = wizard.kicked;
            obj.alt_cast = matches!(wizard.option, Some(CastModeKind::Alternative(_)));
            obj.chosen_player = wizard.chosen_player;
            obj.cast_from_hand = !wizard.free;
            obj.mode_index = match wizard.option {
                Some(CastModeKind::Mode(i)) => Some(i.try_into().expect("mode index fits u8")),
                _ => None,
            };
        }
        // MDFC back-face cast: the object becomes its chosen face (CR 712.4).
        if let Some(CastModeKind::Face(i)) = wizard.option {
            let def = self
                .state
                .object(card)
                .and_then(|o| o.card)
                .and_then(|c| self.lookup.card(c.index))
                .expect("wizard card known");
            self.state.switch_face(card, def, i);
        }
        self.state
            .move_object(card, ZoneLocation::Stack, ZonePosition::Top, Cause::Spell)?;
        // Per-turn tracking for conditional triggers (Esper Sentinel).
        if !face.types.contains(baylee_core::types::TypeSet::CREATURE)
            && let Some(v) = self
                .state
                .per_turn
                .noncreature_spells
                .get_mut(player.get() as usize)
        {
            *v = v.saturating_add(1);
        }
        self.state.journal.record(GameEvent::SpellCast {
            object: card,
            player,
        });
        self.cast_wizard = None;
        self.after_action(player);
        Ok(())
    }
}

/// The mana cost for the wizard's chosen option, X applied.
fn wizard_cost(wizard: &CastWizard) -> ManaCost {
    let base = wizard
        .options
        .iter()
        .find(|o| Some(o.kind) == wizard.option)
        .map_or(ManaCost::ZERO, |o| o.cost);
    base.with_x(wizard.x)
}
