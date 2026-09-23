//! The casting wizard: multi-step spell casting (CR 601.2a–h).
//!
//! Modes/alternative costs → X → targets → kicker → pitch choices →
//! payment. Each step suspends into a pending request; the wizard resumes
//! on the answer. Everything ends in one `SpellCast` event — atomic from
//! the outside.

use super::{
    AbilityDef, CardLookup, Cause, Engine, EngineError, GameEvent, ObjectId, ObjectKind, Pending,
    PlayerId, SmallVec, ZoneLocation, ZonePosition, eval, mana_pay,
};
use crate::casting;
use crate::choice::{CastModeDesc, CastModeKind, ChoicePrompt, TargetPrompt, YesNoPrompt};
use crate::object::GameObject;
use baylee_cards_dsl::{AltCondition, CostPart, SpellMode, TargetReq, TargetSpec};
use baylee_core::ids::NameRef;
use baylee_core::mana::{ManaColor, ManaCost};

/// The largest X the wizard will offer, before anything narrows it.
///
/// A ceiling rather than a rule: X has no printed bound and the mana for a
/// printed `{X}` is validated when the wizard finishes, so this only keeps the
/// question finite for a client that has to draw it. A cost that *can* be
/// bounded — a life payment, which CR 119.4 caps at the caster's own total —
/// narrows it in the stage below.
pub(crate) const X_CEILING: u32 = 50;

/// Where the wizard currently is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum WizardStage {
    /// Choosing a cast mode (skip when only one option).
    ChooseMode,
    /// Choosing X.
    XValue,
    /// Choosing targets.
    Targets,
    /// Choosing the targets of a second instance of the word "target"
    /// (CR 601.2c chooses each instance's; CR 115.3 lets one object be
    /// chosen once for each). Skipped by any spell that says the word once.
    SecondTargets,
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
    /// Targets chosen for the second instance of the word, kept apart for
    /// the reason `GameObject::second_targets` gives.
    pub second_targets: SmallVec<[ObjectId; 1]>,
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

/// The cost parts [`Engine::finish_cast`] pays out of a chosen **alternative**
/// cost.
///
/// A spell has three cost lists and no two of them are paid by the same code,
/// so "the wizard pays it" is a claim that has to name which list. Everything
/// the option scan lets through and this predicate does not name is paid by
/// nobody — including a sacrifice or a discard, which an *activation* can now
/// ask about (`engine::cost_wizard`) and a spell's alternative cost still
/// cannot. That is the reason the pool-wide guard below is the gate here and
/// `can_afford` is not: `can_afford` used to refuse those two parts on every
/// board and no longer does, so what stops a spell being cast for free is a
/// build failure and not a board question.
///
/// It is a *dead offer* only when the printed cost is unaffordable, which is
/// the one case `casting.rs` reads this list at all: its `any_alt` probe sits
/// inside `if !probe(printed)`, so a card whose mana cost is payable is
/// castable on its own and a refused alternative costs it one mode. A card
/// that had nothing else is put in `legal.castable` by that probe and then
/// finds the wizard with no option to give it — the shape the
/// `AltCondition::CommanderControlled` bug had, and the shape a Force of Will
/// with nothing blue to pitch had until that probe learned to ask about the
/// cost's parts as well as its mana.
///
/// Held pool-wide by
/// `offer_tests::no_spell_cost_list_carries_a_part_its_payment_walks_past`.
pub(crate) const fn paid_as_an_alternative_cost(part: &CostPart) -> bool {
    matches!(part, CostPart::PayLife(_) | CostPart::ExileFromHand(_))
}

