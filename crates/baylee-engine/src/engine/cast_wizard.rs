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
use baylee_cards_dsl::{AltCondition, CostPart, TargetReq, TargetSpec};
use baylee_core::mana::ManaCost;

mod announced_costs;
mod commit;
mod cost_parts;
mod division;
mod grants;
mod linked;
mod madness;
mod north_star;
mod options;
mod permissions;
mod pricing;
mod stages;
mod transformed;
pub(crate) mod wave10b;
pub(crate) use cost_parts::{
    CostChoice, additional_card_costs_for_x, fixed_energy_cost, fixed_life_cost,
    mandatory_scales_with_x, next_either_cost, part_scales_with_x, settled_either_mana,
    wizard_cost_parts, wizard_mandatory_parts,
};
pub(crate) use madness::madness_options;
use pricing::FrozenReduction;

/// The most times a repeatable additional cost is announced: replicate's copies
/// and multikicker's payments. Ordinary X choices are bounded by
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
    /// Choosing which permanent to sacrifice for `Emerge` (`CR 702.119c`),
    /// announced before freezing generic reductions and additional costs.
    EmergeSacrifice,
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
    /// Choosing X for a variable kicker cost (`Kicker {X}`, CR 702.33a).
    KickerX,
    /// Buyback yes/no (CR 702.27a). Skipped when the face prints none or
    /// nothing could pay it.
    Buyback,
    /// Bargain yes/no (`CR 702.166a`). Skipped when the face does not print
    /// `bargain` or no artifact, enchantment, or token can pay it.
    Bargain,
    /// Offspring yes/no (`CR 702.175a`). Skipped when the face prints none or
    /// nothing could pay it.
    Offspring,
    /// Casualty yes/no (`CR 702.153a`). Skipped when the face prints none or
    /// no creature controlled by the caster with power `>= N` can pay it.
    Casualty,
    /// How many times to pay the multikicker cost (CR 702.33c). Skipped when
    /// the face prints none or nothing could pay it once.
    Multikicker,
    /// How many times to pay the squad cost (`CR 702.157a`). Skipped when the
    /// face prints none or nothing could pay it once.
    Squad,
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
    /// Optional single creature tap for harmonize (`CR 702.183a`).
    Harmonize,
    /// The additional cost's "sacrifice a creature" (CR 601.2h): which
    /// permanent pays each `Sacrifice` part of
    /// `FaceDef::mandatory_additional_costs`, one question per part.
    Sacrifice,
    /// Ready to pay and cast.
    Done,
    /// Escape's other cards to exile (CR 702.138a), between the pitch and
    /// delve: skipped by every cast but `CastModeKind::Escape`.
    Escape,
    /// Gift yes/no and multiplayer opponent choice (`CR 702.174a`).
    Gift,
    /// Third and subsequent target instances across chosen modes (`CR 700.2c`).
    ExtraTargets,
    /// Settle [`baylee_cards_dsl::CostPart::Either`] branches (`CR 118.8`).
    EitherCost,
    /// Optional hand-card reveals for splice (`CR 702.47a–c`), before targets.
    Splice,
    /// Optional additional tap-two-creatures cost for conspire (`CR 702.78a–b`).
    Conspire,
    /// Choosing `(permanent, counter_kind)` pairs for `RemoveAnyCounter` / `RemoveAnyCounterX`.
    RemoveCounters,
    /// Non-kicker optional additional cost (`FaceDef::optional_cost`, CR 118.8b,
    /// CR 118.9d) when not already settled at `ChooseMode`.
    OptionalCost,
    /// Choosing graveyard cards to exile for `CostPart::CollectEvidence` (`CR 701.59a`).
    CollectEvidence,
    /// Optional generic mana paid by another player for `Assist` (`CR 702.132a`).
    Assist,
}

/// In-flight state for `WizardStage::Assist` (`CR 702.132a`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct AssistWizardState {
    /// Caster has been asked whether another player may assist.
    pub(super) asked_use: bool,
    /// The other player chosen to assist, once selected.
    pub(super) assistant: Option<PlayerId>,
    /// The assistant has finished their `CR 605.3a` mana window.
    pub(super) window_done: bool,
    /// Assistant's window baseline for effect-cast fallback reversal.
    pub(super) helper_start: Option<Box<super::WindowStart>>,
}

/// A spell being cast step by step.
#[derive(Clone, Debug)]
#[allow(clippy::struct_excessive_bools)] // independent casting facts (kicked, behold_paid_mana, free, masked)
#[doc(hidden)]
pub struct CastWizard {
    /// The card being cast.
    pub card: ObjectId,
    /// The casting player.
    pub player: PlayerId,
    /// Chosen cast option.
    pub option: Option<CastModeKind>,
    /// Alternative cost paid independently of a modal choice (CR 118.9a).
    pub alternative_cost: Option<CastModeKind>,
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
    /// Target-dependent cost increases determined once targets are chosen (CR 601.2f).
    pub target_increase: ManaCost,
    /// Target-dependent life cost increases (`Modifier::RaiseCostIfTargets`).
    pub target_life_increase: u32,
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
    /// Third and subsequent target instances (`objects`, `players`).
    pub extra_targets: Vec<(SmallVec<[ObjectId; 2]>, baylee_core::ids::SeatSet)>,
    /// Players chosen as targets, the other half of "any target".
    pub target_players: baylee_core::ids::SeatSet,
    /// Chosen target player, if any.
    pub chosen_player: Option<PlayerId>,
    /// Group controller/owner chosen for the current `SameController` target clause.
    pub target_group_player: Option<PlayerId>,
    /// In-flight per-player target selection for [`baylee_cards_dsl::TargetConstraint::OnePerPlayer`].
    pub one_per_player: Option<Box<OnePerPlayerProgress>>,
    /// Recorded player group bindings for finalized `OnePerPlayer` clauses on this spell.
    pub target_group_bindings: Option<Box<OnePerPlayerBindings>>,
    /// Chosen X.
    pub x: u32,
    /// Whether the kicker was taken.
    pub kicked: bool,
    /// Bitmask of accepted `face.additional_costs[i]` entries on multi-kicker
    /// faces (CR 702.33b, CR 702.33f); zero on single-kicker or unkicked casts.
    pub kicked_mask: u8,
    /// Next `face.additional_costs` index to evaluate in `WizardStage::Kicker`.
    pub kicker_cursor: u8,
    /// How many times the multikicker cost is paid. Above zero the spell is
    /// kicked (CR 702.33d), so `kicked` is true beside it.
    pub multikicked: u8,
    /// What comes off the buyback cost's generic mana, when it was announced
    /// (CR 601.2b); `None` for a spell whose buyback was not. A number and
    /// not the price, because this rides in every `Engine` by value.
    pub buyback: Option<u32>,
    /// Whether the spell's bargain additional cost was announced (`CR 702.166a`).
    pub bargained: bool,
    /// Whether `FaceDef::optional_cost` was announced (`CR 118.8b`).
    pub optional_cost_paid: bool,
    /// Whether the caster has already decided `FaceDef::optional_cost` on this cast.
    pub optional_cost_decided: bool,
    /// Chosen graveyard card groups for `CostPart::CollectEvidence` (`CR 701.59a`).
    pub evidence_exiles: Vec<Vec<ObjectId>>,
    /// Whether the spell's offspring additional cost was announced (`CR 702.175a`).
    pub offspring: bool,
    /// Whether the spell's casualty additional cost was announced (`CR 702.153a`).
    pub casualty: bool,
    /// Minimum power threshold for casualty (printed or stack-granted).
    pub casualty_power: Option<u8>,
    /// Granted replicate mana cost from a stack-reaching continuous effect.
    pub granted_replicate: Option<ManaCost>,
    /// Whether the spell's gift additional cost was promised (`CR 702.174a`).
    pub gift_promised: bool,
    /// The opponent chosen to receive the promised gift (`CR 702.174a`).
    pub gift_recipient: Option<PlayerId>,
    /// How many times the spell's squad additional cost was announced (`CR 702.157a`).
    pub squad: u8,
    /// The permanent chosen to sacrifice for `Emerge` (`CR 702.119c`).
    pub emerge_sacrifice: Option<ObjectId>,
    /// How many times the replicate cost is paid.
    pub replicated: u8,
    /// How many conspire instances were accepted (`CR 702.78a–b`).
    pub conspired: u8,
    /// How many conspire instances have already been asked in `WizardStage::Conspire`.
    pub conspire_cursor: u8,
    /// Hand cards chosen to splice onto this spell in order (`CR 702.47a–c`).
    pub spliced: SmallVec<
        [(
            ObjectId,
            baylee_core::ids::CardIndex,
            u8,
            baylee_cards_dsl::Cost,
        ); 2],
    >,
    /// Non-mana parts of `AbilityDef::Awaken` when `option == Some(CastModeKind::Awaken)`.
    pub awaken_parts: &'static [CostPart],
    /// Cards chosen for pitch (exile-from-hand).
    pub pitch: SmallVec<[ObjectId; 2]>,
    /// The creature returned for beam me up or sneak, chosen without targeting.
    pub returned: Option<ObjectId>,
    /// Non-mana parts of a sneak cost beyond returning an unblocked attacker.
    pub sneak_parts: &'static [CostPart],
    /// The permanent or card in hand chosen to behold (CR 701.60a).
    pub beheld: Option<ObjectId>,
    /// Whether the caster is paying `behold_or_pay`'s mana cost instead of
    /// beholding.
    pub behold_paid_mana: bool,
    /// Number of `CostPart::Either` parts settled on this cast (`CR 118.8`).
    pub either_decided: u8,
    /// Bitmask of settled `CostPart::Either` branches (`1` = first, `0` = second).
    pub either_first_mask: u8,
    /// Whether any chosen `CostPart::Either` branch adds mana.
    pub either_pays_mana: bool,
    /// Cards chosen to delve (exile-from-graveyard, {1} each).
    pub delve_exiles: SmallVec<[ObjectId; 8]>,
    /// The other cards escape's cost exiles from the graveyard. They pay no
    /// mana, which is what keeps them apart from `delve_exiles`.
    pub escape_exiles: SmallVec<[ObjectId; 8]>,
    /// Permanents chosen to tap for convoke or a waterbend ({1} each).
    pub convoke_taps: SmallVec<[ObjectId; 8]>,
    /// Creature tapped to reduce generic cost for Harmonize (`CR 702.183a`).
    pub harmonize_tap: Option<ObjectId>,
    /// Objects chosen for asking cost parts, one answer per object in cost
    /// order. A `TapN` part consumes N distinct answers.
    pub cost_objects: SmallVec<[ObjectId; 4]>,
    /// Chosen `(permanent, counter_kind)` pairs for `RemoveAnyCounter` / `RemoveAnyCounterX`.
    pub counter_removals: SmallVec<[(ObjectId, baylee_cards_dsl::CounterKind); 4]>,
    /// Permanent chosen for the next counter removal while awaiting counter-kind disambiguation.
    pub pending_counter_object: Option<ObjectId>,
    /// Explicit mana cost imposed by an effect (`PlayCostMode::Alternative`).
    pub override_mana_cost: Option<ManaCost>,
    /// Index into `options` selected by `ChooseMode` (or `0` when single-option).
    pub chosen_option_index: Option<u16>,
    /// Continuous `MayCastWithAltCost` routes and chosen grant, allocated only
    /// when a grant applies so `CastWizard` stays compact on the stack.
    pub alt_cast: Option<Box<AltCastWizardState>>,
    /// Whether an effect casting this spell exiles it as it resolves instead of putting it into a graveyard.
    pub exile_instead_of_graveyard: bool,
    /// How each Phyrexian symbol of the chosen option's cost is paid, in the
    /// cost's own order: `true` is 2 life, `false` its mana (CR 107.4f),
    /// announced with the other costs (CR 601.2b) and charged as the cost
    /// is paid (CR 601.2h). Asked at the head of the `XValue` stage, so
    /// the X bound reads the settled cost.
    pub phyrexian: SmallVec<[bool; 4]>,
    /// Generic mana already paid by an assisting player (`CR 702.132a`).
    pub assist_paid: u32,
    /// In-flight assist prompt/window state (`CR 702.132a`).
    pub(super) assist: Option<Box<AssistWizardState>>,
    /// Current stage.
    pub stage: WizardStage,
    /// Options computed at start (kept for the Done stage).
    pub options: Vec<CastModeDesc>,
    /// Whether this cast is free (rebound, suspend finish).
    pub free: bool,
    /// Whether this free cast was started by Suspend's last-time-counter
    /// trigger (CR 702.62a), granting creature spells haste until lost control.
    pub from_suspend: bool,
    /// This nested cast uses the costless, nameless 2/2 face.
    pub masked: bool,
    /// This cast revealed a privately visible exiled card on announcement.
    /// It must be concealed again if that announcement is not completed.
    pub revealed_exile: bool,
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

/// Boxed `MayCastWithAltCost` state on [`CastWizard`], kept out of line so
/// `CastWizard` does not bloat `Engine` / `Checkpoint` stack frames on
/// ordinary casts.
#[derive(Debug, Clone)]
pub(crate) struct AltCastWizardState {
    /// Continuous `MayCastWithAltCost` routes indexed by position in `options`.
    pub routes: Vec<(usize, casting::AltCastGrantRoute)>,
    /// Active `MayCastWithAltCost` route chosen for this cast, if any.
    pub selected: Option<casting::AltCastGrantRoute>,
}

/// Boxed per-player targeting progress for
/// [`baylee_cards_dsl::TargetConstraint::OnePerPlayer`] on [`CastWizard`].
#[derive(Debug, Clone, Hash)]
pub(crate) struct OnePerPlayerProgress {
    /// Seats remaining to prompt in turn order.
    pub remaining: Vec<PlayerId>,
    /// Seat currently being prompted.
    pub current: Option<PlayerId>,
    /// Targets gathered so far across prompted seats.
    pub gathered: SmallVec<[ObjectId; 4]>,
    /// Player group bound to each target in `gathered`.
    pub groups: SmallVec<[PlayerId; 4]>,
    /// Minimum targets required for each seat (`0` or `1`).
    pub min_per_player: u32,
}

/// Recorded player groups for finalized `OnePerPlayer` target clauses on a spell.
#[derive(Debug, Clone, Default, Hash)]
pub(crate) struct OnePerPlayerBindings {
    pub first: SmallVec<[PlayerId; 4]>,
    pub second: SmallVec<[PlayerId; 4]>,
    pub extra: Vec<(usize, SmallVec<[PlayerId; 4]>)>,
}

impl CastWizard {
    pub(in crate::engine) fn selected_alt_grant(&self) -> Option<casting::AltCastGrantRoute> {
        self.alt_cast.as_deref().and_then(|s| s.selected)
    }
    pub(super) fn clause_first_targets(
        &self,
        clause_idx: usize,
    ) -> (Option<ObjectId>, Option<PlayerId>, &[ObjectId]) {
        if clause_idx == 0 {
            return (None, None, &[]);
        }
        match clause_idx - 1 {
            0 => (
                self.targets.first().copied(),
                self.chosen_player
                    .or_else(|| self.target_players.iter().next()),
                self.targets.as_slice(),
            ),
            1 => (
                self.second_targets.first().copied(),
                self.second_target_players.iter().next(),
                self.second_targets.as_slice(),
            ),
            prev => {
                let extra_idx = prev - 2;
                let first_obj = self
                    .extra_targets
                    .get(extra_idx)
                    .and_then(|(objs, _)| objs.first().copied());
                let first_player = self
                    .extra_targets
                    .get(extra_idx)
                    .and_then(|(_, players)| players.iter().next());
                let slice = self
                    .extra_targets
                    .get(extra_idx)
                    .map_or(&[][..], |(objs, _)| objs.as_slice());
                (first_obj, first_player, slice)
            }
        }
    }

