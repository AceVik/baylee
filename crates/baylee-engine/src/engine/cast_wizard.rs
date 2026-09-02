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
use crate::casting;
use crate::choice::{CastModeDesc, CastModeKind, ChoicePrompt, YesNoPrompt};
use crate::object::GameObject;
use baylee_cards_dsl::{AltCondition, CostPart, SpellMode, TargetReq, TargetSpec};
use baylee_core::ids::NameRef;
use baylee_core::mana::{ManaColor, ManaCost};

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
    /// Delve choice (exile-from-graveyard, {1} each).
    Delve,
    /// Convoke choice (tap creatures, {1} each).
    Convoke,
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
    /// Players chosen as targets, the other half of "any target".
    pub target_players: baylee_core::ids::SeatSet,
    /// Chosen target player, if any.
    pub chosen_player: Option<PlayerId>,
    /// Chosen X.
    pub x: u32,
    /// Whether the kicker was taken.
    pub kicked: bool,
    /// Cards chosen for pitch (exile-from-hand).
    pub pitch: SmallVec<[ObjectId; 2]>,
    /// Cards chosen to delve (exile-from-graveyard, {1} each).
    pub delve_exiles: SmallVec<[ObjectId; 8]>,
    /// Creatures chosen to tap for convoke ({1} each).
    pub convoke_taps: SmallVec<[ObjectId; 8]>,
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
            target_players: baylee_core::ids::SeatSet::new(),
            chosen_player: None,
            x: 0,
            kicked: false,
            pitch: SmallVec::new(),
            delve_exiles: SmallVec::new(),
            convoke_taps: SmallVec::new(),
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
            target_players: baylee_core::ids::SeatSet::new(),
            chosen_player: None,
            x: 0,
            kicked: false,
            pitch: SmallVec::new(),
            delve_exiles: SmallVec::new(),
            convoke_taps: SmallVec::new(),
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
            target_players: baylee_core::ids::SeatSet::new(),
            chosen_player: None,
            x: 0,
            kicked: false,
            pitch: SmallVec::new(),
            delve_exiles: SmallVec::new(),
            convoke_taps: SmallVec::new(),
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
        // Mycosynth Lattice: every probe below asks whether the pool covers a
        // cost, and under the Lattice any mana answers any pip.
        let afford = |cost: &baylee_core::mana::ManaCost| {
            casting::wild_or_not(casting::mana_is_wild(&self.state), pool, cost)
        };
        let mut options = Vec::new();
        // Disturb casts come from the graveyard: no normal-cost option.
        let disturb_cast = self
            .state
            .object(card)
            .is_some_and(|o| o.zone == crate::zone::Zone::Graveyard)
            && def.faces.iter().any(|f| f.disturb);
        if disturb_cast {
            for (i, back) in def.faces.iter().enumerate().skip(1) {
                if back.disturb && afford(&back.mana_cost.with_x(0)) {
                    options.push(CastModeDesc {
                        index: (options.len()) as u8,
                        kind: CastModeKind::Face(i),
                        cost: back.mana_cost,
                    });
                }
            }
            return Ok(options);
        }
        // Conditional cost reduction printed on the card (Surgical
        // Metamorph & co.).
        let normal_cost = match face.cost_reduction {
            Some(baylee_cards_dsl::CostReduction::NotStartingPlayer(n))
                if player != self.state.starting_player =>
            {
                face.mana_cost.with_less_generic(n)
            }
            _ => face.mana_cost,
        };
        // Normal cost (X probed with 0; the real check happens at payment).
        if afford(&normal_cost.with_x(0)) {
            options.push(CastModeDesc {
                index: 0,
                kind: CastModeKind::Normal,
                cost: normal_cost,
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
            if afford(&back.mana_cost.with_x(0)) {
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
                if afford(&cost.with_x(0)) {
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
        let result = self.advance_cast_wizard_inner();
        if result.is_err() {
            // A cast that fails mid-wizard (payment, late target legality)
            // fizzles cleanly: drop the wizard and resume the game instead
            // of leaving a consumed choice pending.
            self.cast_wizard = None;
            self.awaiting_answer = false;
            self.run_until_choice();
        }
        result
    }

    #[allow(clippy::too_many_lines)]
    fn advance_cast_wizard_inner(&mut self) -> Result<(), EngineError> {
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
                let player_options = eval::target_player_options(&self.state, &spec);
                // "Any target" is one set: a burn spell with every creature
                // hexproofed is still castable at a player's face, so the
                // count that has to reach `min` spans both halves.
                if options.len() + player_options.len() < min as usize {
                    self.cast_wizard = None;
                    return Err(EngineError::IllegalAction("not enough legal targets"));
                }
                self.pending = Pending::ChooseTargets {
                    player: wizard.player,
                    options,
                    player_options,
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
                    .filter(|p| {
                        !p.has_lost
                            // Player hexproof (Everybody Lives!): can't be
                            // targeted by spells/abilities.
                            && !self.state.effects.iter().any(|fx| {
                                matches!(
                                    fx.modifier,
                                    baylee_cards_dsl::Modifier::PlayerHexproof
                                ) && fx.controller == p.id
                            })
                    })
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
                    source: self.state.object(wizard.card).and_then(|o| {
                        o.card.map(|c| {
                            baylee_core::ids::AbilityRef::new(
                                c.index,
                                baylee_core::ids::AbilityRef::ADDITIONAL_COST,
                            )
                        })
                    }),
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
                wizard.stage = WizardStage::Delve;
                self.cast_wizard = Some(wizard);
                self.advance_cast_wizard()
            }
            WizardStage::Delve => {
                let face = self.wizard_face(&wizard);
                let graveyard: Vec<ObjectId> = self
                    .state
                    .zones
                    .list(ZoneLocation::Graveyard(wizard.player))
                    .clone();
                if !face.delve || graveyard.is_empty() {
                    let mut wizard = wizard;
                    wizard.stage = WizardStage::Convoke;
                    self.cast_wizard = Some(wizard);
                    return self.advance_cast_wizard();
                }
                self.pending = Pending::ChooseCards {
                    player: wizard.player,
                    options: graveyard,
                    min: 0,
                    max: 99,
                    prompt: ChoicePrompt::Generic,
                };
                self.awaiting_answer = true;
                Ok(())
            }
            WizardStage::Convoke => {
                let face = self.wizard_face(&wizard);
                let untapped: Vec<ObjectId> = self
                    .state
                    .zones
                    .list(ZoneLocation::Battlefield)
                    .iter()
                    .copied()
                    .filter(|id| {
                        self.state.object(*id).is_some_and(|o| {
                            o.controller == wizard.player
                                && (o
                                    .characteristics()
                                    .types
                                    .contains(baylee_core::types::TypeSet::CREATURE)
                                    || o.characteristics()
                                        .types
                                        .contains(baylee_core::types::TypeSet::ARTIFACT))
                                && !o.status.contains(crate::object::Status::TAPPED)
                        })
                    })
                    .collect();
                if !face.convoke || untapped.is_empty() {
                    let mut wizard = wizard;
                    wizard.stage = WizardStage::Done;
                    self.cast_wizard = Some(wizard);
                    return self.advance_cast_wizard();
                }
                self.pending = Pending::ChooseTargets {
                    player: wizard.player,
                    options: untapped,
                    player_options: Vec::new(),
                    min: 0,
                    max: 99,
                };
                self.awaiting_answer = true;
                Ok(())
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
        // Delve (CR 702.66): exile the chosen graveyard cards; each pays
        // for {1} of the generic part.
        for &card in &wizard.delve_exiles {
            let _ = self.state.move_object(
                card,
                ZoneLocation::Exile(player),
                ZonePosition::Top,
                Cause::Cost,
            )?;
        }
        // Convoke (CR 702.51): tap the chosen creatures; each pays for {1}.
        for &creature in &wizard.convoke_taps {
            if let Some(obj) = self.state.object_mut(creature) {
                obj.status.insert(crate::object::Status::TAPPED);
            }
        }
        let reduction = (wizard.delve_exiles.len() + wizard.convoke_taps.len()) as u32;
        if reduction > 0 {
            total = reduce_generic(&total, reduction);
        }
        if !wizard.free {
            // Restricted mana (Cavern of Souls & co.): matching entries
            // pay first, their riders apply; the rest comes from the pool.
            let (remaining, riders) = self.spend_restricted(player, wizard.card, total);
            // Mycosynth Lattice: spend mana as though it were any color.
            let wild = self
                .state
                .effects
                .iter()
                .any(|fx| matches!(fx.modifier, baylee_cards_dsl::Modifier::ManaIsAnyColor));
            let paid = if wild {
                mana_pay::pay_wild(
                    &mut self.state.players[player.get() as usize].mana_pool,
                    &remaining,
                )
            } else {
                mana_pay::pay(
                    &mut self.state.players[player.get() as usize].mana_pool,
                    &remaining,
                )
            };
            if !paid {
                // Refund restricted entries (cast cancelled).
                for (mana, _, _) in &riders {
                    self.state.players[player.get() as usize]
                        .mana_pool
                        .add_restricted(*mana);
                }
                self.cast_wizard = None;
                return Err(EngineError::IllegalAction("cannot pay the total cost"));
            }
            self.apply_spend_riders(player, wizard.card, &riders);
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
        // Move the card to the stack as a spell. The targeting requirement
        // rides along on the object: a copy of this spell may be retargeted
        // during resolution (CR 707.10c), and the resolver has no card lookup
        // of its own to re-derive it from.
        let target_req = self.wizard_target_req(wizard);
        let card = wizard.card;
        {
            let obj = self.state.object_mut(card).expect("wizard card exists");
            obj.kind = ObjectKind::Spell;
            obj.set_controller(player);
            obj.targets.clone_from(&wizard.targets);
            obj.target_players = wizard.target_players;
            obj.target_req = target_req;
            obj.x_value = wizard.x;
            obj.kicked = wizard.kicked;
            obj.alt_cast = matches!(wizard.option, Some(CastModeKind::Alternative(_)));
            obj.chosen_player = wizard.chosen_player;
            obj.cast_from_hand = !wizard.free;
            // Flashback (CR 702.34): a spell cast from the graveyard via a
            // grant is exiled instead of hitting the graveyard again.
            if obj.zone == crate::zone::Zone::Graveyard {
                obj.riders.push(crate::object::Rider::Flashback);
                obj.cast_from_hand = false;
            }
            obj.mode_index = match wizard.option {
                Some(CastModeKind::Mode(i)) => Some(i.try_into().expect("mode index fits u8")),
                _ => None,
            };
        }
        // Commander-cast tracking (Commander's Insight): casts from the
        // command zone count.
        if self
            .state
            .object(card)
            .is_some_and(|o| o.zone == crate::zone::Zone::Command)
            && let Some(v) = self.state.commander_casts.get_mut(player.get() as usize)
        {
            *v = v.saturating_add(1);
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
        if let Some(v) = self
            .state
            .per_turn
            .spells_cast
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
/// Removes up to `n` generic mana from a cost (delve/convoke payments).
fn reduce_generic(cost: &ManaCost, n: u32) -> ManaCost {
    cost.with_less_generic(n)
}

/// Reduces a cost by one mana of the given color (colored first, then
/// generic).
impl<L: CardLookup> Engine<L> {
    /// Spends restricted pool entries whose filter matches the spell;
    /// returns the reduced cost and the spent `(mana, source, rider)`.
    #[allow(clippy::type_complexity)]
    fn spend_restricted(
        &mut self,
        player: PlayerId,
        spell: ObjectId,
        cost: ManaCost,
    ) -> (
        ManaCost,
        Vec<(
            baylee_core::mana::RestrictedMana,
            ObjectId,
            baylee_cards_dsl::SpendRider,
        )>,
    ) {
        let mut remaining = cost;
        let mut spent = Vec::new();
        let pool = &mut self.state.players[player.get() as usize].mana_pool;
        let entries: Vec<baylee_core::mana::RestrictedMana> = pool.restricted().to_vec();
        for mana in entries {
            let id = mana.restriction.0;
            let Some(&(source, filter, rider)) = self.state.restriction_info.get(&id) else {
                continue;
            };
            let Some(spell_obj) = self.state.object(spell) else {
                continue;
            };
            if !crate::eval::matches(filter, &self.state, spell_obj, player, source) {
                continue;
            }
            // Consume the entry and reduce the cost by its mana.
            let taken = self.state.players[player.get() as usize]
                .mana_pool
                .take_restricted(id);
            let Some(mana) = taken else { continue };
            for _ in 0..mana.amount {
                remaining = reduce_one(&remaining, mana.color);
            }
            spent.push((mana, source, rider));
        }
        (remaining, spent)
    }

    /// Applies spend riders after a restricted-mana payment (uncounterable
    /// marks, scry triggers).
    fn apply_spend_riders(
        &mut self,
        player: PlayerId,
        spell: ObjectId,
        riders: &[(
            baylee_core::mana::RestrictedMana,
            ObjectId,
            baylee_cards_dsl::SpendRider,
        )],
    ) {
        for (_, _, rider) in riders {
            match rider {
                baylee_cards_dsl::SpendRider::None => {}
                baylee_cards_dsl::SpendRider::Uncounterable => {
                    if let Some(obj) = self.state.object_mut(spell) {
                        obj.riders.push(crate::object::Rider::Uncounterable);
                    }
                }
                baylee_cards_dsl::SpendRider::Scry(n) => {
                    let fx: &'static [baylee_cards_dsl::Effect] =
                        if *n >= 2 { &SCRY_TWO } else { &SCRY_ONE };
                    let card = self
                        .state
                        .object(spell)
                        .and_then(|o| o.card)
                        .map(|c| c.index);
                    if let Some(card) = card {
                        let name = self
                            .state
                            .object(spell)
                            .map_or(NameRef::new(0), |o| o.base.name);
                        let base = self.state.bare_base(name);
                        let id = self.state.arena.insert_with(|id| {
                            GameObject::new_ability_on_stack(
                                id,
                                player,
                                crate::object::AbilityLoc {
                                    card,
                                    index: u32::MAX,
                                    source: spell,
                                },
                                SmallVec::new(),
                                base,
                            )
                        });
                        self.synthetic_fx.insert(id, fx);
                        self.state
                            .zones
                            .insert(id, ZoneLocation::Stack, ZonePosition::Top, false);
                    }
                }
            }
        }
    }
}

/// Reduces a cost by one mana of the given color (colored first, then
/// generic).
fn reduce_one(cost: &ManaCost, color: ManaColor) -> ManaCost {
    let colored = match color {
        ManaColor::White => Some(baylee_core::mana::ManaSymbol::White),
        ManaColor::Blue => Some(baylee_core::mana::ManaSymbol::Blue),
        ManaColor::Black => Some(baylee_core::mana::ManaSymbol::Black),
        ManaColor::Red => Some(baylee_core::mana::ManaSymbol::Red),
        ManaColor::Green => Some(baylee_core::mana::ManaSymbol::Green),
        ManaColor::Colorless => Some(baylee_core::mana::ManaSymbol::Colorless),
    };
    let mut out = ManaCost::ZERO;
    let mut consumed = false;
    for s in cost.symbols() {
        if !consumed && Some(s) == colored {
            consumed = true;
            continue;
        }
        if !consumed && let baylee_core::mana::ManaSymbol::Generic(amount) = s {
            consumed = true;
            if amount > 1 {
                out = out.combine(&ManaCost::from_symbol_generic(amount - 1));
            }
            continue;
        }
        out = out.combine(&ManaCost::from_symbol(s));
    }
    out
}

static SCRY_ONE: [baylee_cards_dsl::Effect; 1] = [baylee_cards_dsl::Effect::Scry {
    amount: baylee_cards_dsl::Amount::Fixed(1),
}];
static SCRY_TWO: [baylee_cards_dsl::Effect; 1] = [baylee_cards_dsl::Effect::Scry {
    amount: baylee_cards_dsl::Amount::Fixed(2),
}];

fn wizard_cost(wizard: &CastWizard) -> ManaCost {
    let base = wizard
        .options
        .iter()
        .find(|o| Some(o.kind) == wizard.option)
        .map_or(ManaCost::ZERO, |o| o.cost);
    base.with_x(wizard.x)
}