/// The cost parts [`Engine::finish_cast`] pays out of
/// `FaceDef.mandatory_additional_costs`.
///
/// The list with no gate at all: nothing runs `can_afford` over it, so unlike
/// the alternative-cost list above it does not even refuse the choice costs —
/// a `Sacrifice(_)` written here would be cast past rather than declined.
/// Toxic Deluge's `PayLifeX` is the whole of the pool's use of it.
///
/// Which is why the one bound this list has is on the *question* instead: the
/// `XValue` stage caps X at the caster's life total (CR 119.4), because
/// `finish_cast` below subtracts what it is given and reads no total. A
/// `PayLife(n)` written here has no such stage and would still be paid past
/// zero — it belongs beside the X the day a card prints one.
pub(crate) const fn paid_as_a_mandatory_additional_cost(part: &CostPart) -> bool {
    matches!(part, CostPart::PayLifeX | CostPart::PayLife(_))
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
            second_targets: SmallVec::new(),
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
            second_targets: SmallVec::new(),
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
        if self
            .state
            .object(card)
            .filter(|o| o.zone == crate::zone::Zone::Exile)
            .and_then(|o| o.card)
            .and_then(|c| self.lookup.card(c.index))
            .is_none()
        {
            return Err(EngineError::IllegalAction("no card to cast from exile"));
        }
        let mut wizard = CastWizard {
            card,
            player,
            option: Some(CastModeKind::Normal),
            targets: SmallVec::new(),
            second_targets: SmallVec::new(),
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
    #[allow(clippy::too_many_lines)] // one branch per printed way to cast; splitting hides the list
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
        // Restricted mana this spell may be paid with counts here for the
        // same reason the convoke count below does: this probe and
        // `casting::can_cast`'s have to be the same probe, or the Cavern's
        // Ally is offered in `LegalActions` and refused the moment it is
        // taken.
        let with_restricted = casting::spendable_pool(&self.state, player, card);
        let pool = with_restricted
            .as_ref()
            .unwrap_or(&self.state.players[player.get() as usize].mana_pool);
        // Commander tax (CR 903.8): {2} more generic for each previous cast
        // of this commander from the command zone. It is a cost increase, so
        // it lands on every way of casting the card, an alternative cost
        // included (CR 601.2f) — and on the affordability probes too, or a
        // mode would be offered that the player then cannot pay for.
        let tax = casting::commander_tax(&self.state, player, card);
        // Convoke (CR 702.51) pays {1} per untapped creature or artifact and
        // delve (CR 702.66a) pays {1} per card exiled from the graveyard, so
        // both are part of what "afford" means. It has to be the same count
        // `casting::can_cast` uses, or the spell is offered in
        // `LegalActions` and then refused here as "no way to cast this
        // spell" — which is what happened, and it is why the count lives in
        // one function rather than in each of them.
        let reduction = casting::keyword_reduction(&self.state, face, player);
        // Mycosynth Lattice: every probe below asks whether the pool covers a
        // cost, and under the Lattice any mana answers any pip.
        let afford = |cost: &baylee_core::mana::ManaCost| {
            casting::wild_or_not(
                casting::mana_is_wild(&self.state),
                pool,
                &cost.with_more_generic(tax).with_less_generic(reduction),
            )
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
                if back.disturb
                    && afford(&back.mana_cost.with_x(0))
                    && casting::face_has_a_legal_target(&self.state, &self.lookup, player, card, i)
                {
                    options.push(CastModeDesc {
                        index: (options.len()) as u8,
                        kind: CastModeKind::Face(i),
                        cost: back.mana_cost.with_more_generic(tax),
                    });
                }
            }
            return Ok(options);
        }
        // Conditional cost reduction printed on the card (Surgical
        // Metamorph & co.), through the same reader `can_cast` uses — this
        // was the second place the two probes disagreed, and in the other
        // direction from convoke: the wizard knew the discount and the offer
        // did not, so the seat entitled to it was never shown the card.
        let normal_cost =
            face.mana_cost
                .with_less_generic(casting::printed_reduction(&self.state, face, player));
        // A modal spell (CR 700.2) has no "no mode" way to be cast: every one
        // of its effects sits under a mode, so a `Normal` option resolves to
        // nothing at all. `progress` looks for an `AbilityDef::Spell` first
        // and falls back to the object's `mode_index`, and a card whose only
        // spell ability is `ModalSpell` cast as `Normal` has neither — it went
        // hand → stack → graveyard and did nothing, which is how Damn was
        // found. The four cards this reaches (damn, cyclonic_rift,
        // heliod_s_intervention, sheoldred_s_edict) stay castable: a mode's
        // cost defaults to the face's, so `Mode(0)` carries exactly the cost
        // `Normal` was offering. `casting::face_has_a_legal_target` already
        // asks this same question of `abilities_for_face`, in the same
        // direction and for the same reason. The two clauses beside it hold
        // the guard to exactly "casting with no mode chosen does nothing": a
        // permanent spell arrives on the battlefield whether a mode was
        // picked or not, and a plain `Spell` printed beside the modes is what
        // `progress` finds first.
        let abilities = def.abilities_for_face(0);
        let modal_only = abilities
            .iter()
            .any(|a| matches!(a, baylee_cards_dsl::AbilityDef::ModalSpell { .. }))
            && !abilities
                .iter()
                .any(|a| matches!(a, baylee_cards_dsl::AbilityDef::Spell { .. }))
            && !face.types.is_permanent();
        // Normal cost (X probed with 0; the real check happens at payment).
        // Guarded by the same CR 202.1b question `can_cast` asks, and for the
        // reason every probe in this function is paired with one there: an
        // option offered here that the offer does not know about is a mode a
        // player can pick and be refused for.
        if !modal_only
            && casting::has_a_printed_cost(&face.mana_cost)
            && afford(&normal_cost.with_x(0))
        {
            options.push(CastModeDesc {
                index: 0,
                kind: CastModeKind::Normal,
                cost: normal_cost.with_more_generic(tax),
            });
        }
        // Alternative costs (pitch, evoke, conditional free).
        for (i, alt) in face.alternative_costs.iter().enumerate() {
            let condition_ok = match alt.condition {
                AltCondition::Always => true,
                AltCondition::NotYourTurn => self.state.turn.active != player,
                AltCondition::CommanderControlled => self.has_commander_on_battlefield(player),
            };
            let taxed = baylee_cards_dsl::Cost {
                mana: alt.cost.mana.with_more_generic(tax),
                ..alt.cost
            };
            if !condition_ok || !self.can_afford(player, card, &taxed) {
                continue;
            }
            options.push(CastModeDesc {
                index: (options.len()) as u8,
                kind: CastModeKind::Alternative(i),
                cost: alt.cost.mana.with_more_generic(tax),
            });
        }
        // MDFC backs (CR 712.11b) and adventures (CR 715), through the reader
        // `can_cast` uses — land faces are played and disturb backs came out
        // above. This loop asked the faces and nothing else, so the adventure
        // was offered again out of the exile its own resolution had put the
        // card in, and at instant speed forever: the offer read the exiled
        // object's *current* face, which was still the instant, so Swift
        // Spiral was `{1}{W}` in `LegalActions` every turn and the wizard
        // priced and cast it. CR 715.3d is the rule it broke.
        let on_adventure = self.state.object(card).is_some_and(|o| {
            o.zone == crate::zone::Zone::Exile
                && o.riders.contains(&crate::object::Rider::Adventure)
        });
        for (i, back) in casting::castable_back_faces(def, on_adventure) {
            if afford(&back.mana_cost.with_x(0))
                && casting::face_has_a_legal_target(&self.state, &self.lookup, player, card, i)
            {
                options.push(CastModeDesc {
                    index: (options.len()) as u8,
                    kind: CastModeKind::Face(i),
                    cost: back.mana_cost.with_more_generic(tax),
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
                // Affordable *and* pointable. Every other option in this
                // function is paired with the probe `can_cast` makes, and a
                // mode was the one that was not: Cyclonic Rift on an empty
                // board offered "return target nonland permanent you don't
                // control" and answered the press with "not enough legal
                // targets", leaving the question standing so an agent pressed
                // it again on the next pass.
                if afford(&cost.with_x(0))
                    && casting::mode_has_a_legal_target(&self.state, &self.lookup, player, card, i)
                {
                    options.push(CastModeDesc {
                        index: (options.len()) as u8,
                        kind: CastModeKind::Mode(i),
                        cost: cost.with_more_generic(tax),
                    });
                }
            }
        }
        if options.is_empty() {
            return Err(EngineError::IllegalAction("no way to cast this spell"));
        }
        Ok(options)
    }

    /// The condition Fierce Guardianship and Flawless Maneuver print — "if
    /// you control a commander", which is card text and not a rule of the
    /// format: does this player control their own commander right now?
    ///
    /// This read the command *zone* until now, and so was always false: a
    /// commander on the battlefield is no longer listed in the zone it left,
    /// so the one place the answer could be yes is the one place the lookup
    /// could not reach. It now defers to `casting`, which is where the
    /// legality probe asks the same question.
    fn has_commander_on_battlefield(&self, player: PlayerId) -> bool {
        casting::controls_a_commander(&self.state, player)
    }

    /// Drives the wizard forward until it needs an answer or finishes.
    #[allow(clippy::too_many_lines)] // the wizard is a flat stage machine; extraction would obscure it
    pub(crate) fn advance_cast_wizard(&mut self) -> Result<(), EngineError> {
        // Read before the stages run: `finish_cast` nulls the wizard itself
        // on the way out, so by the error branch there is nobody left to ask.
        let caster = self.cast_wizard.as_ref().map(|w| w.player);
        let result = self.advance_cast_wizard_inner();
        if result.is_err() {
            // A cast that fails mid-wizard (payment, late target legality)
            // fizzles cleanly: drop the wizard and resume the game instead
            // of leaving a consumed choice pending.
            self.cast_wizard = None;
            self.awaiting_answer = false;
            // CR 601.2h reverses the *whole* casting, so the game returns to
            // the moment before it began — and that includes whose priority
            // it was. Nothing in the wizard path touches `passes` or
            // `priority_holder`, so re-asking the caster is the exact
            // restore. Resuming through `run_until_choice` instead walked
            // the priority round on to the next seat, because to
            // `priority_round` a holder who is no longer being asked has
            // taken their turn: a player whose waterbend could not be paid
            // was told "cannot pay the total cost" and then lost the rest of
            // their own main phase to a spell that never happened.
            if let Some(player) = caster
                && self.priority_holder == Some(player)
            {
                self.pending = Pending::Priority {
                    player,
                    legal: Box::new(self.compute_legal(player)),
                };
                self.awaiting_answer = true;
            } else {
                // A cast that did not start from a priority round (cascade,
                // a miracle offer) has no such moment to return to.
                self.run_until_choice();
            }
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
                    object: wizard.card,
                    options: wizard.options.clone(),
                };
                self.awaiting_answer = true;
                Ok(())
            }
            WizardStage::XValue => {
                // The chosen option's *printed* cost, and deliberately not
                // `wizard_cost`: that one substitutes the chosen X into the
                // cost, and the chosen X at this stage is still the zero the
                // wizard starts with. So `{X}{U}{U}{U}` arrived here as
                // `{0}{U}{U}{U}` — no variable left in it, `needs_x` false,
                // and every spell with an X in its printed cost was quietly
                // cast for X = 0 without anybody being asked. Nothing failed;
                // the question simply never appeared.
                let cost = chosen_option_cost(&wizard);
                // X is asked when the cost has a variable OR a mandatory
                // part scales with it (Toxic Deluge's pay-X-life).
                let pays_life_x = self
                    .wizard_face(&wizard)
                    .mandatory_additional_costs
                    .contains(&CostPart::PayLifeX);
                let needs_x = cost.has_variable() || pays_life_x;
                if needs_x {
                    // A printed `{X}` is bounded by nothing here on purpose:
                    // the mana is validated when the wizard finishes. Life is
                    // not, and cannot be — `mandatory_additional_costs` is the
                    // one cost list no `can_afford` reads (see
                    // [`paid_as_a_mandatory_additional_cost`]) and
                    // `finish_cast` subtracts what comes back without looking
                    // at the total. So CR 119.4 is enforced on the *question*,
                    // which is where this engine puts every other legality:
                    // Toxic Deluge for X = 25 at twenty life used to be
                    // accepted, take the caster to -5, and lose them the game
                    // to a state-based action on the way to resolving.
                    let mut max = X_CEILING;
                    if pays_life_x {
                        let life = self.state.players[wizard.player.get() as usize].life;
                        max = max.min(u32::try_from(life).unwrap_or(0));
                    }
                    self.pending = Pending::ChooseNumber {
                        player: wizard.player,
                        min: 0,
                        max,
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
                    wizard.stage = WizardStage::SecondTargets;
                    self.cast_wizard = Some(wizard);
                    return self.advance_cast_wizard();
                };
                let (mut min, mut max) = (req.min, req.max);
                if req.count_is_x {
                    min = wizard.x as u8;
                    max = wizard.x as u8;
                }
                let spec = req.spec;
                if matches!(spec, TargetSpec::AnyPlayer | TargetSpec::AnyOpponent) {
                    let mut wizard = wizard;
                    wizard.stage = WizardStage::ChoosePlayer;
                    self.cast_wizard = Some(wizard);
                    return self.advance_cast_wizard();
                }
                if max == 0 {
                    // Zero required targets (e.g. X = 0): skip targeting.
                    // This reads `max` off the requirement, before the
                    // options are known; the other way of having nothing to
                    // choose is a `max` the board cannot fill, and that one
                    // is decided below, once they are.
                    let mut wizard = wizard;
                    wizard.stage = WizardStage::SecondTargets;
                    self.cast_wizard = Some(wizard);
                    return self.advance_cast_wizard();
                }
                let options = eval::target_options(&spec, &self.state, wizard.player, wizard.card);
                let player_options = eval::target_player_options(&self.state, &spec, wizard.player);
                // "Any target" is one set: a burn spell with every creature
                // hexproofed is still castable at a player's face, so the
                // count that has to reach `min` spans both halves.
                if options.len() + player_options.len() < min as usize {
                    self.cast_wizard = None;
                    return Err(EngineError::IllegalAction("not enough legal targets"));
                }
                if options.is_empty() && player_options.is_empty() {
                    // Nothing to point at, and `min` is zero or the check
                    // above would have refused the cast: the only answer is
                    // the empty list, so the spell goes on the stack with no
                    // targets instead of the caster being stopped for a
                    // choice they cannot make.
                    //
                    // Eerie Interlude and Clever Concealment are what reach
                    // it today — "any number of target permanents you
                    // control" is `min` 0, `max` 255, so the branch above
                    // never sees them, and either one is castable on an
                    // empty board. `collect_triggers` is the same rule on
                    // the trigger side.
                    let mut wizard = wizard;
                    wizard.stage = WizardStage::SecondTargets;
                    self.cast_wizard = Some(wizard);
                    return self.advance_cast_wizard();
                }
                self.pending = Pending::ChooseTargets {
                    player: wizard.player,
                    options,
                    player_options,
                    min,
                    max,
                    reason: TargetPrompt::Targets,
                };
                self.awaiting_answer = true;
                Ok(())
            }
            WizardStage::SecondTargets => {
                // The second instance is asked the way the first is, and is
                // its own question: its own requirement, its own count, and
                // options that do **not** leave out what the first chose —
                // CR 115.3 lets one object be the target of both instances
                // as long as it fits both. The two spells that reach this
                // today name disjoint sets ("you control" / "you don't"), so
                // the sentence costs nothing there and is right for the card
                // that doesn't.
                let Some(req) = self.wizard_second_target_req(&wizard) else {
                    let mut wizard = wizard;
                    wizard.stage = WizardStage::Kicker;
                    self.cast_wizard = Some(wizard);
                    return self.advance_cast_wizard();
                };
                let options =
                    eval::target_options(&req.spec, &self.state, wizard.player, wizard.card);
                if options.len() < req.min as usize {
                    self.cast_wizard = None;
                    return Err(EngineError::IllegalAction("not enough legal targets"));
                }
                if options.is_empty() || req.max == 0 {
                    // "Up to one" with nothing to point at: the empty answer
                    // is the only one, for the reason the first stage gives.
                    let mut wizard = wizard;
                    wizard.stage = WizardStage::Kicker;
                    self.cast_wizard = Some(wizard);
                    return self.advance_cast_wizard();
                }
                self.pending = Pending::ChooseTargets {
                    player: wizard.player,
                    options,
                    player_options: Vec::new(),
                    min: req.min,
                    max: req.max,
                    reason: TargetPrompt::Targets,
                };
                self.awaiting_answer = true;
                Ok(())
            }
            WizardStage::ChoosePlayer => {
                // The same enumeration the object half of targeting uses, and
                // for the same reason: "target opponent" is a choice over a
                // smaller set, not a different kind of choice (CR 115.1), so
                // the caster is out of it — and so is a teammate. Listing
                // every living player here was a hole even before teams: a
                // "target opponent" spell offered its own caster.
                let spec = self
                    .wizard_target_req(&wizard)
                    .map_or(TargetSpec::AnyPlayer, |req| req.spec);
                let options = eval::target_player_options(&self.state, &spec, wizard.player);
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
                    // The same scan `can_afford` and `casting::can_cast` now
                    // run before this cast was ever offered: one reader, so
                    // the offer and the prompt cannot disagree about what is
                    // in the hand.
                    let options =
                        casting::pitchable(&self.state, wizard.player, wizard.card, filter);
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
                // How many may be exiled is bounded by the *cost*, not by the
                // graveyard. CR 702.66a is "for each generic mana in this
                // spell's total cost, you may exile a card from your
                // graveyard rather than pay that mana", so a seventh card has
                // nothing left to pay for once `{6}{U}{U}` has had six exiled
                // against it. This asked for up to the whole graveyard, and
                // `reduce_generic` cuts *up to* n — so every card past the
                // sixth was exiled for no reduction at all, which is a cost
                // spent for nothing and not merely an odd-looking prompt.
                // Same shape as the X question: a bound that cannot be
                // enforced where the answer is spent goes on the question.
                //
                // The bound is on `max` alone. *Which* cards go is the
                // player's, so the whole graveyard stays in `options` — a
                // truncated list would pick their six for them.
                let generic = usize::try_from(wizard_total_cost(face, &wizard).generic_total())
                    .unwrap_or(usize::MAX);
                let room = graveyard.len().min(generic);
                if !face.delve || room == 0 {
                    let mut wizard = wizard;
                    wizard.stage = WizardStage::Convoke;
                    self.cast_wizard = Some(wizard);
                    return self.advance_cast_wizard();
                }
                let max = u8::try_from(room).unwrap_or(u8::MAX);
                self.pending = Pending::ChooseCards {
                    player: wizard.player,
                    options: graveyard,
                    min: 0,
                    max,
                    prompt: ChoicePrompt::Delve,
                };
                self.awaiting_answer = true;
                Ok(())
            }
            WizardStage::Convoke => {
                let face = self.wizard_face(&wizard);
                let untapped = crate::casting::convoke_sources(&self.state, wizard.player);
                if !face.convoke || untapped.is_empty() {
                    let mut wizard = wizard;
                    wizard.stage = WizardStage::Done;
                    self.cast_wizard = Some(wizard);
                    return self.advance_cast_wizard();
                }
                // The bound is what is actually on the table, not a sentinel.
                // `99` is what a player casting a waterbend spell was shown:
                // "choose up to 99 targets", over two creatures, for a
                // question that is not targeting at all.
                let max = u8::try_from(untapped.len()).unwrap_or(u8::MAX);
                self.pending = Pending::ChooseTargets {
                    player: wizard.player,
                    options: untapped,
                    player_options: Vec::new(),
                    min: 0,
                    max,
                    reason: TargetPrompt::Convoke,
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

    pub(super) fn wizard_target_req(&self, wizard: &CastWizard) -> Option<TargetReq> {
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
                AbilityDef::ModalSpell { modes } => {
                    modes.get(i).and_then(|m: &SpellMode| m.targets)
                }
                _ => None,
            }),
            _ => abilities.iter().find_map(|a| match a {
                AbilityDef::Spell { targets, .. } => *targets,
                _ => None,
            }),
        }
    }

    /// The spell's requirement for a second instance of the word "target",
    /// if it prints one.
    ///
    /// Only [`AbilityDef::Spell`] can: a mode of a modal spell carries one
    /// requirement, so a charm whose mode says "target" twice (Archdruid's
    /// Charm) is not expressible yet and answers `None` here by construction.
    pub(super) fn wizard_second_target_req(&self, wizard: &CastWizard) -> Option<TargetReq> {
        if matches!(wizard.option, Some(CastModeKind::Mode(_))) {
            return None;
        }
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
        def.abilities_for_face(face_index)
            .iter()
            .find_map(|a| match a {
                AbilityDef::Spell { second_targets, .. } => *second_targets,
                _ => None,
            })
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
        let mut total = wizard_total_cost(face, wizard);
        let player = wizard.player;
        // Delve (CR 702.66) and convoke (CR 702.51) each pay for {1} of the
        // generic part, so the count comes off the cost before anything is
        // paid — but the exiling and the tapping happen *after* the mana
        // does, below. They used to happen here, and a cast that then could
        // not pay the rest returned an error with the graveyard already
        // exiled and the creatures already tapped: the spell went back to
        // hand and the costs stayed spent. CR 601.2h reverses the whole
        // casting when a cost cannot be paid, and there is no half of it to
        // keep.
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
        // The mana is paid, so the rest of the cost may now be spent.
        for &card in &wizard.delve_exiles {
            let _ = self.state.move_object(
                card,
                ZoneLocation::Exile(player),
                ZonePosition::Top,
                Cause::Cost,
            )?;
        }
        for &creature in &wizard.convoke_taps {
            self.state.set_tapped(creature, true);
        }
        // Non-mana parts of the chosen alternative cost (pay life etc.).
        if let Some(CastModeKind::Alternative(i)) = wizard.option {
            let alt = &face.alternative_costs[i];
            for part in alt.cost.parts {
                if !paid_as_an_alternative_cost(part) {
                    continue;
                }
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
                    // Already skipped, by [`paid_as_an_alternative_cost`]
                    // above. Named rather than swept into a `_` so that a new
                    // `CostPart` is still a compile error in this match: the
                    // question "and is the new one paid here?" has to be asked
                    // once per list, and a wildcard answers it with silence.
                    CostPart::TapSelf
                    | CostPart::UntapSelf
                    | CostPart::SacrificeSelf
                    | CostPart::Sacrifice(_)
                    | CostPart::Discard(_)
                    | CostPart::DiscardSelf
                    | CostPart::ExileSelf
                    | CostPart::ReturnSelfToHand
                    | CostPart::TapOther(_)
                    | CostPart::ReturnToHand(_)
                    | CostPart::RemoveCounterSelf { .. }
                    | CostPart::RemoveCounterSelfX { .. }
                    | CostPart::PutCounterSelf { .. }
                    | CostPart::PayLifeX => {}
                }
            }
        }
        // Mandatory additional cost parts (e.g. Toxic Deluge's pay X life).
        for part in face.mandatory_additional_costs {
            if !paid_as_a_mandatory_additional_cost(part) {
                continue;
            }
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
                // Already skipped, by [`paid_as_a_mandatory_additional_cost`]
                // above, and named for the reason the alternative-cost match
                // names its own — with the sharper edge that no `can_afford`
                // guards this list, so the three asking costs are skipped
                // here rather than refused.
                CostPart::TapSelf
                | CostPart::UntapSelf
                | CostPart::SacrificeSelf
                | CostPart::Sacrifice(_)
                | CostPart::Discard(_)
                | CostPart::TapOther(_)
                | CostPart::ReturnToHand(_)
                | CostPart::DiscardSelf
                | CostPart::ExileSelf
                | CostPart::ReturnSelfToHand
                | CostPart::RemoveCounterSelf { .. }
                | CostPart::RemoveCounterSelfX { .. }
                | CostPart::PutCounterSelf { .. }
                | CostPart::ExileFromHand(_) => {}
            }
        }
        // Move the card to the stack as a spell. The targeting requirement
        // rides along on the object: a copy of this spell may be retargeted
        // during resolution (CR 707.10c), and the resolver has no card lookup
        // of its own to re-derive it from.
        let target_req = self.wizard_target_req(wizard);
        let second_target_req = self.wizard_second_target_req(wizard);
        let card = wizard.card;
        {
            let obj = self.state.object_mut(card).expect("wizard card exists");
            obj.kind = ObjectKind::Spell;
            obj.set_controller(player);
            obj.targets.clone_from(&wizard.targets);
            obj.target_players = wizard.target_players;
            obj.target_req = target_req;
            obj.set_second(wizard.second_targets.clone(), second_target_req);
            obj.x_value = wizard.x;
            obj.kicked = wizard.kicked;
            obj.alt_cast = matches!(wizard.option, Some(CastModeKind::Alternative(_)));
            obj.chosen_player = wizard.chosen_player;
            obj.cast_from_hand = !wizard.free;
            // Flashback (CR 702.34): a spell cast from the graveyard via a
            // grant is exiled instead of hitting the graveyard again.
            if obj.zone == crate::zone::Zone::Graveyard {
                // Permanent permissions (for example Emry) are not flashback.
                if obj.characteristics().types.intersects(
                    baylee_core::types::TypeSet::INSTANT
                        .union(baylee_core::types::TypeSet::SORCERY),
                ) {
                    obj.riders.push(crate::object::Rider::Flashback);
                }
                obj.cast_from_hand = false;
            }
            obj.mode_index = match wizard.option {
                Some(CastModeKind::Mode(i)) => Some(i.try_into().expect("mode index fits u8")),
                _ => None,
            };
        }
        // Commander-cast tracking: casts from the command zone count, once
        // for the seat (Commander's Insight) and once for the commander
        // itself (CR 903.8's tax). Both here, so the two cannot drift.
        if self
            .state
            .object(card)
            .is_some_and(|o| o.zone == crate::zone::Zone::Command)
        {
            if let Some(v) = self.state.commander_casts.get_mut(player.get() as usize) {
                *v = v.saturating_add(1);
            }
            if let Some(c) = self
                .state
                .commanders
                .get_mut(player.get() as usize)
                .and_then(|cs| cs.iter_mut().find(|c| c.object == card))
            {
                c.casts = c.casts.saturating_add(1);
            }
        }
        // MDFC back-face cast: the object becomes its chosen face (CR 712.11b).
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
                    // A copied spell has no card, and the rider it was cast
                    // with is still the rider it was cast with. The guard
                    // that used to stand here dropped the scry rather than
                    // write a handle it had nothing to put in.
                    let card = self
                        .state
                        .object(spell)
                        .and_then(|o| o.card)
                        .map(|c| c.index);
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
                                index: baylee_core::ids::AbilityRef::SYNTHETIC,
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
    chosen_option_cost(wizard).with_x(wizard.x)
}