    /// A cast nothing has been answered for yet, standing at `stage` with
    /// `options` to choose from: paid out of the pool at its full price by
    /// nobody's effect. Each way of starting a cast says what it changes
    /// beside this, so a new answer the wizard keeps is written once.
    pub(in crate::engine) fn new(
        card: ObjectId,
        player: PlayerId,
        stage: WizardStage,
        options: Vec<CastModeDesc>,
    ) -> Self {
        Self {
            card,
            player,
            option: None,
            alternative_cost: None,
            announced: None,
            forced_face: None,
            north_star: None,
            permission: None,
            permissions: Vec::new(),
            reduction: None,
            target_increase: ManaCost::ZERO,
            target_life_increase: 0,
            target_references: crate::sources::TargetReferences::default(),
            division: Vec::new(),
            targets: SmallVec::new(),
            second_targets: SmallVec::new(),
            second_target_players: baylee_core::ids::SeatSet::new(),
            extra_targets: Vec::new(),
            target_players: baylee_core::ids::SeatSet::new(),
            chosen_player: None,
            target_group_player: None,
            one_per_player: None,
            target_group_bindings: None,
            x: 0,
            kicked: false,
            kicked_mask: 0,
            kicker_cursor: 0,
            multikicked: 0,
            buyback: None,
            bargained: false,
            optional_cost_paid: false,
            optional_cost_decided: false,
            evidence_exiles: Vec::new(),
            offspring: false,
            casualty: false,
            casualty_power: None,
            granted_replicate: None,
            gift_promised: false,
            gift_recipient: None,
            squad: 0,
            emerge_sacrifice: None,
            replicated: 0,
            conspired: 0,
            conspire_cursor: 0,
            spliced: SmallVec::new(),
            awaken_parts: &[],
            pitch: SmallVec::new(),
            returned: None,
            sneak_parts: &[],
            beheld: None,
            behold_paid_mana: false,
            either_decided: 0,
            either_first_mask: 0,
            either_pays_mana: false,
            delve_exiles: SmallVec::new(),
            escape_exiles: SmallVec::new(),
            convoke_taps: SmallVec::new(),
            harmonize_tap: None,
            cost_objects: SmallVec::new(),
            counter_removals: SmallVec::new(),
            pending_counter_object: None,
            override_mana_cost: None,
            chosen_option_index: None,
            alt_cast: None,
            exile_instead_of_graveyard: false,
            phyrexian: SmallVec::new(),
            assist_paid: 0,
            assist: None,
            stage,
            options,
            free: false,
            from_suspend: false,
            masked: false,
            revealed_exile: false,
            by_effect: None,
            started: None,
            paying: casting::Paying::FromThePool,
            window_start: None,
        }
    }

    pub(super) fn face_index(&self) -> usize {
        if let Some(face) = self.forced_face {
            return usize::from(face);
        }
        if let Some(route) = self.selected_alt_grant() {
            return route.face;
        }
        match self.announced.or(self.option) {
            Some(
                CastModeKind::Face(face)
                | CastModeKind::EscapeGranted { face, .. }
                | CastModeKind::LibraryTopLife { face, .. },
            ) => face,
            _ => 0,
        }
    }

