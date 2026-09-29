//! The casting wizard: multi-step spell casting (CR 601.2a–h).
//!
//! Modes/alternative costs → X → kicker → targets → pitch choices →
//! payment. Each step suspends into a pending request; the wizard resumes
//! on the answer. Everything ends in one `SpellCast` event — atomic from
//! the outside.

use super::{
    AbilityDef, CardLookup, Cause, Engine, EngineError, GameEvent, ObjectId, ObjectKind, Pending,
    PlayerId, SmallVec, ZoneLocation, ZonePosition, eval,
};
use crate::casting;
use crate::choice::{
    CastModeDesc, CastModeKind, ChoicePrompt, NumberPrompt, TargetPrompt, YesNoPrompt,
};
use crate::object::GameObject;
use baylee_cards_dsl::{AltCondition, CostPart, SpellMode, TargetReq, TargetSpec};
use baylee_core::ids::NameRef;
use baylee_core::mana::ManaCost;

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
    /// How many times to pay the replicate cost (CR 702.56a), announced
    /// with the other additional costs (CR 601.2b) and so before targets.
    /// Skipped when the face has none or the pool pays for none.
    Replicate,
    /// Pitch choice (exile-from-hand).
    PitchChoice,
    /// Delve choice (exile-from-graveyard, {1} each).
    Delve,
    /// The tap question of convoke (creatures) or of a paid waterbend
    /// (artifacts and creatures), {1} each.
    Convoke,
    /// The additional cost's "sacrifice a creature" (CR 601.2h): which
    /// permanent pays each `Sacrifice` part of
    /// `FaceDef::mandatory_additional_costs`, one question per part.
    Sacrifice,
    /// Ready to pay and cast.
    Done,
    /// Escape's other cards to exile (CR 702.138a), between the pitch and
    /// delve: skipped by every cast but `CastModeKind::Escape`.
    Escape,
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
    /// How many times the replicate cost is paid.
    pub replicated: u8,
    /// Cards chosen for pitch (exile-from-hand).
    pub pitch: SmallVec<[ObjectId; 2]>,
    /// Cards chosen to delve (exile-from-graveyard, {1} each).
    pub delve_exiles: SmallVec<[ObjectId; 8]>,
    /// The other cards escape's cost exiles from the graveyard. They pay no
    /// mana, which is what keeps them apart from `delve_exiles`.
    pub escape_exiles: SmallVec<[ObjectId; 8]>,
    /// Permanents chosen to tap for convoke or a waterbend ({1} each).
    pub convoke_taps: SmallVec<[ObjectId; 8]>,
    /// Permanents chosen to pay the additional cost's sacrifices, one per
    /// asking part in the order the face prints them.
    pub sacrifices: SmallVec<[ObjectId; 1]>,
    /// Current stage.
    pub stage: WizardStage,
    /// Options computed at start (kept for the Done stage).
    pub options: Vec<CastModeDesc>,
    /// Whether this cast is free (rebound, suspend finish).
    pub free: bool,
    /// An effect casting it as it resolves, paying its costs (CR 608.2g,
    /// Conduit of Worlds); `None` for every other cast.
    pub by_effect: Option<EffectCast>,
}

/// A cast an effect makes as it resolves, paying the card's costs
/// (CR 608.2g). No graveyard permission is spent on it, because none was
/// used.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EffectCast {
    /// "You may cast that card."
    Plain,
    /// "…If you do, you can't cast additional spells this turn": set on the
    /// caster once the spell has been cast.
    ThenNoMoreSpells,
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
/// Toxic Deluge's `PayLifeX`, and the "as an additional cost to cast this
/// spell, sacrifice a creature" of Natural Order, Eldritch Evolution,
/// Neoform and Crop Rotation. The sacrifice is the one part here that names
/// an object, so it is the one that is *gated*: the `Sacrifice` stage asks
/// which permanent pays it from `cost_wizard::options`, and
/// `casting::can_cast_form` refuses the cast on a board where that list is
/// empty — the same reader, so the offer and the question agree.
///
/// The rest of the list has no gate at all: nothing runs `can_afford` over
/// it.
///
/// Which is why the one bound this list has is on the *question* instead: the
/// `XValue` stage caps X at the caster's life total (CR 119.4), because
/// `finish_cast` below subtracts what it is given and reads no total. A
/// `PayLife(n)` written here has no such stage and would still be paid past
/// zero — it belongs beside the X the day a card prints one.
pub(crate) const fn paid_as_a_mandatory_additional_cost(part: &CostPart) -> bool {
    matches!(
        part,
        CostPart::PayLifeX | CostPart::PayLife(_) | CostPart::Sacrifice(_)
    )
}

