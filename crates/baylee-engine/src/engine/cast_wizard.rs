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
use baylee_cards_dsl::{AltCondition, CostPart, SpellMode, TargetReq, TargetSpec};
use baylee_core::mana::ManaCost;

mod commit;
mod division;
mod grants;
mod linked;
mod north_star;
mod options;
mod permissions;
mod pricing;
mod stages;
mod transformed;
use pricing::FrozenReduction;

/// The current replicate-copy limit. Ordinary X choices are bounded by
/// resources, never this limit.
pub(crate) const REPLICATE_CEILING: u32 = 50;

const MASKED_FACE: baylee_cards_dsl::FaceDef = baylee_cards_dsl::FaceDef {
    types: baylee_core::types::TypeSet::CREATURE,
    power: Some(2),
    toughness: Some(2),
    ..baylee_cards_dsl::FaceDef::DEFAULT
};

/// Where the wizard currently is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum WizardStage {
    /// Distribute damage after targets and before any costs (CR 601.2d).
    DivideDamage,
    /// Choosing a cast mode (skip when only one option).
    ChooseMode,
    /// Choose which graveyard permission authorizes the announced spell.
    Permission,
    /// Optional spending permission for this spell’s mana cost only.
    ManaPermission,
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
    /// A creature returned to pay the selected graveyard alternative cost.
    ReturnCreature,
    /// Behold a permanent you control or reveal a card from your hand, or pay
    /// the alternative additional mana cost (CR 701.60a).
    Behold,
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
#[allow(clippy::struct_excessive_bools)] // independent casting facts (kicked, behold_paid_mana, free, masked)
pub(crate) struct CastWizard {
    /// The card being cast.
    pub card: ObjectId,
    /// The casting player.
    pub player: PlayerId,
    /// Chosen cast option.
    pub option: Option<CastModeKind>,
    /// Public cost route before separating its face and modal choices.
    pub announced: Option<CastModeKind>,
    /// An effect constrains this cast to a transforming back face.
    pub forced_face: Option<u8>,
    /// Consumed only after this cast has successfully paid every cost.
    pub north_star: Option<baylee_core::ids::EffectId>,
    /// Permission captured before paying any costs; consumed only on success.
    pub permission: Option<casting::GraveyardPermission>,
    /// Alternatives behind a permission question, indexed by its displayed choices.
    pub permissions: Vec<casting::GraveyardPermission>,
    /// Reductions frozen after selecting a casting form, before mana payments.
    pub reduction: Option<FrozenReduction>,
    /// Exact announced identities, preserved across all later cost payments.
    pub target_references: crate::sources::TargetReferences,
    /// Shares in canonical target order (objects, then players).
    pub division: Vec<u32>,
    /// Chosen targets.
    pub targets: SmallVec<[ObjectId; 2]>,
    /// Targets chosen for the second instance of the word, kept apart for
    /// the reason `GameObject::second_targets` gives.
    pub second_targets: SmallVec<[ObjectId; 1]>,
    /// Players named by the second target instance.
    pub second_target_players: baylee_core::ids::SeatSet,
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
    /// The creature returned for beam me up, chosen without targeting.
    pub returned: Option<ObjectId>,
    /// The permanent or card in hand chosen to behold (CR 701.60a).
    pub beheld: Option<ObjectId>,
    /// Whether the caster is paying `behold_or_pay`'s mana cost instead of
    /// beholding.
    pub behold_paid_mana: bool,
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
    /// How each Phyrexian symbol of the chosen option's cost is paid, in the
    /// cost's own order: `true` is 2 life, `false` its mana (CR 107.4f),
    /// announced with the other costs (CR 601.2b) and charged as the cost
    /// is paid (CR 601.2h). Asked at the head of the `XValue` stage, so
    /// the X bound reads the settled cost.
    pub phyrexian: SmallVec<[bool; 4]>,
    /// Current stage.
    pub stage: WizardStage,
    /// Options computed at start (kept for the Done stage).
    pub options: Vec<CastModeDesc>,
    /// Whether this cast is free (rebound, suspend finish).
    pub free: bool,
    /// This nested cast uses the costless, nameless 2/2 face.
    pub masked: bool,
    /// An effect casting it as it resolves, paying its costs (CR 608.2g,
    /// Conduit of Worlds); `None` for every other cast.
    pub by_effect: Option<EffectCast>,
    /// Where the game stood as this cast began from priority, to put back
    /// if it is cancelled (`PlayerAction::CancelCast`) or its window is
    /// passed short. `None` for a cast an effect makes (rebound, a miracle,
    /// "you may cast it"), which only the effect can take back.
    pub started: Option<super::cancel_cast::CastStart>,
    /// `DuringTheCast` for a card cast before its mana was made, from
    /// `LegalActions::payable`: its choices are made first and its mana in a
    /// payment window after them (CR 601.2g). `FromThePool` otherwise.
    pub paying: casting::Paying,
    /// Where the payment window this cast was started from began, for a
    /// cast `Effect::MayCastTarget` said yes to: its mana was made for this
    /// cast, and is given back with the next window's if the cast is not
    /// made (`Engine::give_back_window`). `None` for every other cast.
    pub window_start: Option<Box<super::WindowStart>>,
}

impl CastWizard {
    pub(super) fn face_index(&self) -> usize {
        if let Some(face) = self.forced_face {
            return usize::from(face);
        }
        match self.announced.or(self.option) {
            Some(CastModeKind::Face(face) | CastModeKind::EscapeGranted { face, .. }) => face,
            _ => 0,
        }
    }

    fn escapes(&self) -> bool {
        matches!(
            self.announced.or(self.option),
            Some(CastModeKind::Escape | CastModeKind::EscapeGranted { .. })
        )
    }

    fn escape_count(&self, face: &baylee_cards_dsl::FaceDef) -> Option<u8> {
        match self.announced.or(self.option) {
            Some(CastModeKind::EscapeGranted { exile, .. }) => Some(exile),
            Some(CastModeKind::Escape) => face.escape.map(|escape| escape.exile),
            _ => None,
        }
    }
    /// This cast makes mana after announcing X and targets (CR 601.2b, g).
    /// Effect casts currently make it before entering this wizard instead.
    fn makes_mana_after_choices(&self) -> bool {
        self.option == Some(CastModeKind::Miracle)
            || self.masked
            || self.by_effect.is_some()
            || self.paying == casting::Paying::DuringTheCast
    }