    pub(in crate::engine) fn apply_selected_option(
        &mut self,
        index: usize,
        def: Option<&'static baylee_cards_dsl::CardDef>,
    ) {
        self.chosen_option_index = u16::try_from(index).ok();
        let Some(desc) = self.options.get(index) else {
            return;
        };
        let option = desc.kind;
        self.option = Some(option);
        self.kicked = option == CastModeKind::Kicked;
        self.optional_cost_paid = option == CastModeKind::OptionalCost;
        let matched_route = self
            .alt_cast
            .as_deref()
            .and_then(|s| s.routes.iter().find(|(i, _)| *i == index))
            .map(|(_, route)| *route);
        if matched_route.is_none()
            && !self.free
            && self.override_mana_cost.is_none()
            && self.by_effect.is_none()
            && matches!(option, CastModeKind::Normal | CastModeKind::OptionalCost)
        {
            self.optional_cost_decided = true;
        }
        if let Some(route) = matched_route {
            if let Some(state) = self.alt_cast.as_deref_mut() {
                state.selected = Some(route);
            }
            if route.life.is_none() {
                self.announced = Some(CastModeKind::Permission(route.source));
                self.override_mana_cost = route.base_mana;
                self.kicked = route.kicked;
                if route.prototype {
                    self.option = Some(CastModeKind::Prototype);
                } else if let Some(def) = def {
                    self.option = Some(casting::ordinary_face_kind(
                        def,
                        route.face,
                        route.modes,
                        route.kicked,
                    ));
                }
            }
        }
        if self.option == Some(CastModeKind::Awaken)
            && let Some((_, cost)) = def.and_then(|d| wave10b::face_awaken(d, self.face_index()))
        {
            self.awaken_parts = cost.parts;
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
    #[allow(clippy::too_many_lines)]
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
        // Only where there is one: a record already written holds this number
        // without them, and it must go on replaying.
        if self.multikicked != 0 || self.buyback.is_some() {
            hash = hash
                .wrapping_mul(31)
                .wrapping_add(crate::state::structural_fingerprint(&(
                    self.multikicked,
                    self.buyback,
                )));
        }
        if self.kicked_mask != 0 {
            hash = hash
                .wrapping_mul(31)
                .wrapping_add(u64::from(self.kicked_mask));
        }
        if self.bargained || self.emerge_sacrifice.is_some() || self.offspring || self.squad > 0 {
            hash = hash
                .wrapping_mul(31)
                .wrapping_add(crate::state::structural_fingerprint(&(
                    self.bargained,
                    self.emerge_sacrifice,
                    self.offspring,
                    self.squad,
                )));
        }
        if self.casualty {
            hash = hash.wrapping_mul(31).wrapping_add(162);
        }
        if self.gift_promised || self.gift_recipient.is_some() {
            hash = hash
                .wrapping_mul(31)
                .wrapping_add(crate::state::structural_fingerprint(&(
                    182u8,
                    self.gift_promised,
                    self.gift_recipient,
                )));
        }
        if self.either_decided != 0 {
            hash = hash
                .wrapping_mul(31)
                .wrapping_add(crate::state::structural_fingerprint(&(
                    191u8,
                    self.either_decided,
                    self.either_first_mask,
                    self.either_pays_mana,
                )));
        }
        // A separate alternative cost changes the meaning of modal answers.
        // Keep the legacy fingerprint byte-for-byte when no such route exists.
        if self.alternative_cost.is_some() {
            hash = hash
                .wrapping_mul(31)
                .wrapping_add(crate::state::structural_fingerprint(&(
                    self.alternative_cost,
                    self.option,
                )));
        }
        if !self.extra_targets.is_empty() {
            hash = hash
                .wrapping_mul(31)
                .wrapping_add(crate::state::structural_fingerprint(&self.extra_targets));
        }
        if self.conspired > 0 || !self.spliced.is_empty() {
            hash = hash
                .wrapping_mul(31)
                .wrapping_add(crate::state::structural_fingerprint(&(
                    261u16,
                    self.conspired,
                    &self.spliced,
                )));
        }
        if !self.counter_removals.is_empty()
            || self.pending_counter_object.is_some()
            || self.override_mana_cost.is_some()
            || self.exile_instead_of_graveyard
            || self.selected_alt_grant().is_some()
            || self.target_group_player.is_some()
        {
            hash = hash
                .wrapping_mul(31)
                .wrapping_add(crate::state::structural_fingerprint(&(
                    277u16,
                    &self.counter_removals,
                    self.pending_counter_object,
                    self.override_mana_cost,
                    self.exile_instead_of_graveyard,
                    self.selected_alt_grant().map(|r| r.effect_id),
                    self.target_group_player,
                )));
        }
        if self.one_per_player.is_some() || self.target_group_bindings.is_some() {
            hash = hash
                .wrapping_mul(31)
                .wrapping_add(crate::state::structural_fingerprint(&(
                    311u16,
                    &self.one_per_player,
                    &self.target_group_bindings,
                )));
        }
        if self.optional_cost_paid || !self.evidence_exiles.is_empty() {
            hash = hash
                .wrapping_mul(31)
                .wrapping_add(crate::state::structural_fingerprint(&(
                    283u16,
                    self.optional_cost_paid,
                    &self.evidence_exiles,
                )));
        }
        if self.harmonize_tap.is_some()
            || self.casualty_power.is_some()
            || self.granted_replicate.is_some()
        {
            hash = hash
                .wrapping_mul(31)
                .wrapping_add(crate::state::structural_fingerprint(&(
                    501_u16,
                    self.harmonize_tap,
                    self.casualty_power,
                    self.granted_replicate,
                )));
        }
        if self.assist_paid > 0 || self.assist.is_some() {
            hash = hash
                .wrapping_mul(31)
                .wrapping_add(crate::state::structural_fingerprint(&(
                    506u16,
                    self.assist_paid,
                    self.assist.as_ref().map(|a| {
                        (
                            a.asked_use,
                            a.assistant,
                            a.window_done,
                            a.helper_start.as_ref().map(|s| s.fingerprint()),
                        )
                    }),
                )));
        }
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
        hash = Self::hash_object_slices(
            hash,
            &[
                self.targets.as_slice(),
                self.second_targets.as_slice(),
                self.pitch.as_slice(),
                self.delve_exiles.as_slice(),
                self.escape_exiles.as_slice(),
                self.convoke_taps.as_slice(),
                self.cost_objects.as_slice(),
            ],
        );
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

    fn hash_object_slices(mut hash: u64, slices: &[&[ObjectId]]) -> u64 {
        for targets in slices {
            hash = hash.wrapping_mul(31).wrapping_add(targets.len() as u64);
            for target in *targets {
                hash = hash
                    .wrapping_mul(31)
                    .wrapping_add(u64::from(target.slot()))
                    .wrapping_mul(31)
                    .wrapping_add(u64::from(target.generation()));
            }
        }
        hash
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

/// The non-mana parts the wizard pays for an alternative casting cost.
/// Pool-wide coverage is enforced by
/// `offer_tests::no_spell_cost_list_carries_a_part_its_payment_walks_past`.
pub(crate) fn paid_as_an_alternative_cost(part: &CostPart) -> bool {
    match part {
        CostPart::PayLife(_)
        | CostPart::DealDamageToYou(_)
        | CostPart::ExileFromHand(_)
        | CostPart::Sacrifice(_)
        | CostPart::ReturnToHand(_)
        | CostPart::TapN { .. }
        | CostPart::RemoveAnyCounter { .. }
        | CostPart::PutCounterOnOther { .. }
        | CostPart::Forage
        | CostPart::Behold(_)
        | CostPart::BeholdAndExile(_)
        | CostPart::RevealFromHand(_)
        | CostPart::CollectEvidence(_)
        | CostPart::Waterbend(_) => true,
        CostPart::Either(a, b) => {
            a.parts.iter().all(paid_as_an_alternative_cost)
                && b.parts.iter().all(paid_as_an_alternative_cost)
        }
        _ => false,
    }
}

/// The non-mana parts the wizard pays for mandatory additional costs.
pub(crate) fn paid_as_a_mandatory_additional_cost(part: &CostPart) -> bool {
    match part {
        CostPart::PayLifeX
        | CostPart::PayLife(_)
        | CostPart::DealDamageToYou(_)
        | CostPart::PayEnergy(_)
        | CostPart::PayEnergyX
        | CostPart::Sacrifice(_)
        | CostPart::SacrificeX(_)
        | CostPart::SacrificeAll(_)
        | CostPart::ReturnToHand(_)
        | CostPart::ExilePermanent(_)
        | CostPart::ExileFromHand(_)
        | CostPart::ExileFromGraveyard(_)
        | CostPart::ExileFromGraveyardX(_)
        | CostPart::ExileTopOfLibrary(_)
        | CostPart::Discard(_)
        | CostPart::DiscardX(_)
        | CostPart::DiscardRandom
        | CostPart::DiscardHand
        | CostPart::DiscardLastDrawnThisTurn
        | CostPart::TapOther(_)
        | CostPart::TapX(_)
        | CostPart::TapN { .. }
        | CostPart::RemoveAnyCounter { .. }
        | CostPart::RemoveAnyCounterX { .. }
        | CostPart::PutCounterOnOther { .. }
        | CostPart::Forage
        | CostPart::Behold(_)
        | CostPart::BeholdAndExile(_)
        | CostPart::RevealFromHand(_)
        | CostPart::RevealFromHandX(_)
        | CostPart::CollectEvidence(_)
        | CostPart::Waterbend(_)
        | CostPart::WaterbendX => true,
        CostPart::Either(a, b) => {
            a.parts.iter().all(paid_as_a_mandatory_additional_cost)
                && b.parts.iter().all(paid_as_a_mandatory_additional_cost)
        }
        _ => false,
    }
}

/// The non-mana parts the wizard pays for a buyback cost, the last of them
/// after everything else because it is random (CR 601.2h).
pub(crate) fn paid_as_a_buyback_cost(part: &CostPart) -> bool {
    match part {
        CostPart::PayLife(_)
        | CostPart::DealDamageToYou(_)
        | CostPart::Sacrifice(_)
        | CostPart::Discard(_)
        | CostPart::DiscardRandom
        | CostPart::RemoveAnyCounter { .. }
        | CostPart::PutCounterOnOther { .. }
        | CostPart::Behold(_)
        | CostPart::BeholdAndExile(_)
        | CostPart::RevealFromHand(_)
        | CostPart::Forage => true,
        CostPart::Either(a, b) => {
            a.parts.iter().all(paid_as_a_buyback_cost) && b.parts.iter().all(paid_as_a_buyback_cost)
        }
        _ => false,
    }
}

impl<L: CardLookup, D: crate::engine::LoopDetection> Engine<L, D> {
    pub(crate) fn is_plot_only_modal(&self, player: PlayerId, card: ObjectId) -> bool {
        let Some(obj) = self.state.object(card) else {
            return false;
        };
        let Some(def) = obj.card.and_then(|c| self.lookup.card(c.index)) else {
            return false;
        };
        casting::modes_are_the_only_way(def, 0)
            && casting::plot_permission(&self.state, player, card)
            && !self.commanded_card(card)
            && (def.faces[0].foretell.is_none()
                || !casting::foretell_permission(&self.state, player, card))
            && casting::play_permission(&self.state, player, card).is_none()
    }

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
        let (options, alt_cost_routes) =
            self.cast_options_with_routes(player, card, pay_in_window)?;
        let plot_modal = self.is_plot_only_modal(player, card);
        let alt_cast = (!alt_cost_routes.is_empty()).then(|| {
            Box::new(AltCastWizardState {
                routes: alt_cost_routes,
                selected: None,
            })
        });
        let mut wizard = Box::new(CastWizard {
            alternative_cost: plot_modal.then_some(CastModeKind::Plot),
            alt_cast,
            by_effect: self.commanded_card(card).then_some(EffectCast::Plain),
            paying: if pay_in_window {
                casting::Paying::DuringTheCast
            } else {
                casting::Paying::FromThePool
            },
            started: (!self.commanded_card(card)).then(|| self.cast_start()),
            ..CastWizard::new(card, player, WizardStage::ChooseMode, options)
        });
        if wizard.options.len() == 1 {
            let def = self
                .state
                .object(card)
                .and_then(|o| o.card)
                .and_then(|c| self.lookup.card(c.index));
            wizard.apply_selected_option(0, def);
            wizard.stage = WizardStage::Permission;
        }
        // The offer and option validation used a private projection. The
        // physical card becomes public only now, when its cast is announced.
        wizard.revealed_exile = self.state.reveal_exiled_card(card);
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
        let options = vec![CastModeDesc {
            index: 0,
            kind: CastModeKind::Normal,
            cost,
        }];
        self.cast_wizard = Some(Box::new(CastWizard {
            option: Some(CastModeKind::Normal),
            free: true,
            masked: true,
            by_effect: Some(EffectCast::Plain),
            ..CastWizard::new(card, player, WizardStage::Done, options)
        }));
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

    /// Begins the cast madness's trigger offered, using the ordinary casting
    /// wizard for X, targets, additional costs and the mana opportunity. Its
    /// alternative cost is the card's madness cost, not its mana cost.
    pub(crate) fn start_madness_cast(
        &mut self,
        player: PlayerId,
        card: ObjectId,
        version: u32,
    ) -> Result<(), EngineError> {
        let options = madness::madness_options(&self.state, &self.lookup, player, card, version)
            .ok_or(EngineError::IllegalAction("no way to cast this spell"))?;
        let single = (options.len() == 1).then(|| options[0].kind);
        let stage = if single.is_some() {
            WizardStage::XValue
        } else {
            WizardStage::ChooseMode
        };
        let wizard = Box::new(CastWizard {
            option: single,
            alternative_cost: Some(CastModeKind::Madness),
            by_effect: Some(EffectCast::Plain),
            window_start: Some(Box::new(self.window_start(player))),
            ..CastWizard::new(card, player, stage, options)
        });
        self.cast_wizard = Some(wizard);
        self.advance_cast_wizard()
    }

    /// Whether `player` could choose every target a miracle cast of `card`
    /// requires, which is what decides whether the miracle is offered.
    ///
    /// Choosing the targets is a step of casting (CR 601.2c), and a spell
    /// that cannot take it cannot be cast: the attempt is illegal and is
    /// reversed (CR 601.2, CR 734.1). A "yes" to such a miracle can only
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
            let players = eval::target_player_options_from(
                &self.state,
                &req.spec,
                player,
                eval::live_context(&self.state, card),
            );
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
        let second_ok = self.wizard_second_target_req(&wizard).is_none_or(|req| {
            let players = eval::target_player_options_from(
                &self.state,
                &req.spec,
                player,
                eval::live_context(&self.state, card),
            );
            self.wizard_target_options(&wizard, &req.spec).len() + players.len()
                >= usize::from(req.min)
        });
        let third_ok = self.wizard_third_target_req(&wizard).is_none_or(|req| {
            let players = eval::target_player_options_from(
                &self.state,
                &req.spec,
                player,
                eval::live_context(&self.state, card),
            );
            self.wizard_target_options(&wizard, &req.spec).len() + players.len()
                >= usize::from(req.min)
        });
        if !second_ok || !third_ok {
            return false;
        }
        let (_, _, later) = self.wizard_prior_and_later_pools(&wizard, 0);
        if later.iter().any(|g| g.needed > 0 && g.distinct_from_prior)
            && let Some(first_req) = self.wizard_target_req(&wizard)
        {
            let mut all_pools = Vec::with_capacity(1 + later.len());
            all_pools.push(self.wizard_clause_distinct_pool(&wizard, first_req));
            all_pools.extend(later);
            if !eval::can_complete_target_groups(&[], baylee_core::ids::SeatSet::new(), &all_pools)
            {
                return false;
            }
        }
        true
    }

    /// The wizard a miracle cast of `card` starts with, or `None` when the
    /// card prints no miracle or `player` may not cast it now.
    fn miracle_wizard(&self, player: PlayerId, card: ObjectId) -> Option<Box<CastWizard>> {
        let object = self.state.object(card)?;
        let def = object.card.and_then(|c| self.lookup.card(c.index))?;
        let cost = def.faces[0].miracle?;
        // CR 601.3: a player who may not begin to cast a spell (Conduit of
        // Worlds), or who is forbidden from casting outside sorcery timing
        // while the miracle trigger is on the stack (Teferi, Time Raveler's
        // CR 101.2 restriction beats CR 702.94a's permission), has no miracle
        // cast either, so it is not offered.
        if !casting::may_begin_casting(&self.state, player)
            || !casting::timing_allows(
                &self.state,
                player,
                baylee_core::types::TypeSet::INSTANT,
                baylee_cards_dsl::KeywordSet::EMPTY,
            )
            || !casting::spell_condition_allows(&self.state, player, card, def, 0)
            || casting::face_is_forbidden(&self.state, player, object, def, 0)
            || casting::additional_payment_capacity_for(
                &self.state,
                player,
                card,
                &def.faces[0],
                Some(CastModeKind::Miracle),
                &[],
            )
            .is_none()
        {
            return None;
        }
        let options = vec![CastModeDesc {
            index: 0,
            kind: CastModeKind::Miracle,
            cost: self.front_spell_price(player, card, cost),
        }];
        Some(Box::new(CastWizard {
            option: Some(CastModeKind::Miracle),
            // A miracle cost is paid "rather than its mana cost" (CR 702.94a),
            // which makes it an alternative cost, and an alternative cost
            // with an {X} in it announces X like any other (CR 107.3a) —
            // Entreat the Dead's `{X}{B}{B}` was cast for X = 0 and returned
            // nobody. `XValue` falls through to `Targets` when there is none.
            ..CastWizard::new(card, player, WizardStage::XValue, options)
        }))
    }

    /// Price an alternative/front-face cost through the same increases and
    /// reductions as a normal cast. Waiving mana cost does not waive a tax.
    fn front_spell_price(&self, player: PlayerId, card: ObjectId, cost: ManaCost) -> ManaCost {
        madness::front_spell_price(&self.state, &self.lookup, player, card, cost)
    }

    fn free_normal_options(&self, player: PlayerId, card: ObjectId) -> Vec<CastModeDesc> {
        let base = self
            .state
            .object(card)
            .and_then(|o| o.card)
            .and_then(|c| self.lookup.card(c.index))
            .map_or(ManaCost::ZERO, |def| {
                casting::face_additional_mana_floor(&self.state, player, card, &def.faces[0])
            });
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
        madness::alternative_cost_affordable(&self.state, &self.lookup, player, card, cost)
    }

    /// Starts a free cast (rebound at upkeep, suspend finish, a discovered
    /// card): no payment, but targets and other choices still run through
    /// the wizard.
    pub(crate) fn start_free_cast(
        &mut self,
        player: PlayerId,
        card: ObjectId,
    ) -> Result<(), EngineError> {
        self.start_exile_free_cast(player, card, false)
    }

    /// Starts a free cast from exile, marking whether it comes from Suspend's
    /// last-time-counter trigger (`from_suspend`, CR 702.62a).
    pub(crate) fn start_exile_free_cast(
        &mut self,
        player: PlayerId,
        card: ObjectId,
        from_suspend: bool,
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
        // "Can't be cast" binds an effect's cast too (CR 601.3), or cascade
        // would cast what Meddling Mage named.
        if self.state.object(card).is_some_and(|object| {
            let front = casting::split_front(&self.state, object);
            casting::cast_is_forbidden(&self.state, player, front.as_ref().unwrap_or(object))
        }) {
            return Err(EngineError::IllegalAction("casting prohibited"));
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
        self.start_free_cast_options_ex(player, card, options, false, None, from_suspend)
    }

    fn start_free_cast_options(
        &mut self,
        player: PlayerId,
        card: ObjectId,
        options: Vec<CastModeDesc>,
        by_effect: bool,
        forced_face: Option<u8>,
    ) -> Result<(), EngineError> {
        self.start_free_cast_options_ex(player, card, options, by_effect, forced_face, false)
    }

    fn start_free_cast_options_ex(
        &mut self,
        player: PlayerId,
        card: ObjectId,
        options: Vec<CastModeDesc>,
        by_effect: bool,
        forced_face: Option<u8>,
        from_suspend: bool,
    ) -> Result<(), EngineError> {
        let (option, stage) = match options.as_slice() {
            [] => return Err(EngineError::IllegalAction("no legal free play")),
            [only] => (Some(only.kind), WizardStage::XValue),
            _ => (None, WizardStage::ChooseMode),
        };
        // `XValue` fixes a waived mana-cost X at zero, but still asks about
        // independent additional costs such as Toxic Deluge's X life.
        let wizard = Box::new(CastWizard {
            option,
            forced_face,
            free: true,
            from_suspend,
            by_effect: by_effect.then_some(EffectCast::Plain),
            ..CastWizard::new(card, player, stage, options)
        });
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
        // `XValue` asks nothing of a free wizard (CR 107.3b); from there on it
        // is every other cast.
        let stage = if single {
            WizardStage::XValue
        } else {
            WizardStage::ChooseMode
        };
        let wizard = Box::new(CastWizard {
            option: if single { option } else { None },
            free: true,
            ..CastWizard::new(card, player, stage, options)
        });
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
    /// gives back what both made if it closes without the cast (CR 734.1).
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
            .map(|o| {
                // A split card's left half, where the card is both halves.
                casting::split_front(&self.state, o)
                    .map_or(o.characteristics().mana_cost, |front| {
                        front.characteristics().mana_cost
                    })
            })
            .ok_or(EngineError::IllegalAction("the card has gone"))?;
        if !casting::may_begin_casting(&self.state, player) {
            return Err(EngineError::IllegalAction("no more spells this turn"));
        }
        let options = vec![CastModeDesc {
            index: 0,
            kind: CastModeKind::Normal,
            cost: self.front_spell_price(player, card, cost),
        }];
        let wizard = Box::new(CastWizard {
            option: Some(CastModeKind::Normal),
            by_effect: Some(if any_mana {
                EffectCast::AnyMana
            } else if then_no_more_spells {
                EffectCast::ThenNoMoreSpells
            } else {
                EffectCast::Plain
            }),
            window_start: Some(opened),
            ..CastWizard::new(card, player, WizardStage::XValue, options)
        });
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
        self.modes_with_base(player, card, def, ManaCost::ZERO, make_mana)
    }

    /// Modal choices over one alternative base cost. A mode's own alternative
    /// cost cannot combine with that base; its additional costs still can.
    fn modes_with_base(
        &self,
        player: PlayerId,
        card: ObjectId,
        def: &baylee_cards_dsl::CardDef,
        base: ManaCost,
        make_mana: bool,
    ) -> Vec<CastModeDesc> {
        madness::modes_with_base(
            &self.state,
            &self.lookup,
            player,
            card,
            def,
            base,
            make_mana,
        )
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
    #[doc(hidden)]
    pub fn free_cast_possible(&self, player: PlayerId, card: ObjectId) -> bool {
        let Some(obj) = self.state.object(card) else {
            return false;
        };
        let Some(def) = obj.card.and_then(|c| self.lookup.card(c.index)) else {
            return false;
        };
        let split_front = casting::split_front(&self.state, obj);
        let front = split_front.as_ref().unwrap_or(obj);
        if front
            .characteristics()
            .types
            .contains(baylee_core::types::TypeSet::LAND)
            || casting::cast_is_forbidden(&self.state, player, front)
        {
            return false;
        }
        if casting::additional_payment_capacity(&self.state, player, card, &def.faces[0], &[])
            .is_none()
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
    pub(super) fn replicate_bound(
        &self,
        wizard: &CastWizard,
        face: &baylee_cards_dsl::FaceDef,
        cost: &ManaCost,
    ) -> u32 {
        let player = wizard.player;
        let face_idx = wizard.face_index();
        let delve = casting::has_delve(&self.state, wizard.card, face, player, face_idx);
        let convoke = casting::has_convoke(&self.state, wizard.card, face, player, face_idx);
        let improvise =
            casting::has_improvise_for(&self.state, wizard.card, face, player, face_idx);
        let graveyard = self.state.zones.list(ZoneLocation::Graveyard(player)).len();
        let help = [
            (delve, graveyard),
            (
                convoke || improvise,
                casting::tap_to_pay_sources_for(&self.state, convoke, improvise, player).len(),
            ),
            (
                face.waterbend && (wizard.kicked || wizard.optional_cost_paid),
                casting::waterbend_sources(&self.state, player).len(),
            ),
            (
                wizard.is_harmonize(),
                usize::try_from(casting::harmonize_max_power(&self.state, player))
                    .unwrap_or(usize::MAX),
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
        if result.is_err()
            && let Some(wizard) = self.cast_wizard.take()
        {
            if wizard.revealed_exile {
                self.state.conceal_exiled_card(wizard.card);
            }
            self.unwind_failed_wizard(&wizard);
        }
        result
    }

    fn unwind_failed_wizard(&mut self, wizard: &CastWizard) {
        if !self.rewind_cast(wizard)
            && let Some(assist) = wizard.assist.as_deref()
            && let (Some(helper), Some(start)) = (assist.assistant, assist.helper_start.as_deref())
        {
            self.give_back_window(helper, start);
        }
        if let Some(obj) = self.state.object_mut(wizard.card)
            && obj.zone != crate::zone::Zone::Stack
        {
            obj.x_value = 0;
        }
    }

    /// Carries a cast on after its caster answered one of its questions.
    ///
    /// The answer was one the question offered, so it is taken, whatever
    /// the rest of the checklist then finds. A step the caster cannot
    /// complete — the total cost with the X they named, the targets their
    /// mode needs — makes the casting illegal, and the game returns to the
    /// moment before it was proposed (CR 601.2, CR 734.1): that is the
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
    /// back at instead. The reversal journals nothing, as CR 734.1 has it:
    /// an undone action leaves nothing behind.
    pub(crate) fn continue_cast_wizard(&mut self) {
        // Read before the stages run: `finish_cast` nulls the wizard itself
        // on the way out, so by the error branch there is nobody left to ask.
        let caster = self.cast_wizard.as_ref().map(|w| w.player);
        let madness = self.cast_wizard.as_ref().and_then(|wizard| {
            (wizard.alternative_cost == Some(CastModeKind::Madness))
                .then_some(wizard.card)
                .and_then(|card| {
                    self.state
                        .object(card)
                        .filter(|object| object.zone == crate::zone::Zone::Exile)
                        .map(|object| (card, object.version))
                })
        });
        if self.advance_cast_wizard().is_err() {
            if let Some((card, version)) = madness {
                self.state.madness_to_graveyard(card, version);
            }
            self.reverse_cast(caster);
        }
    }

    /// Returns the game to the moment before a cast was proposed (CR 601.2,
    /// CR 734.1), once the cast has turned out to be illegal part-way.
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
    fn phyrexian_preview_price(
        &self,
        wizard: &CastWizard,
        face: &'static baylee_cards_dsl::FaceDef,
        unsettled: &CastWizard,
    ) -> ManaCost {
        let mut price = wizard_total_cost(face, unsettled);
        if let Some(def) = self
            .state
            .object(wizard.card)
            .and_then(|o| o.card)
            .and_then(|c| self.lookup.card(c.index))
        {
            let adj = casting::face_adjustment(
                &self.state,
                wizard.player,
                wizard.card,
                def,
                wizard.face_index(),
                None,
            );
            if adj.has_target_conditional_reduction {
                let kind = match wizard.option {
                    Some(CastModeKind::Kicked) => casting::PreviewCastKind::Kicked,
                    Some(CastModeKind::Bestow) => casting::PreviewCastKind::Bestow,
                    Some(CastModeKind::Mutate) => casting::PreviewCastKind::Mutate,
                    Some(CastModeKind::Fused) => casting::PreviewCastKind::Fused,
                    _ => casting::PreviewCastKind::Face,
                };
                let preview = casting::preview_cast_metric_bounds_for(
                    &self.state,
                    wizard.player,
                    wizard.card,
                    def,
                    wizard.face_index(),
                    None,
                    kind,
                    &adj,
                );
                price = price.with_less_mana(&preview.max_mana_reduction, false);
            }
        }
        price
    }

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
        let price = self.phyrexian_preview_price(wizard, face, &unsettled);
        let spendable =
            casting::spendable_pool(&self.state, wizard.player, spend_for(wizard, face));
        let pool = spendable
            .as_ref()
            .unwrap_or(&self.state.players[usize::from(wizard.player.get())].mana_pool);
        let open = |settled: &[bool]| {
            let life = unsettled.reserved_life(face).saturating_add(
                2 * i32::try_from(settled.iter().filter(|l| **l).count()).unwrap_or(i32::MAX),
            );
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
        while answers.len() < symbols {
            let mut with_life = answers.clone();
            with_life.push(true);
            let mut with_mana = answers.clone();
            with_mana.push(false);
            match (open(&with_life), open(&with_mana)) {
                (true, true) => break,
                (true, false) => answers.push(true),
                (false, true) => answers.push(false),
                (false, false) => {
                    self.cast_wizard = None;
                    return Some(Err(EngineError::IllegalAction("cannot pay the cost")));
                }
            }
        }
        let mut wizard = Box::new(wizard.clone());
        wizard.phyrexian = answers;
        if wizard.phyrexian.len() < symbols {
            let card = self.state.object(wizard.card).and_then(|o| o.card);
            self.pending = Pending::YesNo {
                player: wizard.player,
                prompt: YesNoPrompt::PayLife {
                    amount: 2,
                    permanent: None,
                },
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
    /// hands the caster their priority back (CR 734.1).
    fn cast_or_make_miracle_mana(&mut self, wizard: Box<CastWizard>) -> Result<(), EngineError> {
        let cost = self.wizard_paid(&wizard);
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
                        cost: self
                            .north_star_payment(&wizard, self.miracle_window_cost(&wizard, cost)),
                        wizard,
                        version,
                        opened,
                    },
                });
                self.pending = self.window_priority(player, legal);
                self.awaiting_answer = true;
                // A commanded card is played only "if able" (Word of
                // Command). A price its player's lands cannot pay at all is
                // a casting that cannot comply with CR 601.2g-h: it is
                // reversed (CR 734.1) before anything is tapped, and the
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
        if let Err(err) = self.finish_cast(&wizard) {
            if wizard.revealed_exile {
                self.state.conceal_exiled_card(wizard.card);
            }
            self.unwind_failed_wizard(&wizard);
            return Err(err);
        }
        Ok(())
    }

    /// Closing the mana opportunity tries the chosen cast once. A short
    /// payment leaves the card in its original zone and cannot reopen an offer.
    /// The captured version also protects casts from graveyard or exile.
    ///
    /// A window that closes without the cast gives back what was made in it
    /// (CR 734.1, `Engine::give_back_window`): declining a miracle the
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
            let reason = if current && !self.wizard_pool_can_pay(wizard, self.wizard_paid(wizard)) {
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
            // abilities with it (CR 734.1); any other gives back the taps.
            // Both kinds were begun casts, so both record their cancellation.
            if !self.rewind_cast(wizard) {
                self.give_back_window(wizard.player, opened);
                if let Some(assist) = wizard.assist.as_deref()
                    && let (Some(helper), Some(start)) =
                        (assist.assistant, assist.helper_start.as_deref())
                {
                    self.give_back_window(helper, start);
                }
            }
            self.state.journal.record(GameEvent::CastCancelled {
                player: wizard.player,
                card: wizard.card,
                reason,
            });
            if let Some(card) = self.state.object_mut(wizard.card) {
                card.x_value = 0;
            }
            // A cast from priority that could not be paid is reversed, and
            // the player who had priority keeps it (CR 734.2).
            if !self.finish_nested_cast() && wizard.paying == casting::Paying::DuringTheCast {
                self.after_action(wizard.player);
            }
            if wizard.alternative_cost == Some(CastModeKind::Madness) {
                self.state.madness_to_graveyard(wizard.card, version);
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
            eval::target_domain::walls3(
                self.wizard_target_req(wizard),
                self.wizard_second_target_req(wizard),
                self.wizard_third_target_req(wizard),
            ),
            reference,
        )
    }

    /// The colours each permanent the cast taps for convoke pays with
    /// (CR 702.51a): `casting::tap_colors` of its chosen taps.
    pub(super) fn convoke_tap_colors(
        &self,
        wizard: &CastWizard,
    ) -> Vec<baylee_core::color::ColorSet> {
        let face = self.wizard_face(wizard);
        let convoke = casting::has_convoke(
            &self.state,
            wizard.card,
            face,
            wizard.player,
            wizard.face_index(),
        );
        casting::tap_colors_for(&self.state, convoke, &wizard.convoke_taps)
    }

    pub(super) fn wizard_effective_casualty(&self, wizard: &CastWizard) -> Option<u8> {
        let face = self.wizard_face(wizard);
        if let Some(n) = face.casualty {
            return Some(n);
        }
        casting::collect_stack_modifiers(
            &self.state,
            wizard.player,
            wizard.card,
            wizard.face_index(),
            |fx| match fx.modifier {
                baylee_cards_dsl::Modifier::GrantCasualty(n) => Some(n),
                _ => None,
            },
        )
        .into_iter()
        .max()
    }

    pub(super) fn wizard_effective_replicate(&self, wizard: &CastWizard) -> Option<ManaCost> {
        let face = self.wizard_face(wizard);
        if let Some(cost) = face.replicate {
            return Some(cost);
        }
        let has_grant = !casting::collect_stack_modifiers(
            &self.state,
            wizard.player,
            wizard.card,
            wizard.face_index(),
            |fx| {
                matches!(
                    fx.modifier,
                    baylee_cards_dsl::Modifier::GrantReplicateCardManaCost
                )
                .then_some(())
            },
        )
        .is_empty();
        has_grant.then_some(face.mana_cost)
    }

    /// What the pool pays for the cast at its payment ([`wizard_paid_total`]).
    pub(super) fn wizard_paid(&self, wizard: &CastWizard) -> ManaCost {
        wizard_paid_total(
            self.wizard_face(wizard),
            wizard,
            &self.convoke_tap_colors(wizard),
        )
        .with_less_generic(wizard.harmonize_reduction(&self.state))
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
        if let Some(clauses) = self.extended_wizard_clauses(wizard) {
            return clauses.first().map(|c| c.2);
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
        if wizard.option == Some(CastModeKind::Bestow) {
            return Some(casting::BESTOW_TARGET_REQ);
        }
        if wizard.kicked
            && let Some(req) = def.faces[face_index].kicked_targets
        {
            return Some(req);
        }
        let abilities = def.abilities_for_face(face_index);
        match wizard.option {
            Some(CastModeKind::Fused) => fused_target_req(def, 0),
            Some(CastModeKind::Mode(i)) => abilities
                .iter()
                .find_map(|a| a.modal_spell()?.0.get(i).and_then(|m| m.targets)),
            // Several modes: the first of them that says "target" is the
            // spell's first instance of the word (CR 700.2c), the second
            // its second — `wizard_second_target_req`.
            Some(CastModeKind::Modes(set)) => casting::modal_target_clauses(abilities, set)
                .first()
                .map(|&(_, _, req)| req),
            Some(CastModeKind::RepeatedModes(packed)) => {
                casting::repeated_modal_target_clauses(abilities, packed)
                    .first()
                    .map(|&(_, _, req)| req)
            }
            _ => abilities.iter().find_map(|a| match a {
                AbilityDef::Spell { targets, .. } => *targets,
                _ => None,
            }),
        }
    }

    pub(super) fn wizard_modal_clause(
        &self,
        wizard: &CastWizard,
        clause_idx: usize,
    ) -> Option<(usize, bool, TargetReq)> {
        if wizard.masked {
            return None;
        }
        let def = self
            .state
            .object(wizard.card)
            .and_then(|o| o.card)
            .and_then(|c| self.lookup.card(c.index))?;
        let face_index = wizard.face_index().min(def.faces.len() - 1);
        let abilities = def.abilities_for_face(face_index);
        match wizard.option {
            Some(CastModeKind::Modes(set)) => casting::modal_target_clauses(abilities, set)
                .get(clause_idx)
                .copied(),
            Some(CastModeKind::RepeatedModes(packed)) => {
                casting::repeated_modal_target_clauses(abilities, packed)
                    .get(clause_idx)
                    .copied()
            }
            _ => None,
        }
    }

    pub(super) fn wizard_extra_clause(
        &self,
        wizard: &CastWizard,
        clause_idx: usize,
    ) -> Option<(usize, bool, TargetReq)> {
        let (mode_idx, is_second, mut req) =
            if let Some(clauses) = self.extended_wizard_clauses(wizard) {
                *clauses.get(clause_idx)?
            } else if let Some(clause) = self.wizard_modal_clause(wizard, clause_idx) {
                clause
            } else if wizard.option == Some(CastModeKind::Fused) {
                let def = self
                    .state
                    .object(wizard.card)
                    .and_then(|o| o.card)
                    .and_then(|c| self.lookup.card(c.index))?;
                (0, false, fused_target_req(def, clause_idx)?)
            } else if clause_idx == 2 {
                (0, true, self.wizard_third_target_req(wizard)?)
            } else {
                return None;
            };
        if is_second
            && let TargetSpec::CardInFirstTargetsGraveyard(filter) = req.spec
            && let Some(player) = wizard.clause_first_targets(clause_idx).1
        {
            req.spec = TargetSpec::CardInPlayersGraveyard(filter, player);
        }
        Some((mode_idx, is_second, req))
    }

    pub(super) fn wizard_third_target_req(&self, wizard: &CastWizard) -> Option<TargetReq> {
        if wizard.masked || matches!(wizard.option, Some(CastModeKind::Disguise)) {
            return None;
        }
        let def = self
            .state
            .object(wizard.card)
            .and_then(|o| o.card)
            .and_then(|c| self.lookup.card(c.index))?;
        let face_index = wizard.face_index().min(def.faces.len() - 1);
        match wizard.option {
            Some(
                CastModeKind::Modes(_)
                | CastModeKind::RepeatedModes(_)
                | CastModeKind::Mode(_)
                | CastModeKind::Fused,
            ) => None,
            _ => def
                .abilities_for_face(face_index)
                .iter()
                .find_map(|a| match a {
                    AbilityDef::Spell { third_targets, .. } => *third_targets,
                    _ => None,
                }),
        }
    }

    pub(super) fn wizard_clause_distinct_pool(
        &self,
        wizard: &CastWizard,
        req: TargetReq,
    ) -> eval::DistinctGroupPool {
        let distinct_from_prior = req.constraint()
            == Some(baylee_cards_dsl::TargetConstraint::DistinctFromPriorTargetGroups);
        let (min, _) = req.bounds(wizard.x);
        if min == 0
            || matches!(
                req.spec,
                TargetSpec::Player(rel) if rel != baylee_cards_dsl::PlayerRel::You
            )
        {
            return eval::DistinctGroupPool {
                needed: 0,
                distinct_from_prior,
                objects: Vec::new(),
                players: Vec::new(),
            };
        }
        let objects = self.wizard_target_options(wizard, &req.spec);
        let players = eval::target_player_options_from(
            &self.state,
            &req.spec,
            wizard.player,
            eval::live_context(&self.state, wizard.card),
        );
        eval::DistinctGroupPool {
            needed: usize::try_from(min).unwrap_or(usize::MAX),
            distinct_from_prior,
            objects,
            players,
        }
    }

    pub(super) fn wizard_prior_and_later_pools(
        &self,
        wizard: &CastWizard,
        clause_idx: usize,
    ) -> (
        Vec<ObjectId>,
        baylee_core::ids::SeatSet,
        Vec<eval::DistinctGroupPool>,
    ) {
        let mut prior_objs = Vec::new();
        let mut prior_players = baylee_core::ids::SeatSet::new();
        if clause_idx >= 1 {
            for &id in &wizard.targets {
                if !prior_objs.contains(&id) {
                    prior_objs.push(id);
                }
            }
            for p in wizard.target_players.iter() {
                prior_players.insert(p);
            }
            if let Some(p) = wizard.chosen_player {
                prior_players.insert(p);
            }
        }
        if clause_idx >= 2 {
            for &id in &wizard.second_targets {
                if !prior_objs.contains(&id) {
                    prior_objs.push(id);
                }
            }
            for p in wizard.second_target_players.iter() {
                prior_players.insert(p);
            }
        }
        if clause_idx > 2 {
            for (objs, pls) in wizard.extra_targets.iter().take(clause_idx - 2) {
                for &id in objs {
                    if !prior_objs.contains(&id) {
                        prior_objs.push(id);
                    }
                }
                for p in pls.iter() {
                    prior_players.insert(p);
                }
            }
        }
        let mut later = Vec::new();
        if clause_idx == 0
            && let Some(req) = self.wizard_second_target_req(wizard)
        {
            later.push(self.wizard_clause_distinct_pool(wizard, req));
        }
        let mut next_extra = clause_idx.max(1) + 1;
        while let Some((_, _, req)) = self.wizard_extra_clause(wizard, next_extra) {
            later.push(self.wizard_clause_distinct_pool(wizard, req));
            next_extra += 1;
        }
        (prior_objs, prior_players, later)
    }

    /// The spell's requirement for a second instance of the word "target",
    /// if it prints one: [`AbilityDef::Spell`]'s, the chosen mode's
    /// (Archdruid's Charm's second mode says "target" twice), or, for a set
    /// of chosen modes, the second target clause across them (CR 700.2c).
    pub(super) fn wizard_second_target_req(&self, wizard: &CastWizard) -> Option<TargetReq> {
        if wizard.masked {
            return None;
        }
        if let Some(clauses) = self.extended_wizard_clauses(wizard) {
            let &(_, is_sec, mut req) = clauses.get(1)?;
            if is_sec
                && let TargetSpec::CardInFirstTargetsGraveyard(filter) = req.spec
                && let Some(player) = wizard
                    .chosen_player
                    .or_else(|| wizard.target_players.iter().next())
            {
                req.spec = TargetSpec::CardInPlayersGraveyard(filter, player);
            }
            return Some(req);
        }
        let def = self
            .state
            .object(wizard.card)
            .and_then(|o| o.card)
            .and_then(|c| self.lookup.card(c.index))
            .expect("wizard card known");
        let face_index = wizard.face_index().min(def.faces.len() - 1);
        let abilities = def.abilities_for_face(face_index);
        let mut is_second_in_mode = true;
        let mut req = match wizard.option {
            Some(CastModeKind::Fused) => fused_target_req(def, 1),
            Some(CastModeKind::Mode(i)) => abilities
                .iter()
                .find_map(|a| a.modal_spell()?.0.get(i).and_then(|m| m.second_targets)),
            Some(CastModeKind::Modes(set)) => {
                let (_, is_sec, r) = *casting::modal_target_clauses(abilities, set).get(1)?;
                is_second_in_mode = is_sec;
                Some(r)
            }
            Some(CastModeKind::RepeatedModes(packed)) => {
                let (_, is_sec, r) =
                    *casting::repeated_modal_target_clauses(abilities, packed).get(1)?;
                is_second_in_mode = is_sec;
                Some(r)
            }
            _ => abilities.iter().find_map(|a| match a {
                AbilityDef::Spell { second_targets, .. } => *second_targets,
                _ => None,
            }),
        }?;
        if is_second_in_mode
            && let TargetSpec::CardInFirstTargetsGraveyard(filter) = req.spec
            && let Some(player) = wizard
                .chosen_player
                .or_else(|| wizard.target_players.iter().next())
        {
            req.spec = TargetSpec::CardInPlayersGraveyard(filter, player);
        }
        Some(req)
    }

    fn wizard_pitch_parts(&self, wizard: &CastWizard) -> SmallVec<[CostPart; 4]> {
        cost_parts::wizard_alternative_parts_expanded(
            self.wizard_face(wizard),
            wizard.cost_choice(),
        )
    }

    pub(super) fn wizard_either_branch_payable(
        &self,
        wizard: &CastWizard,
        face: &'static baylee_cards_dsl::FaceDef,
        choose_first: bool,
        branch_cost: &'static baylee_cards_dsl::Cost,
    ) -> bool {
        let mut probe = wizard.clone();
        let idx = probe.either_decided;
        if choose_first && idx < 8 {
            probe.either_first_mask |= 1_u8 << idx;
        }
        if branch_cost.mana != ManaCost::ZERO {
            probe.either_pays_mana = true;
        }
        probe.either_decided = idx.saturating_add(1);
        if !self
            .state
            .can_pay_life(probe.player, probe.total_life_cost(face))
        {
            return false;
        }
        let pitch_parts = self.wizard_pitch_parts(&probe);
        if !casting::pitch_costs_payable(
            &self.state,
            probe.player,
            probe.card,
            &pitch_parts,
            &probe.pitch,
        ) {
            return false;
        }
        if casting::additional_payment_capacity_for_x(
            &self.state,
            probe.player,
            probe.card,
            face,
            probe.cost_choice(),
            probe.x,
            &probe.cost_objects,
        )
        .is_none()
        {
            return false;
        }
        if probe.makes_mana_after_choices() && self.can_make_mana_for_cast(probe.player) {
            return true;
        }
        if !pays_mana(&probe) {
            return true;
        }
        let probe_face_idx = probe.face_index();
        let probe_delve =
            casting::has_delve(&self.state, probe.card, face, probe.player, probe_face_idx);
        let probe_convoke =
            casting::has_convoke(&self.state, probe.card, face, probe.player, probe_face_idx);
        let probe_improvise =
            casting::has_improvise_for(&self.state, probe.card, face, probe.player, probe_face_idx);
        let graveyard = if probe_delve {
            self.state
                .zones
                .list(ZoneLocation::Graveyard(probe.player))
                .len()
        } else {
            0
        };
        let taps = if probe_convoke || probe_improvise {
            casting::tap_to_pay_sources_for(
                &self.state,
                probe_convoke,
                probe_improvise,
                probe.player,
            )
            .len()
        } else {
            0
        };
        let harmonize = if probe.is_harmonize() {
            usize::try_from(casting::harmonize_max_power(&self.state, probe.player))
                .unwrap_or(usize::MAX)
        } else {
            0
        };
        let help = u32::try_from(graveyard.saturating_add(taps).saturating_add(harmonize))
            .unwrap_or(u32::MAX);
        let cost = wizard_total_cost(face, &probe).with_less_generic(help);
        self.wizard_pool_can_pay(&probe, cost)
    }

    /// Pays everything and puts the spell on the stack, or, refused, pays
    /// nothing.
    ///
    /// The mana is paid first and all at once, and every refusal after it (a
    /// delve, pitch or sacrifice answer naming an object that is gone, the
    /// card itself gone) used to leave what was paid by then spent while the
    /// cast was reversed. CR 734.1 cancels "any payments already made", so
    /// the game is kept before the first of them and put back on a refusal,
    /// with the spell-rider abilities the mana put on the stack.
    fn finish_cast(&mut self, wizard: &CastWizard) -> Result<(), EngineError> {
        let cost = self.wizard_paid(wizard);
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

    /// Whether a free cast's options may count on mana `player` makes
    /// while paying (CR 601.2g): [`Self::can_make_mana_for_cast`].
    ///
    /// The free-cast offers (a linked or immediate free play, a
    /// transformed back face, a cast an effect commands) used to ask
    /// `has_mana_source` of the offer as priority would list it, where an
    /// entry in `abilities` is any activation at all: a seat with a
    /// Prodigal Sorcerer and every land tapped was offered a mode whose
    /// mana nothing could make.
    pub(super) fn free_cast_makes_mana(&self, player: PlayerId) -> bool {
        #[cfg(test)]
        if super::granted::ASK_MANA_AS_BEFORE.with(std::cell::Cell::get) {
            return self.compute_legal(player).has_mana_source();
        }
        self.can_make_mana_for_cast(player)
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
        let def = self
            .state
            .object(wizard.card)
            .and_then(|o| o.card)
            .and_then(|c| self.lookup.card(c.index));
        let has_strive =
            def.is_some_and(|def| def.per_extra_target_cost(wizard.face_index()).is_some());
        let has_dynamic_cast_cost = def.is_some_and(|def| {
            let adj = casting::face_adjustment(
                &self.state,
                wizard.player,
                wizard.card,
                def,
                wizard.face_index(),
                None,
            );
            adj.per_distinct_target_delta != 0 || adj.per_x_paid_delta != 0
        });
        let face_idx = wizard.face_index();
        let delve = casting::has_delve(&self.state, wizard.card, face, wizard.player, face_idx);
        let convoke = casting::has_convoke(&self.state, wizard.card, face, wizard.player, face_idx);
        let improvise =
            casting::has_improvise_for(&self.state, wizard.card, face, wizard.player, face_idx);
        let has_wb = face.waterbend
            || (!(wizard.option == Some(CastModeKind::Disguise) || wizard.masked)
                && face
                    .mandatory_additional_costs
                    .iter()
                    .any(|p| matches!(p, CostPart::Waterbend(_) | CostPart::WaterbendX)));
        !wizard.makes_mana_after_choices()
            && !delve
            && !convoke
            && !improvise
            && !wizard.is_harmonize()
            && !has_wb
            && face.extra_target_cost == 0
            && !has_strive
            && !has_dynamic_cast_cost
            && !self.wizard_has_assist(wizard)
            && self.state.granted_colorless_capacity(wizard.player) == 0
    }

    /// A finite upper bound for a mana X, including ways of paying generic
    /// mana without spending units from the pool. The final payment still
    /// validates fixed colored costs and choices made after the X question.
    #[allow(clippy::too_many_lines)]
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
        if let Some(def) = self
            .state
            .object(wizard.card)
            .and_then(|o| o.card)
            .and_then(|c| self.lookup.card(c.index))
        {
            let adj = casting::face_adjustment(
                &self.state,
                wizard.player,
                wizard.card,
                def,
                wizard.face_index(),
                None,
            );
            if adj.per_distinct_target_delta < 0 {
                let graveyard_and_exile: usize = self
                    .state
                    .players
                    .iter()
                    .map(|p| {
                        self.state.zones.list(ZoneLocation::Graveyard(p.id)).len()
                            + self.state.zones.list(ZoneLocation::Exile(p.id)).len()
                    })
                    .sum();
                let target_pool = u32::try_from(
                    self.state.battlefield_view().len()
                        + self.state.zones.list(ZoneLocation::Stack).len()
                        + self.state.players.len()
                        + graveyard_and_exile,
                )
                .unwrap_or(u32::MAX);
                let per = adj.per_distinct_target_delta.unsigned_abs();
                bound = bound.saturating_add(per.saturating_mul(target_pool));
            }
            if adj.per_x_paid_delta < 0 {
                let max_x =
                    casting::preview_max_mandatory_x(&self.state, wizard.player, wizard.card, face);
                let per = adj.per_x_paid_delta.unsigned_abs();
                bound = bound.saturating_add(per.saturating_mul(max_x));
            }
            if adj.has_target_conditional_reduction || adj.has_target_conditional_tax {
                let kind = match wizard.option {
                    Some(CastModeKind::Kicked) => casting::PreviewCastKind::Kicked,
                    Some(CastModeKind::Bestow) => casting::PreviewCastKind::Bestow,
                    Some(CastModeKind::Mutate) => casting::PreviewCastKind::Mutate,
                    Some(CastModeKind::Fused) => casting::PreviewCastKind::Fused,
                    _ => casting::PreviewCastKind::Face,
                };
                let preview = casting::preview_cast_metric_bounds_for(
                    &self.state,
                    wizard.player,
                    wizard.card,
                    def,
                    wizard.face_index(),
                    None,
                    kind,
                    &adj,
                );
                bound = bound
                    .saturating_add(preview.max_mana_reduction.cmc())
                    .saturating_add(preview.max_reduction);
            }
        }
        let face_idx = wizard.face_index();
        let delve = casting::has_delve(&self.state, wizard.card, face, wizard.player, face_idx);
        let convoke = casting::has_convoke(&self.state, wizard.card, face, wizard.player, face_idx);
        let improvise =
            casting::has_improvise_for(&self.state, wizard.card, face, wizard.player, face_idx);
        for (enabled, count) in [
            (
                delve,
                self.state
                    .zones
                    .list(ZoneLocation::Graveyard(wizard.player))
                    .len(),
            ),
            (
                convoke || improvise,
                casting::tap_to_pay_sources_for(&self.state, convoke, improvise, wizard.player)
                    .len(),
            ),
            (
                face.waterbend
                    || (!(wizard.option == Some(CastModeKind::Disguise) || wizard.masked)
                        && face
                            .mandatory_additional_costs
                            .iter()
                            .any(|p| matches!(p, CostPart::Waterbend(_) | CostPart::WaterbendX))),
                casting::waterbend_sources(&self.state, wizard.player).len(),
            ),
            (
                wizard.is_harmonize(),
                usize::try_from(casting::harmonize_max_power(&self.state, wizard.player))
                    .unwrap_or(usize::MAX),
            ),
        ] {
            if enabled {
                bound = bound.saturating_add(u32::try_from(count).unwrap_or(u32::MAX));
            }
        }
        if self.wizard_has_assist(wizard) {
            for p in &self.state.players {
                if p.loss.is_none() && p.id != wizard.player {
                    if self.mana_window_legal(p.id).is_some() {
                        return u32::MAX;
                    }
                    bound = bound.saturating_add(casting::spendable_units(
                        &self.state,
                        p.id,
                        spend_for(wizard, face),
                    ));
                }
            }
        }
        bound
    }

    pub(super) fn wizard_has_assist(&self, wizard: &CastWizard) -> bool {
        !wizard.masked
            && wizard.option != Some(CastModeKind::Disguise)
            && self
                .wizard_face(wizard)
                .keywords
                .contains(baylee_cards_dsl::KeywordSet::ASSIST)
    }

    pub(super) fn assist_max_generic(
        &self,
        wizard: &CastWizard,
        helper: PlayerId,
        generic_owed: u32,
    ) -> u32 {
        let face = self.wizard_face(wizard);
        let spend = spend_for(wizard, face);
        let allowance = self.wizard_source_allowance(wizard);
        let pool_total = u32::try_from(self.state.players[helper.get() as usize].mana_pool.total())
            .unwrap_or(u32::MAX);
        let mut lo = 0_u32;
        let mut hi = generic_owed.min(pool_total);
        while lo < hi {
            let mid = lo + (hi - lo).div_ceil(2);
            if casting::plan_mana_scoped(
                &self.state,
                helper,
                spend,
                &ManaCost::from_symbol_generic(mid),
                None,
                allowance,
            )
            .is_some()
            {
                lo = mid;
            } else {
                hi = mid - 1;
            }
        }
        lo
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
            .filter(|_| {
                !wizard.free
                    && wizard.option != Some(CastModeKind::Plot)
                    && wizard.alternative_cost != Some(CastModeKind::Plot)
            });
        let has_strive = self
            .state
            .object(wizard.card)
            .and_then(|o| o.card)
            .and_then(|c| self.lookup.card(c.index))
            .is_some_and(|def| def.per_extra_target_cost(wizard.face_index()).is_some());
        if restricted_colors.is_none() && face.extra_target_cost == 0 && !has_strive {
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

    /// Materializes a color-restricted X payment (`x_mana_color` or
    /// `x_mana_colors`) into colored or hybrid mana symbols on the cost
    /// exposed by [`super::PaymentContinuation::Miracle`], so that
    /// [`Self::payment_window`] (`view.owed`) and client/seat mana planners
    /// know which units of X require colored mana (e.g. Drain Life's
    /// "Spend only black mana on X").
    pub(super) fn miracle_window_cost(&self, wizard: &CastWizard, total: ManaCost) -> ManaCost {
        let (cost, Some(restriction)) = self.restricted_x_payment(wizard, total) else {
            return total;
        };
        if restriction.amount == 0 {
            return cost;
        }
        let bits = restriction.types;
        let symbol = match bits.count_ones() {
            1 => {
                let color = baylee_core::color::Color::ALL[bits.trailing_zeros() as usize];
                Some(baylee_core::mana::ManaSymbol::of_color(color))
            }
            2 => {
                let first = baylee_core::color::Color::ALL[bits.trailing_zeros() as usize];
                let rest = bits & (bits - 1);
                let second = baylee_core::color::Color::ALL[rest.trailing_zeros() as usize];
                Some(baylee_core::mana::ManaSymbol::Hybrid(
                    baylee_core::color::ColorPair::new(first, second),
                ))
            }
            _ => None,
        };
        if let Some(symbol) = symbol {
            cost.with_less_generic(restriction.amount)
                .with_symbol(symbol, restriction.amount)
        } else {
            cost
        }
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
    let before = wizard_total_before_reduction(face, wizard);
    let Some(reduction) = wizard.reduction else {
        return before;
    };
    before
        .with_less_mana(&reduction.colored_only_remainder, true)
        .with_less_mana(&reduction.mana_remainder, false)
        .with_less_generic(reduction.remainder)
}

/// What the pool pays for the cast at its payment: the total less what
/// delve and convoke already paid. A convoke tap pays a coloured symbol of
/// its colour where it can (CR 702.51a; `tap_colors` from
/// [`Engine::convoke_tap_colors`]), and generic mana otherwise; delve's
/// exiles pay generic mana (CR 702.66a).
pub(super) fn wizard_paid_total(
    face: &baylee_cards_dsl::FaceDef,
    wizard: &CastWizard,
    tap_colors: &[baylee_core::color::ColorSet],
) -> ManaCost {
    let (cost, colored) = wizard_total_cost(face, wizard).convoked_colored(tap_colors);
    let generic = u32::try_from(wizard.delve_exiles.len() + wizard.convoke_taps.len())
        .unwrap_or(u32::MAX)
        .saturating_sub(colored)
        .saturating_add(wizard.assist_paid);
    cost.with_less_generic(generic)
}

pub(super) fn wizard_mandatory_waterbend_generic(
    face: &baylee_cards_dsl::FaceDef,
    wizard: &CastWizard,
) -> u32 {
    if wizard.option == Some(CastModeKind::Disguise) || wizard.masked {
        return 0;
    }
    cost_parts::wizard_cost_parts(face, wizard.cost_choice())
        .filter_map(|part| match part {
            CostPart::Waterbend(n) => Some(u32::from(*n)),
            CostPart::WaterbendX => Some(wizard.x),
            _ => None,
        })
        .fold(0_u32, u32::saturating_add)
}

impl CastWizard {
    /// Whether this cast is using a stack/graveyard `GrantsHarmonizeManaCost` route.
    pub(crate) fn is_harmonize(&self) -> bool {
        self.selected_alt_grant().is_some_and(|r| r.harmonize)
    }

    /// Generic mana reduction paid by the single creature tapped for Harmonize.
    pub(crate) fn harmonize_reduction(&self, state: &crate::state::GameState) -> u32 {
        self.harmonize_tap
            .and_then(|id| state.object(id))
            .and_then(|o| o.characteristics().power)
            .map_or(0, |p| u32::try_from(p.max(0)).unwrap_or(0))
    }

    /// Targets chosen beyond the first across all target clauses (`CR 207.2c` Strive).
    pub(super) fn extra_target_count(&self) -> u32 {
        let players = self.target_players.len()
            + usize::from(
                self.chosen_player
                    .is_some_and(|p| !self.target_players.contains(p)),
            );
        let extra_groups: usize = self
            .extra_targets
            .iter()
            .map(|(objs, pls)| objs.len() + pls.len())
            .sum();
        let extra = (self.targets.len()
            + self.second_targets.len()
            + self.second_target_players.len()
            + extra_groups
            + players)
            .saturating_sub(1);
        u32::try_from(extra).unwrap_or(u32::MAX)
    }
}

fn wizard_total_before_reduction(
    face: &baylee_cards_dsl::FaceDef,
    wizard: &CastWizard,
) -> ManaCost {
    let mut total = wizard_cost(wizard).combine(&wizard.target_increase);
    let wb = wizard_mandatory_waterbend_generic(face, wizard);
    if wb > 0 {
        total = total.with_more_generic(wb);
    }
    let extra = wizard.extra_target_count();
    total = total.with_more_generic(face.extra_target_cost.saturating_mul(extra));
    if wizard.kicked
        && wizard.option != Some(CastModeKind::Kicked)
        && !wizard.selected_alt_grant().is_some_and(|r| r.kicked)
        && !matches!(
            wizard.announced,
            Some(
                CastModeKind::EscapeGranted { kicked: true, .. }
                    | CastModeKind::LibraryTopLife { kicked: true, .. }
            )
        )
    {
        for (idx, add) in face.additional_costs.iter().enumerate() {
            if wizard.kicked_mask == 0 || (idx < 8 && (wizard.kicked_mask & (1_u8 << idx)) != 0) {
                total = total.combine(&add.mana.with_x(wizard.x));
            }
        }
    }
    if let Some(multikicker) = face.multikicker {
        total = total.combine_n(&multikicker, u32::from(wizard.multikicked));
    }
    if let (Some(less), Some(buyback)) = (wizard.buyback, face.buyback) {
        total = total.combine(&buyback.mana.with_less_generic(less));
    }
    if let Some(replicate) = wizard.granted_replicate.or(face.replicate) {
        total = total.combine_n(&replicate, u32::from(wizard.replicated));
    }
    if wizard.offspring
        && let Some(offspring) = face.offspring
    {
        total = total.combine(&offspring.mana.with_x(wizard.x));
    }
    if wizard.optional_cost_paid
        && wizard.option != Some(CastModeKind::OptionalCost)
        && let Some(opt_cost) = face.optional_cost
    {
        total = total.combine(&opt_cost.mana.with_x(wizard.x));
    }
    if wizard.squad > 0
        && let Some(squad) = face.squad
    {
        total = total.combine_n(&squad.mana.with_x(wizard.x), u32::from(wizard.squad));
    }
    if wizard.behold_paid_mana
        && let Some(behold) = face.behold_or_pay
    {
        total = total.combine(&behold.cost);
    }
    if wizard.either_decided > 0 {
        let either_mana = cost_parts::settled_either_mana(face, wizard.cost_choice());
        if either_mana != ManaCost::ZERO {
            total = total.combine(&either_mana.with_x(wizard.x));
        }
    }
    for &(_, _, _, cost) in &wizard.spliced {
        total = total.combine(&cost.mana.with_x(wizard.x));
    }
    total
}

/// Whether this cast pays mana at all. A free cast pays none of its mana
/// cost (CR 601.2h pays nothing it was not asked for), but an additional
/// cost it chose is still paid (CR 118.9d, 601.2f): a kicker, a buyback, and
/// the replicate cost however many times it was announced.
fn pays_mana(wizard: &CastWizard) -> bool {
    (!wizard.free
        && !matches!(wizard.option, Some(CastModeKind::Plot))
        && !matches!(wizard.alternative_cost, Some(CastModeKind::Plot)))
        || wizard.kicked
        || wizard.buyback.is_some()
        || wizard.replicated > 0
        || wizard.offspring
        || wizard.optional_cost_paid
        || wizard.squad > 0
        || wizard.behold_paid_mana
        || wizard.either_pays_mana
        || wizard.target_increase != ManaCost::ZERO
        || !wizard.spliced.is_empty()
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
            Some(CastModeKind::Bestow) => face.bestow.map(casting::SpellForm::Bestow),
            Some(CastModeKind::Blitz) => Some(casting::SpellForm::Blitz),
            Some(CastModeKind::Plot) => Some(casting::SpellForm::Plot),
            Some(CastModeKind::Impending) => face
                .impending
                .map(|(_, c)| casting::SpellForm::Impending(c)),
            _ if wizard.alternative_cost == Some(CastModeKind::Plot) => {
                Some(casting::SpellForm::Plot)
            }
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
        Some(CastModeKind::Bestow) => casting::SpendFor::SpellAs(
            wizard.card,
            casting::SpellForm::Bestow(face.bestow.expect("bestow option")),
        ),
        Some(CastModeKind::Blitz) => {
            casting::SpendFor::SpellAs(wizard.card, casting::SpellForm::Blitz)
        }
        Some(CastModeKind::Foretell) => casting::SpendFor::SpellAs(
            wizard.card,
            casting::SpellForm::Foretell(face.foretell.expect("foretell option")),
        ),
        Some(CastModeKind::Plot) => {
            casting::SpendFor::SpellAs(wizard.card, casting::SpellForm::Plot)
        }
        Some(CastModeKind::Impending) => casting::SpendFor::SpellAs(
            wizard.card,
            casting::SpellForm::Impending(face.impending.expect("impending option").1),
        ),
        _ if wizard.alternative_cost == Some(CastModeKind::Plot) => {
            casting::SpendFor::SpellAs(wizard.card, casting::SpellForm::Plot)
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
    if let Some(idx) = wizard.chosen_option_index
        && let Some(opt) = wizard.options.get(usize::from(idx))
    {
        return opt.cost;
    }
    wizard
        .options
        .iter()
        .find(|o| Some(o.kind) == wizard.announced.or(wizard.option))
        .map_or(ManaCost::ZERO, |o| o.cost)
}

/// Target requirements across both halves of a fused split spell (CR 702.102a,
/// CR 702.102d): left half's target groups followed by right half's.
fn fused_target_req(def: &baylee_cards_dsl::CardDef, slot: usize) -> Option<TargetReq> {
    (0..def.faces.len().min(2))
        .flat_map(|face_index| {
            def.abilities_for_face(face_index)
                .iter()
                .flat_map(|ability| match ability {
                    AbilityDef::Spell {
                        targets,
                        second_targets,
                        third_targets,
                        ..
                    } => [*targets, *second_targets, *third_targets]
                        .into_iter()
                        .flatten(),
                    _ => [None, None, None].into_iter().flatten(),
                })
        })
        .nth(slot)
}

#[cfg(test)]
mod restricted_x_tests;

#[cfg(test)]
mod pool_x_tests;

#[cfg(test)]
mod convoke_x_tests;

/// The engine's own steps, named for the card test crates (`test-support`) and
/// their kit: the steps stay the engine's, these only call them.
#[cfg(any(test, feature = "test-support"))]
impl<L: CardLookup, D: crate::engine::LoopDetection> Engine<L, D> {
    #[doc(hidden)]
    pub fn start_free_cast_for_tests(
        &mut self,
        player: PlayerId,
        card: ObjectId,
    ) -> Result<(), EngineError> {
        self.start_free_cast(player, card)
    }
}

/// The engine's own steps, named for the card test crates (`test-support`) and
/// their kit: the steps stay the engine's, these only call them.
#[cfg(any(test, feature = "test-support"))]
impl<L: CardLookup, D: crate::engine::LoopDetection> Engine<L, D> {
    #[doc(hidden)]
    pub fn start_paid_cast_for_tests(
        &mut self,
        player: PlayerId,
        card: ObjectId,
        version: u32,
        then_no_more_spells: bool,
        any_mana: bool,
        opened: Box<super::WindowStart>,
    ) -> Result<(), EngineError> {
        self.start_paid_cast(player, card, version, then_no_more_spells, any_mana, opened)
    }
}