/// The parts of a spell's additional cost that ask which permanent pays
/// them, in printed order: the questions the `Sacrifice` stage puts.
pub(crate) fn additional_sacrifices(
    face: &'static baylee_cards_dsl::FaceDef,
) -> impl Iterator<Item = &'static CostPart> {
    face.mandatory_additional_costs
        .iter()
        .filter(|p| matches!(p, CostPart::Sacrifice(_)))
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
            replicated: 0,
            pitch: SmallVec::new(),
            delve_exiles: SmallVec::new(),
            escape_exiles: SmallVec::new(),
            convoke_taps: SmallVec::new(),
            sacrifices: SmallVec::new(),
            stage: WizardStage::ChooseMode,
            options,
            free: false,
            by_effect: None,
        };
        if wizard.options.len() == 1 {
            wizard.option = Some(wizard.options[0].kind);
            wizard.kicked = wizard.option == Some(CastModeKind::Kicked);
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
        if !casting::may_begin_casting(&self.state, player) {
            return Ok(());
        }
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
            replicated: 0,
            pitch: SmallVec::new(),
            delve_exiles: SmallVec::new(),
            escape_exiles: SmallVec::new(),
            convoke_taps: SmallVec::new(),
            sacrifices: SmallVec::new(),
            // A miracle cost is paid "rather than its mana cost" (CR 702.94a),
            // which makes it an alternative cost, and an alternative cost
            // with an {X} in it announces X like any other (CR 107.3a) —
            // Entreat the Dead's `{X}{B}{B}` was cast for X = 0 and returned
            // nobody. `XValue` falls through to `Targets` when there is none.
            stage: WizardStage::XValue,
            options,
            free: false,
            by_effect: None,
        };
        let _ = &mut wizard;
        self.cast_wizard = Some(wizard);
        self.advance_cast_wizard()
    }

    /// Starts a free cast (rebound at upkeep, suspend finish, a discovered
    /// card): no payment, but targets and other choices still run through
    /// the wizard.
    pub(crate) fn start_free_cast(
        &mut self,
        player: PlayerId,
        card: ObjectId,
    ) -> Result<(), EngineError> {
        let Some(def) = self
            .state
            .object(card)
            .filter(|o| o.zone == crate::zone::Zone::Exile)
            .and_then(|o| o.card)
            .and_then(|c| self.lookup.card(c.index))
        else {
            return Err(EngineError::IllegalAction("no card to cast from exile"));
        };
        if !casting::may_begin_casting(&self.state, player) {
            return Err(EngineError::IllegalAction("no more spells this turn"));
        }
        // A spell whose every effect sits under a mode is cast *in* a mode
        // (CR 700.2), a free cast included: cast `Normal`, Damn went to the
        // stack as a spell with nothing to do and resolved to nothing.
        let options = if casting::modes_are_the_only_way(def, 0) {
            let options = self.free_modes(player, card, def);
            if options.is_empty() {
                return Err(EngineError::IllegalAction("no way to cast this spell"));
            }
            options
        } else {
            Vec::new()
        };
        let (option, stage) = match options.as_slice() {
            [] => (Some(CastModeKind::Normal), WizardStage::Kicker),
            [only] => (Some(only.kind), WizardStage::Kicker),
            _ => (None, WizardStage::ChooseMode),
        };
        let mut wizard = CastWizard {
            card,
            player,
            option,
            targets: SmallVec::new(),
            second_targets: SmallVec::new(),
            target_players: baylee_core::ids::SeatSet::new(),
            chosen_player: None,
            x: 0,
            kicked: false,
            replicated: 0,
            pitch: SmallVec::new(),
            delve_exiles: SmallVec::new(),
            escape_exiles: SmallVec::new(),
            convoke_taps: SmallVec::new(),
            sacrifices: SmallVec::new(),
            // Straight past `XValue`, and deliberately: a spell cast paying
            // neither its mana cost nor an alternative cost with X in it has
            // exactly one legal X, which is 0 (CR 107.3b). A mode chosen
            // from several comes back through `XValue`, which asks nothing:
            // every mode offered here costs nothing.
            stage,
            options,
            free: true,
            by_effect: None,
        };
        let _ = &mut wizard;
        self.cast_wizard = Some(wizard);
        self.advance_cast_wizard()
    }

    /// Starts a cast under a play permission that waives the mana cost
    /// (Dauthi Voidwalker's "you may play it this turn without paying its
    /// mana cost"): no payment and X = 0, as [`Self::start_free_cast`], but
    /// started by the player from a priority round, so a modal spell still
    /// announces its modes (CR 601.2b) — each at no cost — instead of being
    /// cast as a `Normal` that has nothing to resolve.
    pub(crate) fn start_permitted_free_cast(
        &mut self,
        player: PlayerId,
        card: ObjectId,
    ) -> Result<(), EngineError> {
        let def = self
            .state
            .object(card)
            .and_then(|o| o.card)
            .and_then(|c| self.lookup.card(c.index))
            .ok_or(EngineError::IllegalAction("unknown card"))?;
        if !casting::may_begin_casting(&self.state, player) {
            return Err(EngineError::IllegalAction("no more spells this turn"));
        }
        // The modes a free cast may choose, as `start_free_cast` offers them
        // (`free_modes`): each at no cost, a spree's sets with their own.
        let modal_only = casting::modes_are_the_only_way(def, 0);
        let options = if modal_only {
            self.free_modes(player, card, def)
        } else {
            Vec::new()
        };
        if modal_only && options.is_empty() {
            return Err(EngineError::IllegalAction("no way to cast this spell"));
        }
        let single = options.len() <= 1;
        let option = if modal_only {
            options.first().map(|o| o.kind)
        } else {
            Some(CastModeKind::Normal)
        };
        let wizard = CastWizard {
            card,
            player,
            option: if single { option } else { None },
            targets: SmallVec::new(),
            second_targets: SmallVec::new(),
            target_players: baylee_core::ids::SeatSet::new(),
            chosen_player: None,
            x: 0,
            kicked: false,
            replicated: 0,
            pitch: SmallVec::new(),
            delve_exiles: SmallVec::new(),
            escape_exiles: SmallVec::new(),
            convoke_taps: SmallVec::new(),
            sacrifices: SmallVec::new(),
            // `XValue` asks nothing of a free wizard (CR 107.3b); from there
            // on it is every other cast.
            stage: if single {
                WizardStage::XValue
            } else {
                WizardStage::ChooseMode
            },
            options,
            free: true,
            by_effect: None,
        };
        self.cast_wizard = Some(wizard);
        self.advance_cast_wizard()
    }

    /// Starts the cast `Effect::MayCastTarget` said yes to, once its payment
    /// window has closed (CR 608.2g): the card's own mana cost, paid out of
    /// the pool the window filled, with no timing asked, because nobody is
    /// casting it with priority. X is announced as for any cast paying a
    /// cost with X in it (CR 107.3a); targets and every other choice run
    /// through the wizard as usual.
    ///
    /// Nothing is cast when the card is no longer the object that was
    /// targeted (CR 400.7) or its caster may cast nothing more this turn.
    pub(crate) fn start_paid_cast(
        &mut self,
        player: PlayerId,
        card: ObjectId,
        version: u32,
        then_no_more_spells: bool,
    ) -> Result<(), EngineError> {
        let cost = self
            .state
            .object(card)
            .filter(|o| o.version == version)
            .map(|o| o.characteristics().mana_cost)
            .ok_or(EngineError::IllegalAction("the card has gone"))?;
        if !casting::may_begin_casting(&self.state, player) {
            return Err(EngineError::IllegalAction("no more spells this turn"));
        }
        let wizard = CastWizard {
            card,
            player,
            option: Some(CastModeKind::Normal),
            targets: SmallVec::new(),
            second_targets: SmallVec::new(),
            target_players: baylee_core::ids::SeatSet::new(),
            chosen_player: None,
            x: 0,
            kicked: false,
            replicated: 0,
            pitch: SmallVec::new(),
            delve_exiles: SmallVec::new(),
            escape_exiles: SmallVec::new(),
            convoke_taps: SmallVec::new(),
            sacrifices: SmallVec::new(),
            stage: WizardStage::XValue,
            options: vec![CastModeDesc {
                index: 0,
                kind: CastModeKind::Normal,
                cost,
            }],
            free: false,
            by_effect: Some(if then_no_more_spells {
                EffectCast::ThenNoMoreSpells
            } else {
                EffectCast::Plain
            }),
        };
        self.cast_wizard = Some(wizard);
        self.advance_cast_wizard()
    }

    /// The modes a modal spell may be cast in without paying its mana cost,
    /// each at no cost.
    ///
    /// Not a mode with a cost of its own: overload is an alternative cost
    /// (CR 702.96a), "without paying its mana cost" is one too (CR 118.9),
    /// and only one alternative cost can be applied to a spell (CR 118.9a).
    /// A mode no target can be found for is not offered either, as
    /// `cast_options` offers none.
    fn free_modes(
        &self,
        player: PlayerId,
        card: ObjectId,
        def: &baylee_cards_dsl::CardDef,
    ) -> Vec<CastModeDesc> {
        let mut options = Vec::new();
        for ability in def.abilities_for_face(0) {
            let AbilityDef::ModalSpell { modes, choose } = ability else {
                continue;
            };
            if !choose.is_one() {
                // A set of modes, and each mode's own cost with it: those
                // are additional costs, added to the alternative cost a free
                // cast is (CR 118.9d), so a free Final Showdown still pays
                // {1} for each "+ {1}" it chooses. Offered only when the pool
                // already holds it, as every cost in the wizard is.
                let with_restricted =
                    casting::spendable_pool(&self.state, player, casting::SpendFor::Spell(card));
                let pool = with_restricted
                    .as_ref()
                    .unwrap_or(&self.state.players[player.get() as usize].mana_pool);
                for set in casting::mode_sets(modes, *choose) {
                    let cost = casting::mode_set_cost(ManaCost::ZERO, modes, set);
                    if casting::wild_or_not(casting::mana_is_wild(&self.state), pool, &cost)
                        && casting::mode_set_has_legal_targets(
                            &self.state,
                            &self.lookup,
                            player,
                            card,
                            set,
                        )
                    {
                        options.push(CastModeDesc {
                            index: u8::try_from(options.len()).unwrap_or(u8::MAX),
                            kind: CastModeKind::Modes(set),
                            cost,
                        });
                    }
                }
                continue;
            }
            for (i, mode) in modes.iter().enumerate() {
                if mode.cost_override.is_none()
                    && casting::mode_has_a_legal_target(&self.state, &self.lookup, player, card, i)
                {
                    options.push(CastModeDesc {
                        index: u8::try_from(options.len()).unwrap_or(u8::MAX),
                        kind: CastModeKind::Mode(i),
                        cost: ManaCost::ZERO,
                    });
                }
            }
        }
        options
    }

    /// Whether [`Self::start_free_cast`] can put `card` on the stack for
    /// `player` right now: the probe a discovered card is offered on
    /// (CR 701.57a), so that "cast it?" is only asked of a card that can be.
    ///
    /// The refusals the wizard can meet with nothing to pay: a land (never a
    /// spell), a cast an effect forbids (CR 601.3a), a mode or target that
    /// cannot be chosen (CR 601.2c), and an additional sacrifice with nothing
    /// to sacrifice (CR 601.2h). Timing is not one of them: a spell cast
    /// while something resolves ignores it (CR 608.2g).
    pub(crate) fn free_cast_possible(&self, player: PlayerId, card: ObjectId) -> bool {
        let Some(obj) = self.state.object(card) else {
            return false;
        };
        let Some(def) = obj.card.and_then(|c| self.lookup.card(c.index)) else {
            return false;
        };
        if obj
            .characteristics()
            .types
            .contains(baylee_core::types::TypeSet::LAND)
            || casting::cast_is_forbidden(&self.state, player, obj)
        {
            return false;
        }
        if additional_sacrifices(&def.faces[0])
            .any(|part| super::cost_wizard::options(&self.state, player, card, part).is_empty())
        {
            return false;
        }
        if casting::modes_are_the_only_way(def, 0) {
            return !self.free_modes(player, card, def).is_empty();
        }
        casting::face_has_a_legal_target(&self.state, &self.lookup, player, card, 0)
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
        let with_restricted =
            casting::spendable_pool(&self.state, player, casting::SpendFor::Spell(card));
        let pool = with_restricted
            .as_ref()
            .unwrap_or(&self.state.players[player.get() as usize].mana_pool);
        // Commander tax (CR 903.8): {2} more generic for each previous cast
        // of this commander from the command zone. It is a cost increase, so
        // it lands on every way of casting the card, an alternative cost
        // included (CR 601.2f) — and on the affordability probes too, or a
        // mode would be offered that the player then cannot pay for.
        let tax = casting::commander_tax(&self.state, player, card);
        // Convoke (CR 702.51a) pays {1} per untapped creature and
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
            // A graveyard permission for the front (Muldrotha, Wrenn's
            // emblem) is a second way, offered below beside the backs.
            let front_too = self.state.object(card).is_some_and(|o| {
                casting::graveyard_cast_permission(&self.state, player, o).is_some()
            });
            if !front_too {
                return Ok(options);
            }
        }
        // Printed flashback (CR 702.34a) from the graveyard: its own cost,
        // "rather than paying its mana cost" — and beside it, where a grant
        // also reaches the card (Past in Flames) or a permission casts any
        // spell from there (Forgotten Cellar), the mana cost as `Normal`.
        // Nothing else is cast from a graveyard: one alternative cost at a
        // time (CR 118.9a).
        let in_graveyard = self
            .state
            .object(card)
            .is_some_and(|o| o.zone == crate::zone::Zone::Graveyard);
        // Escape (CR 702.138a) from the graveyard: its mana and the other
        // cards it exiles, the count asked of the list the wizard offers.
        // Beside it, where a permission casts the card from there (Muldrotha,
        // Wrenn's emblem), the mana cost as `Normal`.
        if in_graveyard && let Some(escape) = face.escape {
            let permitted = self.state.object(card).is_some_and(|o| {
                casting::graveyard_cast_permission(&self.state, player, o).is_some()
            });
            if permitted && afford(&face.mana_cost.with_x(0)) {
                options.push(CastModeDesc {
                    index: 0,
                    kind: CastModeKind::Normal,
                    cost: face.mana_cost.with_more_generic(tax),
                });
            }
            let fodder = casting::escape_exile_options(&self.state, player, card).len();
            if afford(&escape.cost.with_x(0)) && fodder >= usize::from(escape.exile) {
                options.push(CastModeDesc {
                    index: options.len() as u8,
                    kind: CastModeKind::Escape,
                    cost: escape.cost.with_more_generic(tax),
                });
            }
            if options.is_empty() {
                return Err(EngineError::IllegalAction("no way to cast this spell"));
            }
            return Ok(options);
        }
        if in_graveyard && let Some(flashback) = face.flashback {
            let at_mana_cost = casting::flashback_granted(&self.state, card)
                || self.state.object(card).is_some_and(|o| {
                    casting::graveyard_cast_permission(&self.state, player, o).is_some()
                });
            if at_mana_cost && afford(&face.mana_cost.with_x(0)) {
                options.push(CastModeDesc {
                    index: 0,
                    kind: CastModeKind::Normal,
                    cost: face.mana_cost.with_more_generic(tax),
                });
            }
            if afford(&flashback.with_x(0)) {
                options.push(CastModeDesc {
                    index: options.len() as u8,
                    kind: CastModeKind::Flashback,
                    cost: flashback.with_more_generic(tax),
                });
            }
            if options.is_empty() {
                return Err(EngineError::IllegalAction("no way to cast this spell"));
            }
            return Ok(options);
        }
        // Conditional cost reduction printed on the card (Surgical
        // Metamorph & co.), through the same reader `can_cast` uses — this
        // was the second place the two probes disagreed, and in the other
        // direction from convoke: the wizard knew the discount and the offer
        // did not, so the seat entitled to it was never shown the card.
        let normal_cost = face.mana_cost.with_less_generic(casting::printed_reduction(
            &self.state,
            face,
            player,
            card,
        ));
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
            && casting::can_cast_form(&self.state, &self.lookup, player, card, None).is_ok()
            && casting::has_a_printed_cost(&face.mana_cost)
            && afford(&normal_cost.with_x(0))
            && (face.kicked_targets.is_none()
                || casting::ordinary_targets_reachable(&self.state, def, player, card))
        {
            options.push(CastModeDesc {
                index: options.len() as u8,
                kind: CastModeKind::Normal,
                cost: normal_cost.with_more_generic(tax),
            });
        }
        if let Some(req) = face.kicked_targets {
            let cost = casting::kicked_mana_cost(face)
                .with_less_generic(casting::printed_reduction(&self.state, face, player, card));
            if afford(&cost.with_x(0))
                && casting::requirement_is_reachable(Some(req), &self.state, player, card)
            {
                options.push(CastModeDesc {
                    index: options.len() as u8,
                    kind: CastModeKind::Kicked,
                    cost: cost.with_more_generic(tax),
                });
            }
        }
        if let Some(prototype) = face.prototype
            && casting::can_cast_form(
                &self.state,
                &self.lookup,
                player,
                card,
                Some(casting::SpellForm::Prototype(prototype)),
            )
            .is_ok()
        {
            options.push(CastModeDesc {
                index: options.len() as u8,
                kind: CastModeKind::Prototype,
                cost: prototype.cost.with_more_generic(tax),
            });
        }
        if face.disguise.is_some() {
            let cost = const { ManaCost::parse("{3}") };
            if casting::can_cast_form(
                &self.state,
                &self.lookup,
                player,
                card,
                Some(casting::SpellForm::Disguise),
            )
            .is_ok()
            {
                options.push(CastModeDesc {
                    index: options.len() as u8,
                    kind: CastModeKind::Disguise,
                    cost: cost.with_more_generic(tax),
                });
            }
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
            // The spell's own payment, restricted mana included: `can_cast`
            // probes an alternative cost against the same pool, and a probe
            // that read the plain pool here offered an evoke off a Cavern in
            // `LegalActions` and then refused it as "no way to cast this
            // spell".
            if !condition_ok
                || !self.can_afford(player, card, &taxed, casting::SpendFor::Spell(card))
            {
                continue;
            }
            options.push(CastModeDesc {
                index: (options.len()) as u8,
                kind: CastModeKind::Alternative(i),
                cost: alt.cost.mana.with_more_generic(tax),
            });
        }
        // Dash (CR 702.109a): its cost "rather than its mana cost", which is
        // an alternative cost (CR 118.9) paid by the rules for one (601.2b,
        // 601.2f–h) — asked of the pool the way the alternatives above are.
        if let Some(dash) = face.dash {
            let taxed = baylee_cards_dsl::Cost {
                mana: dash.with_more_generic(tax),
                parts: &[],
            };
            if self.can_afford(player, card, &taxed, casting::SpendFor::Spell(card)) {
                options.push(CastModeDesc {
                    index: (options.len()) as u8,
                    kind: CastModeKind::Dash,
                    cost: taxed.mana,
                });
            }
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
        // Modal spells (overload & friends): one option per mode — or, for a
        // spell that chooses more than one, one per set of modes it may be
        // cast with, each priced with its modes' own costs (CR 700.2h).
        for ability in def.abilities {
            let AbilityDef::ModalSpell { modes, choose } = ability else {
                continue;
            };
            if !choose.is_one() {
                for set in casting::mode_sets(modes, *choose) {
                    let cost = casting::mode_set_cost(face.mana_cost, modes, set);
                    if afford(&cost.with_x(0))
                        && casting::mode_set_has_legal_targets(
                            &self.state,
                            &self.lookup,
                            player,
                            card,
                            set,
                        )
                    {
                        options.push(CastModeDesc {
                            index: (options.len()) as u8,
                            kind: CastModeKind::Modes(set),
                            cost: cost.with_more_generic(tax),
                        });
                    }
                }
                continue;
            }
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
        // Timing, per way of casting (CR 601.3): the front's for every option
        // that casts the front, and a back face's own for that face
        // (`casting::face_timing_allows`, CR 712.11c, 715.3a). An enchantment
        // with an instant Adventure, cast while the stack is not empty, is
        // offered as the Adventure alone. Renumbered, because an option's
        // index is its place in this list.
        let front = obj.characteristics();
        let front_now = casting::timing_allows(&self.state, player, front.types, front.keywords);
        options.retain(|o| match o.kind {
            CastModeKind::Face(i) => casting::face_timing_allows(&self.state, player, def, i),
            _ => front_now,
        });
        for (i, option) in options.iter_mut().enumerate() {
            option.index = u8::try_from(i).unwrap_or(u8::MAX);
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

    /// The most times the caster may announce paying `cost`, a replicate
    /// cost, on top of the rest of this cast; `0` when not once.
    ///
    /// CR 702.56a allows any number, so what bounds the question is what can
    /// be paid: a cast is paid out of the floating pool (`finish_cast`), so
    /// the count is the largest the pool covers beside everything the cast
    /// already costs, asked through the reader the payment spends by. The
    /// generic mana delve, convoke or a paid waterbend could still take off
    /// in the stages after this one is counted as paid, which makes it an
    /// upper bound: the question never offers less than the caster could
    /// pay, and a count the payment then cannot cover unwinds the whole
    /// cast (CR 601.2h), as a printed X too large does. Capped at
    /// [`X_CEILING`], which is also how many copies the trigger can list.
    fn replicate_bound(
        &self,
        wizard: &CastWizard,
        face: &baylee_cards_dsl::FaceDef,
        cost: &ManaCost,
    ) -> u32 {
        let player = wizard.player;
        let graveyard = self.state.zones.list(ZoneLocation::Graveyard(player)).len();
        let help = [
            (face.delve, graveyard),
            (
                face.convoke,
                casting::convoke_sources(&self.state, player).len(),
            ),
            (
                face.waterbend && wizard.kicked,
                casting::waterbend_sources(&self.state, player).len(),
            ),
        ]
        .iter()
        .filter(|(applies, _)| *applies)
        .map(|(_, n)| u32::try_from(*n).unwrap_or(u32::MAX))
        .fold(0_u32, u32::saturating_add);
        let before = wizard_total_cost(face, wizard);
        (1..=X_CEILING)
            .take_while(|n| {
                self.can_pay_mana(
                    player,
                    spend_for(wizard, face),
                    &times(before, cost, *n).with_less_generic(help),
                )
            })
            .last()
            .unwrap_or(0)
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
                // A spell cast without paying its mana cost has exactly one
                // legal X, which is 0 (CR 107.3b), so a free cast is not
                // asked for it; a life payment that scales with X still is.
                let needs_x = (cost.has_variable() && !wizard.free) || pays_life_x;
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
                    // to a state-based action on the way to resolving. The
                    // bound is `life_payable`, the one `can_pay_life` reads,
                    // so a player who can't lose life is offered X = 0 only
                    // (CR 119.8, CR 119.4b).
                    let mut max = X_CEILING;
                    if pays_life_x {
                        let life = self.state.life_payable(wizard.player);
                        max = max.min(u32::try_from(life).unwrap_or(0));
                    }
                    self.pending = Pending::ChooseNumber {
                        player: wizard.player,
                        min: 0,
                        max,
                        reason: NumberPrompt::X,
                    };
                    self.awaiting_answer = true;
                    return Ok(());
                }
                let mut wizard = wizard;
                wizard.stage = WizardStage::Kicker;
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
                let (min, max) = req.bounds(wizard.x);
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
                    wizard.stage = WizardStage::PitchChoice;
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
                    wizard.stage = WizardStage::PitchChoice;
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
                if face.additional_costs.is_empty()
                    || (face.kicked_targets.is_some() && !wizard.free)
                {
                    let mut wizard = wizard;
                    wizard.stage = WizardStage::Replicate;
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
            WizardStage::Replicate => {
                let face = self.wizard_face(&wizard);
                let (cost, max) = face.replicate.map_or((ManaCost::ZERO, 0), |cost| {
                    (cost, self.replicate_bound(&wizard, face, &cost))
                });
                if max == 0 {
                    // Nothing to ask: no replicate, or not one payment the
                    // pool covers, and zero is then the only answer.
                    let mut wizard = wizard;
                    wizard.stage = WizardStage::Targets;
                    self.cast_wizard = Some(wizard);
                    return self.advance_cast_wizard();
                }
                self.pending = Pending::ChooseNumber {
                    player: wizard.player,
                    min: 0,
                    max,
                    reason: NumberPrompt::Replicate { cost },
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
                wizard.stage = WizardStage::Escape;
                self.cast_wizard = Some(wizard);
                self.advance_cast_wizard()
            }
            WizardStage::Escape => {
                let exile = (wizard.option == Some(CastModeKind::Escape))
                    .then(|| self.wizard_face(&wizard).escape)
                    .flatten()
                    .map(|escape| escape.exile);
                let Some(n) = exile else {
                    let mut wizard = wizard;
                    wizard.stage = WizardStage::Delve;
                    self.cast_wizard = Some(wizard);
                    return self.advance_cast_wizard();
                };
                // The same list the offer counted, so the question is never
                // shorter than its answer.
                let options =
                    casting::escape_exile_options(&self.state, wizard.player, wizard.card);
                if options.len() < usize::from(n) {
                    self.cast_wizard = None;
                    return Err(EngineError::IllegalAction(
                        "not enough other cards in the graveyard to escape",
                    ));
                }
                self.pending = Pending::ChooseCards {
                    player: wizard.player,
                    options,
                    min: n,
                    max: n,
                    prompt: ChoicePrompt::CostExile,
                };
                self.awaiting_answer = true;
                Ok(())
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
                // Two keywords tap permanents to pay, and they differ in
                // what they tap and in how much of the cost the taps may pay
                // (#229). A waterbend's taps exist only once its cost is paid
                // and pay that cost's generic mana alone (CR 701.67b). Convoke
                // may pay the spell's whole total cost (CR 702.51a), but
                // `finish_cast` takes the taps off its generic mana only, so
                // the bound is the generic delve has left; #230 teaches the
                // payment a creature's colour and moves this bound with it.
                // A bound the payment cannot honour is a tap spent for
                // nothing, as in the delve stage above.
                let generic =
                    |cost: &ManaCost| usize::try_from(cost.generic_total()).unwrap_or(usize::MAX);
                let (sources, room) = if face.waterbend && wizard.kicked {
                    let waterbend = face
                        .additional_costs
                        .iter()
                        .fold(ManaCost::ZERO, |total, add| total.combine(&add.mana));
                    (
                        crate::casting::waterbend_sources(&self.state, wizard.player),
                        generic(&waterbend),
                    )
                } else if face.convoke {
                    (
                        crate::casting::convoke_sources(&self.state, wizard.player),
                        generic(&wizard_total_cost(face, &wizard))
                            .saturating_sub(wizard.delve_exiles.len()),
                    )
                } else {
                    (Vec::new(), 0)
                };
                if sources.is_empty() || room == 0 {
                    let mut wizard = wizard;
                    wizard.stage = WizardStage::Sacrifice;
                    self.cast_wizard = Some(wizard);
                    return self.advance_cast_wizard();
                }
                // The bound is what is actually on the table, not a sentinel.
                // `99` is what a player casting a waterbend spell was shown:
                // "choose up to 99 targets", over two creatures, for a
                // question that is not targeting at all. *Which* permanents
                // tap is the player's, so every source stays in `options`.
                let max = u8::try_from(sources.len().min(room)).unwrap_or(u8::MAX);
                self.pending = Pending::ChooseTargets {
                    player: wizard.player,
                    options: sources,
                    player_options: Vec::new(),
                    min: 0,
                    max,
                    reason: TargetPrompt::Convoke,
                };
                self.awaiting_answer = true;
                Ok(())
            }
            WizardStage::Sacrifice => {
                let face = self.wizard_face(&wizard);
                let asked = wizard.sacrifices.len();
                let Some(part) = additional_sacrifices(face).nth(asked) else {
                    let mut wizard = wizard;
                    wizard.stage = WizardStage::Done;
                    self.cast_wizard = Some(wizard);
                    return self.advance_cast_wizard();
                };
                // The reader `casting::can_cast_form` refused an empty board
                // with, less what an earlier part of this cost already named:
                // nothing is paid until every question is answered
                // (CR 601.2h), so one creature must not pay two sacrifices.
                let mut options =
                    super::cost_wizard::options(&self.state, wizard.player, wizard.card, part);
                options.retain(|id| !wizard.sacrifices.contains(id));
                if options.is_empty() {
                    self.cast_wizard = None;
                    return Err(EngineError::IllegalAction("nothing can pay this cost"));
                }
                self.pending = Pending::ChooseCards {
                    player: wizard.player,
                    options,
                    min: 1,
                    max: 1,
                    prompt: super::cost_wizard::prompt(part),
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
        if matches!(wizard.option, Some(CastModeKind::Disguise)) {
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
        if wizard.kicked
            && let Some(req) = def.faces[face_index].kicked_targets
        {
            return Some(req);
        }
        let abilities = def.abilities_for_face(face_index);
        match wizard.option {
            Some(CastModeKind::Mode(i)) => abilities.iter().find_map(|a| match a {
                AbilityDef::ModalSpell { modes, .. } => {
                    modes.get(i).and_then(|m: &SpellMode| m.targets)
                }
                _ => None,
            }),
            // Several modes: the first of them that says "target" is the
            // spell's first instance of the word (CR 700.2c), the second
            // its second — `wizard_second_target_req`.
            Some(CastModeKind::Modes(set)) => targeted_mode(abilities, set, 0),
            _ => abilities.iter().find_map(|a| match a {
                AbilityDef::Spell { targets, .. } => *targets,
                _ => None,
            }),
        }
    }

    /// The spell's requirement for a second instance of the word "target",
    /// if it prints one: [`AbilityDef::Spell`]'s, the chosen mode's
    /// (Archdruid's Charm's second mode says "target" twice), or, for a set
    /// of chosen modes, the second of them that says "target" (Three Steps
    /// Ahead, CR 700.2c).
    pub(super) fn wizard_second_target_req(&self, wizard: &CastWizard) -> Option<TargetReq> {
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
                AbilityDef::ModalSpell { modes, .. } => {
                    modes.get(i).and_then(|m: &SpellMode| m.second_targets)
                }
                _ => None,
            }),
            Some(CastModeKind::Modes(set)) => targeted_mode(abilities, set, 1),
            _ => abilities.iter().find_map(|a| match a {
                AbilityDef::Spell { second_targets, .. } => *second_targets,
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
        // What the pool held of each color before the payment, restricted
        // units included: the colors it holds less of afterwards are the
        // colors spent (converge).
        let held_before = units_by_color(&self.state.players[player.get() as usize].mana_pool);
        // A free cast pays nothing but what is added to it: a kicker, the
        // replicate cost, or the costs of the modes it chose (CR 118.9d).
        let owed = pays_mana(wizard) || total != ManaCost::ZERO;
        if owed {
            // Restricted mana (Cavern of Souls & co.) is spent where the
            // spell may spend it, and a rider applies for each entry that
            // actually paid. A failed payment has touched nothing.
            let Some(spent) =
                casting::pay_mana_for(&mut self.state, player, spend_for(wizard, face), &total)
            else {
                self.cast_wizard = None;
                return Err(EngineError::IllegalAction("cannot pay the total cost"));
            };
            self.apply_spend_riders(player, wizard.card, &spent);
        }
        // The mana is paid, so the rest of the cost may now be spent.
        for &card in wizard.delve_exiles.iter().chain(&wizard.escape_exiles) {
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
                        self.state.change_life(player, -i32::from(*n), Cause::Cost);
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
                    | CostPart::Crew(_)
                    | CostPart::ReturnToHand(_)
                    | CostPart::ExileFromGraveyard(_)
                    | CostPart::RemoveCounterSelf { .. }
                    | CostPart::RemoveCounterSelfX { .. }
                    | CostPart::PutCounterSelf { .. }
                    | CostPart::PayLifeX => {}
                }
            }
        }
        // Mandatory additional cost parts (e.g. Toxic Deluge's pay X life).
        let mut sacrificed = wizard.sacrifices.iter();
        let mut sacrificed_mana_value = None;
        for part in face.mandatory_additional_costs {
            if !paid_as_a_mandatory_additional_cost(part) {
                continue;
            }
            match part {
                CostPart::PayLifeX => {
                    self.state
                        .change_life(player, -(wizard.x as i32), Cause::Cost);
                }
                CostPart::PayLife(n) => {
                    self.state.change_life(player, -i32::from(*n), Cause::Cost);
                }
                // The answer the `Sacrifice` stage took for this part, through
                // the door an activation's sacrifice goes through. Its mana
                // value is read first, off the permanent as it last existed on
                // the battlefield (CR 608.2h): "the sacrificed creature's
                // mana value" is a question the spell asks as it resolves,
                // when the card is already in a graveyard and a copy has
                // stopped being one.
                CostPart::Sacrifice(_) => {
                    let Some(&chosen) = sacrificed.next() else {
                        self.cast_wizard = None;
                        return Err(EngineError::IllegalAction(
                            "a sacrifice the cast never asked about",
                        ));
                    };
                    sacrificed_mana_value = self
                        .state
                        .object(chosen)
                        .map(|o| o.characteristics().mana_value());
                    super::cost_wizard::pay(&mut self.state, player, part, chosen)?;
                }
                // Already skipped, by [`paid_as_a_mandatory_additional_cost`]
                // above, and named for the reason the alternative-cost match
                // names its own — with the sharper edge that no `can_afford`
                // guards this list, so the three asking costs are skipped
                // here rather than refused.
                CostPart::TapSelf
                | CostPart::UntapSelf
                | CostPart::SacrificeSelf
                | CostPart::Discard(_)
                | CostPart::TapOther(_)
                | CostPart::Crew(_)
                | CostPart::ReturnToHand(_)
                | CostPart::ExileFromGraveyard(_)
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
        // The face being cast prints "exile it instead" (a disturb back): the
        // object still shows the face it had in the graveyard, so the face is
        // read off the cast option, as the target requirement above is.
        let exiles_itself = {
            let face = match wizard.option {
                Some(CastModeKind::Face(i)) => i,
                _ => 0,
            };
            self.state
                .object(card)
                .and_then(|o| o.card)
                .and_then(|c| self.lookup.card(c.index))
                .is_some_and(|def| {
                    def.abilities_for_face(face.min(def.faces.len() - 1))
                        .iter()
                        .any(|a| {
                            matches!(
                                a,
                                AbilityDef::Replacement(
                                    baylee_cards_dsl::ReplacementRule::ExileSelfInsteadOfGraveyard
                                )
                            )
                        })
                })
        };
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
            obj.replicated = wizard.replicated;
            obj.alt_cast = matches!(wizard.option, Some(CastModeKind::Alternative(_)));
            obj.chosen_player = wizard.chosen_player;
            // Where it was cast from, read before it moves: a card cast from
            // exile under a play permission (Expressive Iteration) or from
            // the command zone was not cast from a hand, and rebound reads
            // this (CR 702.88a).
            obj.cast_from_hand = !wizard.free && obj.zone == crate::zone::Zone::Hand;
            // A permanent spell cast from the graveyard under Muldrotha's
            // allowance uses a type of it for the turn. Not a disturb back,
            // and not a card a permission of its own opened.
            if obj.zone == crate::zone::Zone::Graveyard
                && wizard.by_effect.is_none()
                && !matches!(
                    wizard.option,
                    Some(CastModeKind::Face(_) | CastModeKind::Escape)
                )
                && casting::play_permission(&self.state, player, card).is_none()
            {
                let obj = self.state.object(card).expect("wizard card exists");
                let permission = casting::graveyard_cast_permission(&self.state, player, obj);
                let types = obj.characteristics().types;
                casting::note_graveyard_play(&mut self.state, player, permission, types);
            }
            // Flashback (CR 702.34a): "If the flashback cost was paid, exile
            // this card". A permission to cast from the graveyard is not
            // flashback (Wrenn's emblem, Muldrotha, Forgotten Cellar), so a
            // spell it let through leaves the stack the usual way. An
            // instant or sorcery cast from there otherwise came by a
            // flashback, printed or granted, or an effect cast it, and none
            // of those used the permission.
            let by_permission = wizard.by_effect.is_none()
                && !matches!(wizard.option, Some(CastModeKind::Flashback))
                && self.state.object(card).is_some_and(|o| {
                    casting::graveyard_cast_permission(&self.state, player, o).is_some()
                });
            let obj = self.state.object_mut(card).expect("wizard card exists");
            if obj.zone == crate::zone::Zone::Graveyard {
                if !by_permission
                    && obj.characteristics().types.intersects(
                        baylee_core::types::TypeSet::INSTANT
                            .union(baylee_core::types::TypeSet::SORCERY),
                    )
                {
                    obj.riders.push(crate::object::Rider::Flashback);
                }
                obj.cast_from_hand = false;
            }
            // Every cast decides afresh: the same card cast later from a
            // hand, front face up, is not exiled for what it once was.
            obj.riders
                .retain(|r| *r != crate::object::Rider::ExileInsteadOfGraveyard);
            // Dash (CR 702.109a): the spell was cast for its dash cost, and
            // the permanent it becomes has haste and goes back at the next
            // end step (`Engine::finalize_spell` writes that trigger).
            obj.riders.retain(|r| *r != crate::object::Rider::Dashed);
            if wizard.option == Some(CastModeKind::Dash) {
                obj.riders.push(crate::object::Rider::Dashed);
            }
            // Escape (CR 702.138b): the spell escaped, and so will the
            // permanent it becomes.
            obj.riders.retain(|r| *r != crate::object::Rider::Escaped);
            if wizard.option == Some(CastModeKind::Escape) {
                obj.riders.push(crate::object::Rider::Escaped);
            }
            if exiles_itself {
                obj.riders
                    .push(crate::object::Rider::ExileInsteadOfGraveyard);
            }
            obj.mode_index = match wizard.option {
                Some(CastModeKind::Mode(i)) => Some(i.try_into().expect("mode index fits u8")),
                _ => None,
            };
            obj.modes = match wizard.option {
                Some(CastModeKind::Modes(set)) => set,
                _ => 0,
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
        // What was paid, written after the move for the reason the move
        // gives it up: the spell on the stack is the object that reads it.
        // A free cast spent no mana (CR 601.2h pays nothing it was not
        // asked for), whatever its printed cost says.
        let mana_spent = if owed { total.cmc() } else { 0 };
        let held_after = units_by_color(&self.state.players[player.get() as usize].mana_pool);
        let colors_spent = baylee_core::color::Color::ALL
            .into_iter()
            .zip(held_before.into_iter().zip(held_after))
            .filter(|(_, (before, after))| after < before)
            .fold(baylee_core::color::ColorSet::EMPTY, |set, (color, _)| {
                set.union(baylee_core::color::ColorSet::of(color))
            });
        if (mana_spent > 0 || sacrificed_mana_value.is_some())
            && let Some(obj) = self.state.object_mut(card)
        {
            obj.paid = Some(Box::new(crate::object::PaidRecord {
                sacrificed_mana_value,
                mana_spent,
                colors_spent,
                tapped: None,
            }));
        }
        if matches!(wizard.option, Some(CastModeKind::Prototype))
            && let Some(prototype) = face.prototype
            && let Some(obj) = self.state.object_mut(card)
        {
            obj.original_base = Some(std::sync::Arc::clone(&obj.base));
            obj.prototyped = true;
            let base = obj.base_mut();
            base.mana_cost = prototype.cost;
            base.colors = prototype.cost.colors();
            base.power = Some(prototype.power);
            base.toughness = Some(prototype.toughness);
            obj.cache.clear();
        }
        if matches!(wizard.option, Some(CastModeKind::Disguise)) {
            self.make_disguised(card);
        }
        // Per-turn tracking for conditional triggers (Esper Sentinel).
        if !matches!(wizard.option, Some(CastModeKind::Disguise))
            && !face.types.contains(baylee_core::types::TypeSet::CREATURE)
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
        // "If you do, you can't cast additional spells this turn": the
        // spell is cast, so the lock is on.
        if wizard.by_effect == Some(EffectCast::ThenNoMoreSpells)
            && let Some(v) = self
                .state
                .per_turn
                .no_more_spells
                .get_mut(player.get() as usize)
        {
            *v = true;
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

impl<L: CardLookup> Engine<L> {
    /// Applies spend riders after a payment (uncounterable marks, scry
    /// triggers), for restricted mana and for mana whose rider named the
    /// spell.
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
        // Each unit spent is its own rider: a replacement that made more of
        // the mana made more delayed triggers (CR 106.6a).
        let units = riders
            .iter()
            .flat_map(|(mana, _, rider)| std::iter::repeat_n(rider, usize::from(mana.amount)));
        for rider in units {
            match rider {
                baylee_cards_dsl::SpendRider::None => {}
                baylee_cards_dsl::SpendRider::Uncounterable => {
                    if let Some(obj) = self.state.object_mut(spell)
                        && !obj.riders.contains(&crate::object::Rider::Uncounterable)
                    {
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
    if wizard.kicked && wizard.option != Some(CastModeKind::Kicked) {
        for add in face.additional_costs {
            total = total.combine(&add.mana);
        }
    }
    if let Some(replicate) = face.replicate {
        total = times(total, &replicate, u32::from(wizard.replicated));
    }
    total
}

/// How many units of each color `pool` holds, plain and restricted alike,
/// in [`baylee_core::color::Color::ALL`]'s order.
fn units_by_color(pool: &baylee_core::mana::ManaPool) -> [u32; 5] {
    baylee_core::color::Color::ALL.map(|color| {
        let mana = baylee_core::mana::ManaColor::from_color(color);
        u32::from(pool.available(mana))
            + pool
                .restricted()
                .iter()
                .filter(|r| r.color == mana)
                .map(|r| u32::from(r.amount))
                .sum::<u32>()
    })
}

/// The `nth` of the chosen modes that says "target" (0 or 1), and what it
/// asks for: a spell cast with several modes points each of up to two of
/// them at something, as its first and second instance of the word
/// (CR 700.2c). `lints::modes_fault` holds a spell that chooses several to
/// at most two such modes.
fn targeted_mode(abilities: &'static [AbilityDef], set: u8, nth: usize) -> Option<TargetReq> {
    abilities.iter().find_map(|a| match a {
        AbilityDef::ModalSpell { modes, .. } => casting::chosen_modes(modes, set)
            .filter_map(|(_, mode)| mode.targets)
            .nth(nth),
        _ => None,
    })
}

/// `total` with `cost` added `n` times: the replicate cost paid `n` times
/// (CR 702.56a, 601.2f).
fn times(total: ManaCost, cost: &ManaCost, n: u32) -> ManaCost {
    (0..n).fold(total, |total, _| total.combine(cost))
}

/// Whether this cast pays mana at all. A free cast pays none of its mana
/// cost (CR 601.2h pays nothing it was not asked for), but an additional
/// cost it chose is still paid (CR 118.9d, 601.2f): a kicker, and the
/// replicate cost however many times it was announced.
const fn pays_mana(wizard: &CastWizard) -> bool {
    !wizard.free || wizard.kicked || wizard.replicated > 0
}

/// What the cast's mana pays for, which restricted mana asks (CR 106.6):
/// the one reader of the payment in `finish_cast` and of the replicate
/// question's bound, so the question never offers a count the payment
/// would refuse over a Cavern of Souls' mana.
fn spend_for(wizard: &CastWizard, face: &baylee_cards_dsl::FaceDef) -> casting::SpendFor {
    match wizard.option {
        Some(CastModeKind::Prototype) => casting::SpendFor::SpellAs(
            wizard.card,
            casting::SpellForm::Prototype(face.prototype.expect("prototype option")),
        ),
        Some(CastModeKind::Disguise) => {
            casting::SpendFor::SpellAs(wizard.card, casting::SpellForm::Disguise)
        }
        _ => casting::SpendFor::Spell(wizard.card),
    }
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