    /// The answers held by a miracle payment window, for replay comparison.
    /// Its mode is fixed, but identical boards can still owe different casts.
    pub(super) fn miracle_payment_fingerprint(&self) -> u64 {
        let mut hash = crate::state::structural_fingerprint(&(
            self.permission,
            self.announced,
            self.forced_face,
            self.returned,
            self.beheld,
            self.behold_paid_mana,
            self.north_star,
        ))
        .wrapping_mul(31)
        .wrapping_add(u64::from(self.card.slot()))
        .wrapping_mul(31)
        .wrapping_add(u64::from(self.card.generation()))
        .wrapping_mul(31)
        .wrapping_add(u64::from(self.x))
        .wrapping_mul(2)
        .wrapping_add(u64::from(self.kicked))
        .wrapping_mul(257)
        .wrapping_add(u64::from(self.replicated))
        .wrapping_mul(17)
        .wrapping_add(self.chosen_player.map_or(0, |p| u64::from(p.get()) + 1))
        .wrapping_mul(2)
        .wrapping_add(u64::from(self.paying == casting::Paying::DuringTheCast));
        hash = hash
            .wrapping_mul(2)
            .wrapping_add(u64::from(self.reduction.is_some()));
        if let Some(reduction) = self.reduction {
            hash = hash
                .wrapping_mul(31)
                .wrapping_add(u64::from(reduction.total));
            hash = hash
                .wrapping_mul(31)
                .wrapping_add(u64::from(reduction.remainder));
            hash = hash
                .wrapping_mul(31)
                .wrapping_add(crate::state::structural_fingerprint(&reduction.allowance));
        }
        hash = hash
            .wrapping_mul(31)
            .wrapping_add(crate::state::structural_fingerprint(
                &self.second_target_players,
            ));
        for targets in [
            self.targets.as_slice(),
            self.second_targets.as_slice(),
            self.pitch.as_slice(),
            self.delve_exiles.as_slice(),
            self.escape_exiles.as_slice(),
            self.convoke_taps.as_slice(),
            self.sacrifices.as_slice(),
        ] {
            hash = hash.wrapping_mul(31).wrapping_add(targets.len() as u64);
            for target in targets {
                hash = hash
                    .wrapping_mul(31)
                    .wrapping_add(u64::from(target.slot()))
                    .wrapping_mul(31)
                    .wrapping_add(u64::from(target.generation()));
            }
        }
        hash = hash
            .wrapping_mul(31)
            .wrapping_add(self.division.len() as u64);
        for share in &self.division {
            hash = hash.wrapping_mul(31).wrapping_add(u64::from(*share));
        }
        let seats = self
            .target_players
            .iter()
            .fold(0, |bits, p| bits | (1_u64 << p.get()));
        // The Phyrexian answers, with their count so `[]` and `[false]`
        // differ: each `true` is 2 life owed.
        let phyrexian = self
            .phyrexian
            .iter()
            .take(32)
            .enumerate()
            .fold(self.phyrexian.len() as u64, |bits, (i, life)| {
                bits | (u64::from(*life) << (i + 8))
            });
        hash.wrapping_mul(65_537)
            .wrapping_add(seats)
            .wrapping_mul(31)
            .wrapping_add(phyrexian)
    }
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
    /// "…and mana of any type can be spent to cast that spell."
    AnyMana,
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
        CostPart::PayLifeX
            | CostPart::PayLife(_)
            | CostPart::Sacrifice(_)
            | CostPart::ExilePermanent(_)
            | CostPart::ExileFromGraveyard(_)
            | CostPart::Discard(_)
            | CostPart::DiscardHand
            | CostPart::DiscardLastDrawnThisTurn
    )
}

/// The mandatory additional cost parts for `face`, plus entwine's non-mana
/// cost parts when `option` chooses all modes (CR 702.42a).
pub(crate) fn wizard_mandatory_parts(
    face: &'static baylee_cards_dsl::FaceDef,
    option: Option<CastModeKind>,
) -> impl Iterator<Item = &'static CostPart> {
    let entwine_parts = match (option, face.entwine) {
        (Some(CastModeKind::Modes(_)), Some(entwine)) => entwine.parts,
        _ => &[],
    };
    face.mandatory_additional_costs
        .iter()
        .chain(entwine_parts.iter())
}

/// The parts of a spell's additional cost that ask which permanent pays
/// them, in printed order: the questions the `Sacrifice` stage puts.
pub(crate) fn additional_card_costs(
    face: &'static baylee_cards_dsl::FaceDef,
) -> impl Iterator<Item = &'static CostPart> {
    additional_card_costs_for(face, None)
}

pub(crate) fn additional_card_costs_for(
    face: &'static baylee_cards_dsl::FaceDef,
    option: Option<CastModeKind>,
) -> impl Iterator<Item = &'static CostPart> {
    wizard_mandatory_parts(face, option).filter(|p| {
        matches!(
            p,
            CostPart::Sacrifice(_)
                | CostPart::ExilePermanent(_)
                | CostPart::ExileFromGraveyard(_)
                | CostPart::Discard(_)
        )
    })
}

impl<L: CardLookup> Engine<L> {
    /// Starts the casting wizard for `card` (validated castable).
    pub(crate) fn start_cast_wizard(
        &mut self,
        player: PlayerId,
        card: ObjectId,
    ) -> Result<(), EngineError> {
        // A card the pool cannot pay for, cast to be paid for as it is cast
        // (CR 601.2g): the offer said so (`LegalActions::payable`).
        let pay_in_window = !self.commanded_card(card)
            && casting::can_cast(&self.state, &self.lookup, player, card).is_err()
            && self.compute_legal(player).payable.contains(&card);
        let options = self.cast_options(player, card, pay_in_window)?;
        let mut wizard = CastWizard {
            card,
            player,
            option: None,
            announced: None,
            forced_face: None,
            north_star: None,
            permission: None,
            permissions: Vec::new(),
            reduction: None,
            target_references: crate::sources::TargetReferences::default(),
            division: Vec::new(),
            targets: SmallVec::new(),
            second_targets: SmallVec::new(),
            second_target_players: baylee_core::ids::SeatSet::new(),
            target_players: baylee_core::ids::SeatSet::new(),
            chosen_player: None,
            x: 0,
            kicked: false,
            replicated: 0,
            pitch: SmallVec::new(),
            returned: None,
            beheld: None,
            behold_paid_mana: false,
            delve_exiles: SmallVec::new(),
            escape_exiles: SmallVec::new(),
            convoke_taps: SmallVec::new(),
            sacrifices: SmallVec::new(),
            phyrexian: SmallVec::new(),
            stage: WizardStage::ChooseMode,
            options,
            free: false,
            masked: false,
            by_effect: self.commanded_card(card).then_some(EffectCast::Plain),
            window_start: None,
            paying: if pay_in_window {
                casting::Paying::DuringTheCast
            } else {
                casting::Paying::FromThePool
            },
            started: (!self.commanded_card(card)).then(|| self.cast_start()),
        };
        if wizard.options.len() == 1 {
            wizard.option = Some(wizard.options[0].kind);
            wizard.kicked = wizard.option == Some(CastModeKind::Kicked);
            wizard.stage = WizardStage::Permission;
        }
        self.cast_wizard = Some(wizard);
        self.advance_cast_wizard()
    }