/// The whole mana cost this cast is about to pay: the chosen option with X
/// filled in, plus the kicker's own mana when the kicker was taken.
///
/// Two callers, and the second is the reason it is a function.
/// [`Engine::finish_cast`] needs the sum to pay it; the delve stage needs it
/// to *bound its question*, because CR 702.66a is worded "for each generic
/// mana in this spell's total cost, you may exile a card from your graveyard
/// rather than pay that mana" — total, so a kicker that has already been
/// taken is part of what delve may pay for, and X likewise (`with_x` turns
/// the variable into generic mana before this is read).
fn wizard_total_cost(face: &baylee_cards_dsl::FaceDef, wizard: &CastWizard) -> ManaCost {
    let mut total = wizard_cost(wizard);
    if wizard.kicked {
        for add in face.additional_costs {
            total = total.combine(&add.mana);
        }
    }
    total
}

/// The chosen cast option's cost as printed, X still in it.
///
/// Split out from [`wizard_cost`] because the two are wanted at different
/// moments: everything downstream of the X question wants the cost with X
/// filled in, and the X question itself has to look at the cost that still
/// says X.
fn chosen_option_cost(wizard: &CastWizard) -> ManaCost {
    wizard
        .options
        .iter()
        .find(|o| Some(o.kind) == wizard.option)
        .map_or(ManaCost::ZERO, |o| o.cost)
}