    /// A resolving instruction supplies the face-down alternative cost.
    pub(super) fn start_masked_cast(
        &mut self,
        player: PlayerId,
        card: ObjectId,
    ) -> Result<(), EngineError> {
        let object = self
            .state
            .object(card)
            .ok_or(EngineError::IllegalAction("card left hand"))?;
        if object.zone != crate::zone::Zone::Hand || object.zone_owner != Some(player) {
            return Err(EngineError::IllegalAction("card left hand"));
        }
        let projected = casting::SpellForm::Disguise.project(object);
        if !casting::may_begin_casting(&self.state, player)
            || casting::cast_is_forbidden(&self.state, player, &projected)
        {
            return Err(EngineError::IllegalAction("casting prohibited"));
        }
        let cost =
            casting::spell_adjustment(&self.state, player, &projected).price(ManaCost::ZERO, 0);
        self.cast_wizard = Some(CastWizard {
            card,
            player,
            option: Some(CastModeKind::Normal),
            announced: None,
            forced_face: None,
            north_star: None,
            permission: None,
            permissions: Vec::new(),
            reduction: None,
            target_references: crate::sources::TargetReferences::default(),
            division: Vec::new(),
            targets: SmallVec::new(),
            second_targets: SmallVec::new(),
            second_target_players: baylee_core::ids::SeatSet::new(),
            target_players: baylee_core::ids::SeatSet::new(),
            chosen_player: None,
            x: 0,
            kicked: false,
            replicated: 0,
            pitch: SmallVec::new(),
            returned: None,
            beheld: None,
            behold_paid_mana: false,
            delve_exiles: SmallVec::new(),
            escape_exiles: SmallVec::new(),
            convoke_taps: SmallVec::new(),
            sacrifices: SmallVec::new(),
            phyrexian: SmallVec::new(),
            stage: WizardStage::Done,
            options: vec![CastModeDesc {
                index: 0,
                kind: CastModeKind::Normal,
                cost,
            }],
            free: true,
            masked: true,
            by_effect: Some(EffectCast::Plain),
            window_start: None,
            paying: casting::Paying::FromThePool,
            started: None,
        });
        self.advance_cast_wizard()
    }

    /// Starts a miracle cast (CR 702.94): timing-free, at the miracle
    /// cost; targets and other choices still run through the wizard.
    pub(crate) fn start_miracle_cast(
        &mut self,
        player: PlayerId,
        card: ObjectId,
    ) -> Result<(), EngineError> {
        let wizard = self
            .miracle_wizard(player, card)
            .ok_or(EngineError::IllegalAction("not a miracle card"))?;
        self.cast_wizard = Some(wizard);
        self.advance_cast_wizard()
    }

    /// Whether `player` could choose every target a miracle cast of `card`
    /// requires, which is what decides whether the miracle is offered.
    ///
    /// Choosing the targets is a step of casting (CR 601.2c), and a spell
    /// that cannot take it cannot be cast: the attempt is illegal and is
    /// reversed (CR 601.2, CR 732.1). A "yes" to such a miracle can only
    /// come back as a "no", so the question has one outcome and is not
    /// asked. Banishing Stroke drawn onto a board with no artifact, creature
    /// or enchantment was asked anyway; the house said yes, the engine
    /// refused it after spending the offer, and three of r001's games could
    /// not be replayed.
    ///
    /// Read the way the wizard's own target stages read it — the same
    /// requirement, the same bounds at X = 0, the same enumerations — so the
    /// offer and the cast cannot disagree. X = 0 is the most any X spell
    /// could ask for less of. The cost is not checked here: whether it can
    /// be paid is the payment's answer, and a "yes" it cannot pay is taken
    /// and the cast reversed, which leaves the card in hand as a "no" does.
    pub(crate) fn miracle_targets_available(&self, player: PlayerId, card: ObjectId) -> bool {
        let Some(wizard) = self.miracle_wizard(player, card) else {
            return false;
        };
        if let Some(req) = self.wizard_target_req(&wizard) {
            let (min, max) = req.bounds(wizard.x);
            let players = eval::target_player_options(&self.state, &req.spec, player);
            if matches!(req.spec, TargetSpec::AnyPlayer | TargetSpec::AnyOpponent) {
                // The `ChoosePlayer` stage: one player, from this list.
                if players.is_empty() {
                    return false;
                }
            } else if max > 0 {
                let objects = self.wizard_target_options(&wizard, &req.spec);
                if u64::try_from(objects.len() + players.len()).unwrap_or(u64::MAX) < u64::from(min)
                {
                    return false;
                }
            }
        }
        self.wizard_second_target_req(&wizard).is_none_or(|req| {
            self.wizard_target_options(&wizard, &req.spec).len() >= usize::from(req.min)
        })
    }

    /// The wizard a miracle cast of `card` starts with, or `None` when the
    /// card prints no miracle or `player` may not cast a spell at all.
    fn miracle_wizard(&self, player: PlayerId, card: ObjectId) -> Option<CastWizard> {
        let cost = self
            .state
            .object(card)
            .and_then(|o| o.card)
            .and_then(|c| self.lookup.card(c.index))
            .and_then(|def| def.faces[0].miracle)?;
        // CR 601.3: a player who may not begin to cast a spell (Conduit of
        // Worlds) has no miracle cast either, so it is not offered.
        if !casting::may_begin_casting(&self.state, player) {
            return None;
        }
        let options = vec![CastModeDesc {
            index: 0,
            kind: CastModeKind::Miracle,
            cost: self.front_spell_price(player, card, cost),
        }];
        Some(CastWizard {
            card,
            player,
            option: Some(CastModeKind::Miracle),
            announced: None,
            forced_face: None,
            north_star: None,
            permission: None,
            permissions: Vec::new(),
            reduction: None,
            target_references: crate::sources::TargetReferences::default(),
            division: Vec::new(),
            targets: SmallVec::new(),
            second_targets: SmallVec::new(),
            second_target_players: baylee_core::ids::SeatSet::new(),
            target_players: baylee_core::ids::SeatSet::new(),
            chosen_player: None,
            x: 0,
            kicked: false,
            replicated: 0,
            pitch: SmallVec::new(),
            returned: None,
            beheld: None,
            behold_paid_mana: false,
            delve_exiles: SmallVec::new(),
            escape_exiles: SmallVec::new(),
            convoke_taps: SmallVec::new(),
            sacrifices: SmallVec::new(),
            phyrexian: SmallVec::new(),
            // A miracle cost is paid "rather than its mana cost" (CR 702.94a),
            // which makes it an alternative cost, and an alternative cost
            // with an {X} in it announces X like any other (CR 107.3a) —
            // Entreat the Dead's `{X}{B}{B}` was cast for X = 0 and returned
            // nobody. `XValue` falls through to `Targets` when there is none.
            stage: WizardStage::XValue,
            options,
            free: false,
            masked: false,
            by_effect: None,
            window_start: None,
            paying: casting::Paying::FromThePool,
            started: None,
        })
    }

    /// Price an alternative/front-face cost through the same increases and
    /// reductions as a normal cast. Waiving mana cost does not waive a tax.
    fn front_spell_price(&self, player: PlayerId, card: ObjectId, cost: ManaCost) -> ManaCost {
        let Some(def) = self
            .state
            .object(card)
            .and_then(|o| o.card)
            .and_then(|c| self.lookup.card(c.index))
        else {
            return cost;
        };
        casting::face_adjustment(&self.state, player, card, def, 0, None).price(
            cost,
            casting::printed_reduction(&self.state, &def.faces[0], player, card),
        )
    }

    fn free_normal_options(&self, player: PlayerId, card: ObjectId) -> Vec<CastModeDesc> {
        let base = self
            .state
            .object(card)
            .and_then(|o| o.card)
            .and_then(|c| self.lookup.card(c.index))
            .and_then(|def| def.faces[0].behold_or_pay)
            .filter(|b| casting::behold_options(&self.state, player, card, b.filter).is_empty())
            .map_or(ManaCost::ZERO, |b| b.cost);
        let min_cost = self.front_spell_price(player, card, base);
        if !self.free_cost_affordable(player, card, min_cost) {
            return Vec::new();
        }
        let cost = self.front_spell_price(player, card, ManaCost::ZERO);
        vec![CastModeDesc {
            index: 0,
            kind: CastModeKind::Normal,
            cost,
        }]
    }

    fn free_cost_affordable(&self, player: PlayerId, card: ObjectId, cost: ManaCost) -> bool {
        let reduction = self
            .state
            .object(card)
            .and_then(|o| o.card)
            .and_then(|c| self.lookup.card(c.index))
            .map_or(0, |def| {
                casting::keyword_reduction(&self.state, &def.faces[0], player, card)
            });
        let restricted =
            casting::planning_pool(&self.state, player, casting::SpendFor::Spell(card), 0);
        let pool = restricted
            .as_ref()
            .unwrap_or(&self.state.players[player.get() as usize].mana_pool);
        let what = casting::SpendFor::Spell(card);
        casting::source_affordable(
            &self.state,
            player,
            what,
            pool,
            &cost.with_less_generic(reduction),
            &[],
            Some(casting::live_allowance(&self.state, player, what).reduced(reduction)),
        )
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
            self.free_normal_options(player, card)
        };
        if options.is_empty() {
            return Err(EngineError::IllegalAction("cannot pay spell cost increase"));
        }
        self.start_free_cast_options(player, card, options, false, None)
    }

    fn start_free_cast_options(
        &mut self,
        player: PlayerId,
        card: ObjectId,
        options: Vec<CastModeDesc>,
        by_effect: bool,
        forced_face: Option<u8>,
    ) -> Result<(), EngineError> {
        let (option, stage) = match options.as_slice() {
            [] => return Err(EngineError::IllegalAction("no legal free play")),
            [only] => (Some(only.kind), WizardStage::XValue),
            _ => (None, WizardStage::ChooseMode),
        };
        let mut wizard = CastWizard {
            card,
            player,
            option,
            announced: None,
            forced_face,
            north_star: None,
            permission: None,
            permissions: Vec::new(),
            reduction: None,
            target_references: crate::sources::TargetReferences::default(),
            division: Vec::new(),
            targets: SmallVec::new(),
            second_targets: SmallVec::new(),
            second_target_players: baylee_core::ids::SeatSet::new(),
            target_players: baylee_core::ids::SeatSet::new(),
            chosen_player: None,
            x: 0,
            kicked: false,
            replicated: 0,
            pitch: SmallVec::new(),
            returned: None,
            beheld: None,
            behold_paid_mana: false,
            delve_exiles: SmallVec::new(),
            escape_exiles: SmallVec::new(),
            convoke_taps: SmallVec::new(),
            sacrifices: SmallVec::new(),
            phyrexian: SmallVec::new(),
            // `XValue` fixes a waived mana-cost X at zero, but still asks
            // about independent additional costs such as Toxic Deluge's X life.
            stage,
            options,
            free: true,
            masked: false,
            by_effect: by_effect.then_some(EffectCast::Plain),
            window_start: None,
            paying: casting::Paying::FromThePool,
            started: None,
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
        let options = self.permitted_free_options(player, card, def);
        if options.is_empty() {
            return Err(EngineError::IllegalAction("no way to cast this spell"));
        }
        let single = options.len() <= 1;
        let option = options.first().map(|o| o.kind);
        let wizard = CastWizard {
            card,
            player,
            option: if single { option } else { None },
            announced: None,
            forced_face: None,
            north_star: None,
            permission: None,
            permissions: Vec::new(),
            reduction: None,
            target_references: crate::sources::TargetReferences::default(),
            division: Vec::new(),
            targets: SmallVec::new(),
            second_targets: SmallVec::new(),
            second_target_players: baylee_core::ids::SeatSet::new(),
            target_players: baylee_core::ids::SeatSet::new(),
            chosen_player: None,
            x: 0,
            kicked: false,
            replicated: 0,
            pitch: SmallVec::new(),
            returned: None,
            beheld: None,
            behold_paid_mana: false,
            delve_exiles: SmallVec::new(),
            escape_exiles: SmallVec::new(),
            convoke_taps: SmallVec::new(),
            sacrifices: SmallVec::new(),
            phyrexian: SmallVec::new(),
            // `XValue` asks nothing of a free wizard (CR 107.3b); from there
            // on it is every other cast.
            stage: if single {
                WizardStage::XValue
            } else {
                WizardStage::ChooseMode
            },
            options,
            free: true,
            masked: false,
            by_effect: None,
            window_start: None,
            paying: casting::Paying::FromThePool,
            started: None,
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
    ///
    /// `opened` is where that window began. The wizard carries it, so a
    /// second window its Done stage opens for what the pool still lacks
    /// gives back what both made if it closes without the cast (CR 732.1).
    pub(crate) fn start_paid_cast(
        &mut self,
        player: PlayerId,
        card: ObjectId,
        version: u32,
        then_no_more_spells: bool,
        any_mana: bool,
        opened: Box<super::WindowStart>,
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
            announced: None,
            forced_face: None,
            north_star: None,
            permission: None,
            permissions: Vec::new(),
            reduction: None,
            target_references: crate::sources::TargetReferences::default(),
            division: Vec::new(),
            targets: SmallVec::new(),
            second_targets: SmallVec::new(),
            second_target_players: baylee_core::ids::SeatSet::new(),
            target_players: baylee_core::ids::SeatSet::new(),
            chosen_player: None,
            x: 0,
            kicked: false,
            replicated: 0,
            pitch: SmallVec::new(),
            returned: None,
            beheld: None,
            behold_paid_mana: false,
            delve_exiles: SmallVec::new(),
            escape_exiles: SmallVec::new(),
            convoke_taps: SmallVec::new(),
            sacrifices: SmallVec::new(),
            phyrexian: SmallVec::new(),
            stage: WizardStage::XValue,
            options: vec![CastModeDesc {
                index: 0,
                kind: CastModeKind::Normal,
                cost: self.front_spell_price(player, card, cost),
            }],
            free: false,
            masked: false,
            by_effect: Some(if any_mana {
                EffectCast::AnyMana
            } else if then_no_more_spells {
                EffectCast::ThenNoMoreSpells
            } else {
                EffectCast::Plain
            }),
            window_start: Some(opened),
            paying: casting::Paying::FromThePool,
            started: None,
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
        self.free_modes_with_mana(player, card, def, false)
    }

    fn free_modes_with_mana(
        &self,
        player: PlayerId,
        card: ObjectId,
        def: &baylee_cards_dsl::CardDef,
        make_mana: bool,
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
                for set in casting::mode_sets(modes, *choose) {
                    let cost = self.front_spell_price(
                        player,
                        card,
                        casting::mode_set_cost(ManaCost::ZERO, modes, set),
                    );
                    if (make_mana || self.free_cost_affordable(player, card, cost))
                        && casting::mode_set_has_legal_targets(
                            &self.state,
                            &self.lookup,
                            player,
                            card,
                            set,
                        )
                    {
                        options.push(CastModeDesc {
                            index: u32::try_from(options.len()).unwrap_or(u32::MAX),
                            kind: CastModeKind::Modes(set),
                            cost,
                        });
                    }
                }
                continue;
            }
            for (i, mode) in modes.iter().enumerate() {
                let cost = self.front_spell_price(player, card, ManaCost::ZERO);
                if mode.cost_override.is_none()
                    && (make_mana || self.free_cost_affordable(player, card, cost))
                    && casting::mode_has_a_legal_target(&self.state, &self.lookup, player, card, i)
                {
                    options.push(CastModeDesc {
                        index: u32::try_from(options.len()).unwrap_or(u32::MAX),
                        kind: CastModeKind::Mode(i),
                        cost,
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
        if additional_card_costs(&def.faces[0])
            .any(|part| super::cost_wizard::options(&self.state, player, card, part).is_empty())
        {
            return false;
        }
        if casting::modes_are_the_only_way(def, 0) {
            return !self.free_modes(player, card, def).is_empty();
        }
        !self.free_normal_options(player, card).is_empty()
            && casting::face_has_a_legal_target(&self.state, &self.lookup, player, card, 0)
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
    /// [`REPLICATE_CEILING`], which is also how many copies the trigger can list.
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
                face.convoke || casting::has_improvise(&self.state, face, player),
                casting::tap_to_pay_sources(&self.state, face, player).len(),
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
        let before = wizard_total_before_reduction(face, wizard);
        let reduction =
            help.saturating_add(wizard.reduction.map_or(0, |reduction| reduction.remainder));
        casting::greatest_affordable(REPLICATE_CEILING, |n| {
            let payment = self.north_star_payment(
                wizard,
                before.combine_n(cost, n).with_less_generic(reduction),
            );
            self.can_pay_mana(player, spend_for(wizard, face), &payment)
        })
    }

    /// Drives the wizard forward until it needs an answer or finishes.
    ///
    /// A stage that cannot go on drops the wizard and touches nothing else:
    /// no stage sets the question before it has decided to ask it, and a
    /// failed payment "has touched nothing" (`finish_cast`). So on the first
    /// press of a cast, an error here leaves the engine as the press found
    /// it, which is what a refused `apply` must do. Past the first question
    /// the caster has *answered* something, and that is
    /// [`Self::continue_cast_wizard`]'s to settle.
    pub(crate) fn advance_cast_wizard(&mut self) -> Result<(), EngineError> {
        let result = self.advance_cast_wizard_inner();
        if result.is_err() {
            self.cast_wizard = None;
        }
        result
    }

    /// Carries a cast on after its caster answered one of its questions.
    ///
    /// The answer was one the question offered, so it is taken, whatever
    /// the rest of the checklist then finds. A step the caster cannot
    /// complete — the total cost with the X they named, the targets their
    /// mode needs — makes the casting illegal, and the game returns to the
    /// moment before it was proposed (CR 601.2, CR 732.1): that is the
    /// answer's consequence, not a reason to refuse it.
    ///
    /// It used to be refused *and* reversed, which is the one combination a
    /// game record cannot hold: a refused answer is not recorded, so a
    /// replay stayed inside the wizard while the table had gone back to
    /// priority, and the next recorded answer was put to the wrong question.
    ///
    /// What taking it costs: the reason is no longer handed back. Nobody read
    /// it — gamehost's `Session` answers every refusal with the same "illegal
    /// action for your seat" — and the caster now sees the priority they are
    /// back at instead. The reversal journals nothing, as CR 732.1 has it:
    /// an undone action leaves nothing behind.
    pub(crate) fn continue_cast_wizard(&mut self) {
        // Read before the stages run: `finish_cast` nulls the wizard itself
        // on the way out, so by the error branch there is nobody left to ask.
        let caster = self.cast_wizard.as_ref().map(|w| w.player);
        if self.advance_cast_wizard().is_err() {
            self.reverse_cast(caster);
        }
    }

    /// Returns the game to the moment before a cast was proposed (CR 601.2,
    /// CR 732.1), once the cast has turned out to be illegal part-way.
    pub(crate) fn reverse_cast(&mut self, caster: Option<PlayerId>) {
        // The X the card was given as it was announced goes with the rest
        // of the cast: a card that never left its zone announced nothing.
        if let Some(card) = self.cast_wizard.take().map(|w| w.card)
            && let Some(obj) = self.state.object_mut(card)
            && obj.zone != crate::zone::Zone::Stack
        {
            obj.x_value = 0;
        }
        self.awaiting_answer = false;
        if self.finish_nested_cast() {
            return;
        }
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

    /// The Phyrexian question of the chosen option's cost, one symbol at a
    /// time: CR 601.2b has the player announce, for each, whether they pay
    /// 2 life or its coloured mana. Asked only where both answers still
    /// leave a way to pay; where one does, it is taken without asking, and
    /// where neither does the cast is refused. `None` once every symbol is
    /// answered, or for a cast that pays no mana cost. The reader is
    /// `casting::phyrexian_affordable`, the one the offer asked.
    fn ask_phyrexian(&mut self, wizard: &CastWizard) -> Option<Result<(), EngineError>> {
        if wizard.free {
            return None;
        }
        let symbols =
            usize::try_from(chosen_option_cost(wizard).phyrexian_count()).unwrap_or(usize::MAX);
        if symbols == 0 || symbols > 8 || wizard.phyrexian.len() >= symbols {
            return None;
        }
        let face = self.wizard_face(wizard);
        let deferred = wizard.makes_mana_after_choices();
        // The price with every Phyrexian symbol still as printed, at its
        // lowest (X 0, no kicker yet), so the question never refuses an
        // answer some later choice could pay.
        let mut unsettled = wizard.clone();
        unsettled.phyrexian.clear();
        let price = wizard_total_cost(face, &unsettled);
        let spendable =
            casting::spendable_pool(&self.state, wizard.player, spend_for(wizard, face));
        let pool = spendable
            .as_ref()
            .unwrap_or(&self.state.players[usize::from(wizard.player.get())].mana_pool);
        let open = |settled: &[bool]| {
            let life =
                2 * i32::try_from(settled.iter().filter(|l| **l).count()).unwrap_or(i32::MAX);
            self.state.can_pay_life(wizard.player, life)
                && (deferred
                    || if wizard.by_effect == Some(EffectCast::AnyMana) {
                        casting::component_affordable(
                            &self.state,
                            wizard.player,
                            spend_for(wizard, self.wizard_face(wizard)),
                            pool,
                            &price,
                            &price,
                            settled,
                            self.wizard_source_allowance(wizard),
                        )
                    } else if let Some(component) = wizard
                        .north_star
                        .and_then(|_| self.north_star_component(wizard))
                    {
                        casting::component_affordable(
                            &self.state,
                            wizard.player,
                            spend_for(wizard, self.wizard_face(wizard)),
                            pool,
                            &price,
                            &component,
                            settled,
                            self.wizard_source_allowance(wizard),
                        )
                    } else {
                        casting::source_affordable(
                            &self.state,
                            wizard.player,
                            spend_for(wizard, self.wizard_face(wizard)),
                            pool,
                            &price,
                            settled,
                            Some(self.wizard_source_allowance(wizard)),
                        )
                    })
        };
        let mut answers = wizard.phyrexian.clone();
        let mut ask = false;
        while answers.len() < symbols {
            let mut with_life = answers.clone();
            with_life.push(true);
            let mut with_mana = answers.clone();
            with_mana.push(false);
            match (open(&with_life), open(&with_mana)) {
                (true, true) => {
                    ask = true;
                    break;
                }
                (true, false) => answers.push(true),
                (false, true) => answers.push(false),
                (false, false) => {
                    self.cast_wizard = None;
                    return Some(Err(EngineError::IllegalAction("cannot pay the cost")));
                }
            }
        }
        let mut wizard = wizard.clone();
        wizard.phyrexian = answers;
        if ask {
            let card = self.state.object(wizard.card).and_then(|o| o.card);
            self.pending = Pending::YesNo {
                player: wizard.player,
                prompt: YesNoPrompt::PayLife { amount: 2 },
                source: card.map(|c| {
                    baylee_core::ids::AbilityRef::new(c.index, baylee_core::ids::AbilityRef::SPELL)
                }),
            };
            self.cast_wizard = Some(wizard);
            self.awaiting_answer = true;
            return Some(Ok(()));
        }
        self.cast_wizard = Some(wizard);
        Some(self.advance_cast_wizard())
    }

    /// Opens the mana opportunity after choices fix the price (CR 601.2g).
    /// A miracle cannot float mana before its offer; a per-target increase
    /// likewise cannot be known until targets are chosen; and a card cast
    /// from `LegalActions::payable` makes all its mana here, so the
    /// triggers of that mana wait until the spell is cast (CR 601.2i). Keep the wizard
    /// outside the active choice slot while a mana ability asks its choices.
    ///
    /// A cast off the pool whose total the pool does not cover (an X larger
    /// than what floats, say) gets the same window: the caster may still
    /// activate mana abilities before paying (CR 601.2g), and is not sent
    /// back to priority for want of mana they could have made. From then on
    /// it is paid as a cast from `payable` is, so a window closed short
    /// hands the caster their priority back (CR 732.1).
    fn cast_or_make_miracle_mana(&mut self, wizard: CastWizard) -> Result<(), EngineError> {
        let cost = wizard_paid_total(self.wizard_face(&wizard), &wizard);
        if self.commanded_card(wizard.card) || !self.wizard_pool_can_pay(&wizard, cost) {
            let mut legal = self.compute_legal(wizard.player);
            self.narrow_to_mana(&mut legal);
            if legal.has_mana_source() {
                let mut wizard = wizard;
                if wizard.started.is_some() {
                    wizard.paying = casting::Paying::DuringTheCast;
                }
                let player = wizard.player;
                let version = self
                    .state
                    .object(wizard.card)
                    .expect("wizard card exists")
                    .version;
                self.cast_wizard = None;
                // A cast started from an earlier window keeps that window's
                // start: what it made was made for this cast too.
                let opened = wizard
                    .window_start
                    .clone()
                    .unwrap_or_else(|| Box::new(self.window_start(player)));
                self.mana_window = Some(super::PaymentWindow {
                    player,
                    suspended: super::PaymentContinuation::Miracle {
                        cost: self.north_star_payment(
                            &wizard,
                            self.restricted_x_payment(&wizard, cost).0,
                        ),
                        wizard: Box::new(wizard),
                        version,
                        opened,
                    },
                });
                self.pending = Pending::Priority {
                    player,
                    legal: Box::new(legal),
                };
                self.awaiting_answer = true;
                // A commanded card is played only "if able" (Word of
                // Command). A price its player's lands cannot pay at all is
                // a casting that cannot comply with CR 601.2g-h: it is
                // reversed (CR 732.1) before anything is tapped, and the
                // card stays where it was.
                if self.commanded_payment_feasible() == Some(false) {
                    self.mana_window = None;
                    return Err(EngineError::IllegalAction(
                        "the commanded card cannot be paid",
                    ));
                }
                return Ok(());
            }
        }
        self.finish_cast(&wizard)
    }

    /// Closing the mana opportunity tries the chosen cast once. A short
    /// payment leaves the card in its original zone and cannot reopen an offer.
    /// The captured version also protects casts from graveyard or exile.
    ///
    /// A window that closes without the cast gives back what was made in it
    /// (CR 732.1, `Engine::give_back_window`): declining a miracle the
    /// player had begun to pay for leaves the lands they tapped untapped and
    /// nothing floating.
    pub(super) fn finish_miracle_payment(
        &mut self,
        wizard: &CastWizard,
        version: u32,
        opened: &super::WindowStart,
    ) -> Result<bool, EngineError> {
        let current = self
            .state
            .object(wizard.card)
            .is_some_and(|card| card.version == version);
        if !current || self.finish_cast(wizard).is_err() {
            // Read before anything is put back: the pool as the window
            // closed is what was short, if anything was.
            let reason = if current
                && !self.wizard_pool_can_pay(
                    wizard,
                    wizard_paid_total(self.wizard_face(wizard), wizard),
                ) {
                crate::event::CastCancelReason::Unpaid
            } else {
                crate::event::CastCancelReason::Illegal
            };
            // A constrained activation cannot turn into freely floating mana
            // merely by declining the selected spell. Keep the payment question
            // and every obligation until a legal completion is submitted.
            if self
                .state
                .constrained_payment(wizard.player)
                .is_some_and(|payment| {
                    payment.card.object == wizard.card && !payment.required.is_empty()
                })
            {
                return Err(EngineError::IllegalAction("generated mana remains unspent"));
            }
            // A cast begun from priority is put back whole, its mana
            // abilities with it (CR 732.1); any other gives back the taps.
            if self.rewind_cast(wizard) {
                self.state.journal.record(GameEvent::CastCancelled {
                    player: wizard.player,
                    card: wizard.card,
                    reason,
                });
            } else {
                self.give_back_window(wizard.player, opened);
            }
            if let Some(card) = self.state.object_mut(wizard.card) {
                card.x_value = 0;
            }
            // A cast from priority that could not be paid is reversed, and
            // the player who had priority keeps it (CR 732.2).
            if !self.finish_nested_cast() && wizard.paying == casting::Paying::DuringTheCast {
                self.after_action(wizard.player);
            }
            return Ok(false);
        }
        Ok(true)
    }

    pub(super) fn wizard_target_options(
        &self,
        wizard: &CastWizard,
        spec: &TargetSpec,
    ) -> Vec<ObjectId> {
        self.wizard_target_options_relative(wizard, spec, None)
    }

    /// [`Self::wizard_target_options`] with the object a relational
    /// requirement is relative to: the first target, for the second
    /// instance of "target" (Glyph of Delusion's Wall).
    pub(super) fn wizard_target_options_relative(
        &self,
        wizard: &CastWizard,
        spec: &TargetSpec,
        reference: Option<ObjectId>,
    ) -> Vec<ObjectId> {
        let Some(def) = self
            .state
            .object(wizard.card)
            .and_then(|o| o.card)
            .and_then(|c| self.lookup.card(c.index))
        else {
            return Vec::new();
        };
        let face = wizard.face_index();
        let form = match spend_for(wizard, self.wizard_face(wizard)) {
            casting::SpendFor::SpellAs(_, form) => Some(form),
            casting::SpendFor::KickedSpell(_, _, form) => form,
            _ => None,
        };
        eval::spell_target_options_relative(
            spec,
            &self.state,
            wizard.player,
            wizard.card,
            Some((def, face)),
            form,
            eval::target_domain::walls(
                self.wizard_target_req(wizard),
                self.wizard_second_target_req(wizard),
            ),
            reference,
        )
    }

    pub(super) fn wizard_face(&self, wizard: &CastWizard) -> &'static baylee_cards_dsl::FaceDef {
        if wizard.masked {
            return &MASKED_FACE;
        }
        let card = self
            .state
            .object(wizard.card)
            .expect("wizard card exists")
            .card
            .expect("wizard card is card-backed");
        let def = self.lookup.card(card.index).expect("wizard card known");
        let face_index = wizard.face_index().min(def.faces.len() - 1);
        &def.faces[face_index]
    }

    pub(super) fn wizard_target_req(&self, wizard: &CastWizard) -> Option<TargetReq> {
        if wizard.masked || matches!(wizard.option, Some(CastModeKind::Disguise)) {
            return None;
        }
        let def = self
            .state
            .object(wizard.card)
            .and_then(|o| o.card)
            .and_then(|c| self.lookup.card(c.index))
            .expect("wizard card known");
        let face_index = wizard.face_index().min(def.faces.len() - 1);
        if wizard.option == Some(CastModeKind::Mutate) {
            return Some(casting::MUTATE_TARGET_REQ);
        }
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
        if wizard.masked {
            return None;
        }
        let def = self
            .state
            .object(wizard.card)
            .and_then(|o| o.card)
            .and_then(|c| self.lookup.card(c.index))
            .expect("wizard card known");
        let face_index = wizard.face_index().min(def.faces.len() - 1);
        let abilities = def.abilities_for_face(face_index);
        let mut req = match wizard.option {
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
        }?;
        if let TargetSpec::CardInFirstTargetsGraveyard(filter) = req.spec
            && let Some(player) = wizard
                .chosen_player
                .or_else(|| wizard.target_players.iter().next())
        {
            req.spec = TargetSpec::CardInPlayersGraveyard(filter, player);
        }
        Some(req)
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

    /// Pays everything and puts the spell on the stack, or, refused, pays
    /// nothing.
    ///
    /// The mana is paid first and all at once, and every refusal after it (a
    /// delve, pitch or sacrifice answer naming an object that is gone, the
    /// card itself gone) used to leave what was paid by then spent while the
    /// cast was reversed. CR 732.1 cancels "any payments already made", so
    /// the game is kept before the first of them and put back on a refusal,
    /// with the spell-rider abilities the mana put on the stack.
    fn finish_cast(&mut self, wizard: &CastWizard) -> Result<(), EngineError> {
        let cost = wizard_paid_total(self.wizard_face(wizard), wizard);
        if self.wizard_pool_can_pay(wizard, cost) && self.ask_escape_payment(wizard)? {
            return Ok(());
        }
        let before = self.state.checkpoint();
        let riders = self.synthetic_fx.clone();
        let cast = self.pay_and_cast(wizard);
        if cast.is_err() {
            self.state.roll_back(before);
            self.synthetic_fx = riders;
        }
        cast
    }

    /// Whether `player` has anything left to make mana with in a payment
    /// window: the same narrowed offer `cast_or_make_miracle_mana` asks
    /// before it opens one, so an X left open for the window is an X a
    /// window will be there to pay.
    pub(super) fn can_make_mana_for_cast(&self, player: PlayerId) -> bool {
        let mut legal = self.compute_legal(player);
        self.narrow_to_mana(&mut legal);
        legal.has_mana_source()
    }

    /// Whether the floating mana is all that pays this cast's total once no
    /// mana is left to make, so the greatest X it pays is known before X is
    /// asked.
    ///
    /// Not when the cast makes its mana after its choices (a payment
    /// window, CR 601.2g), and not when anything besides the pool pays
    /// generic mana: delve, convoke and waterbend (chosen after X, and
    /// convoke may pay a coloured symbol a generic-only probe would miss),
    /// a life-for-colorless permission, or a surcharge per extra target.
    /// Those keep the looser [`Self::x_resource_bound`], and the payment
    /// decides.
    pub(super) fn pool_alone_pays_x(&self, wizard: &CastWizard) -> bool {
        let face = self.wizard_face(wizard);
        !wizard.makes_mana_after_choices()
            && !face.delve
            && !face.convoke
            && !casting::has_improvise(&self.state, face, wizard.player)
            && !face.waterbend
            && face.extra_target_cost == 0
            && self.state.granted_colorless_capacity(wizard.player) == 0
    }

    /// A finite upper bound for a mana X, including ways of paying generic
    /// mana without spending units from the pool. The final payment still
    /// validates fixed colored costs and choices made after the X question.
    fn x_resource_bound(&self, wizard: &CastWizard) -> u32 {
        // Mana abilities may sacrifice permanents, ask choices, or produce
        // a dynamic amount. Counting untapped lands would still exclude
        // legal announcements. The later transactional payment decides
        // whether the announced total can actually be paid.
        if wizard.makes_mana_after_choices() {
            return u32::MAX;
        }
        let face = self.wizard_face(wizard);
        let mut bound =
            casting::spendable_units(&self.state, wizard.player, spend_for(wizard, face))
                .saturating_add(self.state.granted_colorless_capacity(wizard.player))
                .saturating_add(wizard.reduction.map_or(0, |reduction| reduction.remainder));
        for (enabled, count) in [
            (
                face.delve,
                self.state
                    .zones
                    .list(ZoneLocation::Graveyard(wizard.player))
                    .len(),
            ),
            (
                face.convoke || casting::has_improvise(&self.state, face, wizard.player),
                casting::tap_to_pay_sources(&self.state, face, wizard.player).len(),
            ),
            (
                face.waterbend,
                casting::waterbend_sources(&self.state, wizard.player).len(),
            ),
        ] {
            if enabled {
                bound = bound.saturating_add(u32::try_from(count).unwrap_or(u32::MAX));
            }
        }
        bound
    }

    /// Resolves an actual-mana restriction after cost reduction. A reduction
    /// can consume X as well as the fixed generic part; reductions already
    /// consumed by the option price must not be applied twice.
    pub(super) fn restricted_x_payment(
        &self,
        wizard: &CastWizard,
        total: ManaCost,
    ) -> (ManaCost, Option<crate::mana_pay::GenericRestriction>) {
        let face = self.wizard_face(wizard);
        let restricted_colors = face
            .x_mana_colors
            .or_else(|| face.x_mana_color.map(baylee_core::color::ColorSet::of))
            .filter(|_| !wizard.free);
        if restricted_colors.is_none() && face.extra_target_cost == 0 {
            return (total, None);
        }
        let reduction = wizard.reduction.map_or(0, |reduction| reduction.total);
        let restricted = wizard
            .x
            .saturating_sub(reduction)
            .min(total.generic_total());
        (
            total,
            restricted_colors.map(|colors| crate::mana_pay::GenericRestriction {
                types: colors.bits(),
                amount: restricted,
            }),
        )
    }

    /// The X menu and payment ask the same actual-mana constraint.
    fn restricted_x_affordable(&self, wizard: &CastWizard) -> bool {
        let face = self.wizard_face(wizard);
        self.wizard_pool_can_pay(wizard, wizard_total_cost(face, wizard))
    }

    /// Both the announcement and a deferred payment use actual-mana rules.
    fn wizard_pool_can_pay(&self, wizard: &CastWizard, total: ManaCost) -> bool {
        let face = self.wizard_face(wizard);
        let (cost, restriction) = self.restricted_x_payment(wizard, total);
        let cost = self.north_star_payment(wizard, cost);
        let spendable =
            casting::spendable_pool(&self.state, wizard.player, spend_for(wizard, face));
        let pool = spendable
            .as_ref()
            .unwrap_or(&self.state.players[usize::from(wizard.player.get())].mana_pool);
        casting::source_plan(
            &self.state,
            spend_for(wizard, face),
            pool,
            &cost,
            casting::spell_mana_spending(&self.state, wizard.player, wizard.card),
            restriction,
            self.wizard_source_allowance(wizard),
        )
        .is_some()
    }
}

/// The mana cost for the wizard's chosen option, X applied.
/// Removes up to `n` generic mana from a cost (delve/convoke payments).
fn reduce_generic(cost: &ManaCost, n: u32) -> ManaCost {
    cost.with_less_generic(n)
}

///
/// Its Phyrexian symbols are settled once the cast has answered for them
/// (`CastWizard::phyrexian`); before that they stand as printed, which the
/// payment reads as their mana.
fn wizard_cost(wizard: &CastWizard) -> ManaCost {
    let printed = chosen_option_cost(wizard);
    let printed = if wizard.phyrexian.is_empty() {
        printed
    } else {
        printed.with_phyrexian_settled(phyrexian_life_mask(wizard))
    };
    printed.with_x(wizard.x)
}

/// The Phyrexian answers as the mask `ManaCost::with_phyrexian_settled`
/// reads: bit `n` set is the `n`-th symbol paid with 2 life.
fn phyrexian_life_mask(wizard: &CastWizard) -> u32 {
    wizard
        .phyrexian
        .iter()
        .take(32)
        .enumerate()
        .fold(0u32, |mask, (i, life)| mask | (u32::from(*life) << i))
}

/// The life the cast's Phyrexian answers owe: 2 for each paid with life
/// (CR 107.4f).
fn phyrexian_life(wizard: &CastWizard) -> i32 {
    2 * i32::try_from(phyrexian_life_mask(wizard).count_ones()).unwrap_or(i32::MAX)
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
    wizard_total_before_reduction(face, wizard)
        .with_less_generic(wizard.reduction.map_or(0, |reduction| reduction.remainder))
}

/// What the pool pays for the cast at its payment: the total less the
/// generic that delve and convoke already paid.
fn wizard_paid_total(face: &baylee_cards_dsl::FaceDef, wizard: &CastWizard) -> ManaCost {
    wizard_total_cost(face, wizard)
        .with_less_generic((wizard.delve_exiles.len() + wizard.convoke_taps.len()) as u32)
}

fn wizard_total_before_reduction(
    face: &baylee_cards_dsl::FaceDef,
    wizard: &CastWizard,
) -> ManaCost {
    let mut total = wizard_cost(wizard);
    let players = wizard.target_players.len()
        + usize::from(
            wizard
                .chosen_player
                .is_some_and(|p| !wizard.target_players.contains(p)),
        );
    let extra = (wizard.targets.len()
        + wizard.second_targets.len()
        + wizard.second_target_players.len()
        + players)
        .saturating_sub(1);
    total = total.with_more_generic(
        face.extra_target_cost
            .saturating_mul(u32::try_from(extra).unwrap_or(u32::MAX)),
    );
    if wizard.kicked
        && wizard.option != Some(CastModeKind::Kicked)
        && !matches!(
            wizard.announced,
            Some(CastModeKind::EscapeGranted { kicked: true, .. })
        )
    {
        for add in face.additional_costs {
            total = total.combine(&add.mana);
        }
    }
    if let Some(replicate) = face.replicate {
        total = total.combine_n(&replicate, u32::from(wizard.replicated));
    }
    if wizard.behold_paid_mana
        && let Some(behold) = face.behold_or_pay
    {
        total = total.combine(&behold.cost);
    }
    total
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

/// Whether this cast pays mana at all. A free cast pays none of its mana
/// cost (CR 601.2h pays nothing it was not asked for), but an additional
/// cost it chose is still paid (CR 118.9d, 601.2f): a kicker, and the
/// replicate cost however many times it was announced.
const fn pays_mana(wizard: &CastWizard) -> bool {
    !wizard.free || wizard.kicked || wizard.replicated > 0 || wizard.behold_paid_mana
}

/// What the cast's mana pays for, which restricted mana asks (CR 106.6):
/// the one reader of the payment in `finish_cast` and of the replicate
/// question's bound, so the question never offers a count the payment
/// would refuse over a Cavern of Souls' mana.
pub(super) fn spend_for(
    wizard: &CastWizard,
    face: &baylee_cards_dsl::FaceDef,
) -> casting::SpendFor {
    if wizard.kicked {
        let form = match wizard.option {
            Some(CastModeKind::Prototype) => face.prototype.map(casting::SpellForm::Prototype),
            Some(CastModeKind::Disguise) => Some(casting::SpellForm::Disguise),
            _ if wizard.masked => Some(casting::SpellForm::Disguise),
            _ => None,
        };
        return casting::SpendFor::KickedSpell(wizard.card, wizard.face_index(), form);
    }
    if wizard.masked {
        return casting::SpendFor::SpellAs(wizard.card, casting::SpellForm::Disguise);
    }
    match wizard.option {
        Some(CastModeKind::Face(face)) => casting::SpendFor::SpellOnFace(wizard.card, face),
        Some(CastModeKind::Prototype) => casting::SpendFor::SpellAs(
            wizard.card,
            casting::SpellForm::Prototype(face.prototype.expect("prototype option")),
        ),
        Some(CastModeKind::Disguise) => {
            casting::SpendFor::SpellAs(wizard.card, casting::SpellForm::Disguise)
        }
        _ if wizard.face_index() != 0 => {
            casting::SpendFor::SpellOnFace(wizard.card, wizard.face_index())
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
        .find(|o| Some(o.kind) == wizard.announced.or(wizard.option))
        .map_or(ManaCost::ZERO, |o| o.cost)
}

#[cfg(test)]
mod restricted_x_tests;

#[cfg(test)]
mod pool_x_tests;
