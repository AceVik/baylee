//! Object identity for instructions which affect their own subject.
//! Damage attribution deliberately continues to use the original source.
use super::{Resolution, source_version};
use crate::{
    event::{Cause, GameEvent},
    state::GameState,
    zone::Zone,
};
use baylee_core::ids::{DamageSourceRef, ObjectId};

/// The answer and proof state of a named-card guess.
#[derive(Clone, Copy, Debug, Hash)]
pub struct GuessOutcome {
    /// Who made the guess.
    pub guesser: baylee_core::ids::PlayerId,
    /// Card index of the chosen name.
    pub card: baylee_core::ids::CardIndex,
    /// Face of `card` whose name was chosen.
    pub face: u8,
    /// Whether a card with the chosen name was actually in the hand.
    pub actual: bool,
    /// What the guessing player claimed.
    pub claimed: bool,
    /// Whether the hand was revealed as proof.
    pub revealed: bool,
}

/// Per-mode narrowed target bindings for modal spells with 3+ target clauses or
/// within-mode `second_targets` (`CR 700.2c`, `CR 608.2b–c`).
#[derive(Clone, Debug, Hash)]
pub struct ModalTargetStep {
    /// Remaining effects from the end of `res.effects` at which this mode starts.
    pub left_from_end: usize,
    /// Legal object targets of this mode's first target clause.
    pub targets: smallvec::SmallVec<[baylee_core::ids::ObjectId; 2]>,
    /// Legal player targets of this mode's first target clause.
    pub target_players: baylee_core::ids::SeatSet,
    /// Legal object targets of this mode's second target clause.
    pub second_targets: smallvec::SmallVec<[baylee_core::ids::ObjectId; 1]>,
    /// Legal player targets of this mode's second target clause.
    pub second_players: baylee_core::ids::SeatSet,
    /// Legal object targets of this segment's third target clause, if any.
    pub third_targets: smallvec::SmallVec<[baylee_core::ids::ObjectId; 2]>,
    /// Legal player targets of this segment's third target clause, if any.
    pub third_players: baylee_core::ids::SeatSet,
}

/// Per-nesting-level state for [`baylee_cards_dsl::Effect::RepeatProcess`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(in crate::resolve) struct RepeatProcessFrame {
    pub(super) tail_from_end: usize,
    pub(super) completed: u32,
    pub(super) limit: u32,
    pub(super) saved_this_way_from: Option<usize>,
}

/// Which target group is currently active in `res.targets` / `res.target_players`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(in crate::resolve) enum ActiveTargetGroup {
    Second,
    Third,
    All,
}

/// Saved outer target bindings pushed on entry to
/// [`baylee_cards_dsl::Effect::WithSecondTarget`] or
/// [`baylee_cards_dsl::Effect::WithThirdTarget`].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(in crate::resolve) struct TargetRestoreFrame {
    pub(super) tail_from_end: usize,
    pub(super) active_group: ActiveTargetGroup,
    pub(super) saved_targets: smallvec::SmallVec<[baylee_core::ids::ObjectId; 2]>,
    pub(super) saved_target_players: baylee_core::ids::SeatSet,
    pub(super) saved_chosen_player: Option<baylee_core::ids::PlayerId>,
    pub(super) saved_targeted: bool,
}

/// Cumulative last-known-information stats across permanents sacrificed during
/// this resolution (`CR 608.2h`, `CR 107.1b`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct SacrificeStats {
    /// Number of permanents actually sacrificed during this resolution.
    pub count: u32,
    /// Signed sum of LKI powers (floored at 0 when evaluated per `CR 107.1b`).
    pub total_power: i64,
    /// Signed sum of LKI toughnesses (floored at 0 when evaluated per `CR 107.1b`).
    pub total_toughness: i64,
    /// Saturating sum of LKI mana values.
    pub total_mana_value: u32,
    /// Union of LKI colors across all sacrificed permanents.
    pub colors: baylee_core::color::ColorSet,
}

impl SacrificeStats {
    /// Accumulate one sacrificed permanent's pre-departure LKI snapshot.
    pub(crate) fn record(
        &mut self,
        lki: crate::object::SacrificedLki,
        colors: baylee_core::color::ColorSet,
    ) {
        self.count = self.count.saturating_add(1);
        self.total_power = self.total_power.saturating_add(i64::from(lki.power));
        self.total_toughness = self
            .total_toughness
            .saturating_add(i64::from(lki.toughness));
        self.total_mana_value = self.total_mana_value.saturating_add(lki.mana_value);
        self.colors = self.colors.union(colors);
    }
}

/// Public-zone objects found through earlier instructions of this resolution.
#[derive(Clone, Debug, Default)]
pub struct SubjectContext {
    second_target_players: baylee_core::ids::SeatSet,
    third_targets: smallvec::SmallVec<[baylee_core::ids::ObjectId; 2]>,
    third_target_players: baylee_core::ids::SeatSet,
    modal_steps: Vec<ModalTargetStep>,
    followed: Vec<DamageSourceRef>,
    pub(super) reanimation_links: Vec<super::linked_reanimation::PendingLink>,
    /// Exact future/current battlefield incarnations from the latest search.
    pub(super) found: Vec<DamageSourceRef>,
    /// Original inspected segment of an unfinished multi-instruction library search.
    pub(super) inspected_library: Option<Vec<ObjectId>>,
    pub(super) guess: Option<GuessOutcome>,
    pub(super) named: Option<(crate::object::PrintedFace, baylee_core::ids::NameRef)>,
    pub(super) chosen_subtype: Option<baylee_core::ids::SubtypeId>,
    pub(super) chosen_color: Option<baylee_core::mana::ManaColor>,
    pub(super) sacrificed: Option<crate::object::SacrificedLki>,
    pub(super) sacrifice_stats: SacrificeStats,
    circled_colors: baylee_core::color::ColorSet,
    acquisition: Option<u64>,
    /// Original ability source, separate from any successor found by its costs.
    pub(super) source_version: Option<u32>,
    mana_tap_source: Option<DamageSourceRef>,
    event_player: Option<baylee_core::ids::PlayerId>,
    event_identity: Option<(DamageSourceRef, i16)>,
    pub(super) for_each_remaining: Option<Vec<DamageSourceRef>>,
    pub(super) destruction_batches: Vec<super::destruction_receipts::Batch>,
    pub(super) payment_batches: Vec<super::choices::PaymentBatch>,
    /// A `ForEachMatching` whose instructions sacrifice, from its count to
    /// the sacrifices it ends with (`resolve::for_each`).
    pub(super) counted: Option<Box<super::for_each::Count>>,
    checkpoint: Option<(Vec<DamageSourceRef>, u64)>,
    pub(super) last_die_roll: Option<u32>,
    pub(super) coins_won: u32,
    pub(super) chosen_number: Option<u32>,
    pub(super) repeat_frames: Vec<super::wave3_ops::RepeatFrame>,
    pub(super) repeat_processes: Vec<RepeatProcessFrame>,
    pub(super) vote_queue: Option<Box<super::vote::VoteQueue>>,
    pub(super) branch_frames: Vec<super::wave13a::BranchFrame>,
    pub(super) target_restore_frames: Vec<TargetRestoreFrame>,
    /// Objects remembered by the innermost enclosing
    /// [`baylee_cards_dsl::Effect::CaptureAffected`], active while its `then`
    /// runs (`CR 400.7`, `CR 608.2h`).
    pub(crate) remembered_objects: Option<std::sync::Arc<[super::remembered::RememberedObject]>>,
    pub(in crate::resolve) capture_frames: Vec<super::remembered::CaptureFrame>,
    /// Damage prevented or replaced by an enclosing prevention/replacement-with-rider
    /// shield or static modifier (`ThisWay::DamagePrevented`, CR 615.5).
    pub(crate) damage_prevented: u32,
    /// Stashed opponent copy target during `Effect::Demonstrate` (`CR 702.148a`).
    pub(crate) demonstrate_opp: Option<(DamageSourceRef, baylee_core::ids::PlayerId)>,
    /// Running sum accumulated by `Effect::AccumulateChosenNumber` (Join forces).
    pub(super) accumulated_number: u32,
    /// Players remembered during this resolution (`ChoosePlayerEx`, `SecretChooseNumbers`).
    pub(super) remembered_players: smallvec::SmallVec<[baylee_core::ids::PlayerId; 4]>,
    /// Queued `(player, chosen_number, effects)` steps for multi-player numeric choices.
    pub(super) queued_player_number_steps: std::collections::VecDeque<(
        baylee_core::ids::PlayerId,
        u32,
        &'static [baylee_cards_dsl::Effect],
    )>,
    /// Outer `(event_player, chosen_number)` saved before splicing queued
    /// multi-player numeric choice steps, restored once the last step finishes.
    pub(super) queued_player_number_saved:
        Option<(Option<baylee_core::ids::PlayerId>, Option<u32>)>,
}
impl SubjectContext {
    /// Preserve the independent player targets of the second target instance.
    pub(crate) fn with_second_players(mut self, players: baylee_core::ids::SeatSet) -> Self {
        self.second_target_players = players;
        self
    }

    /// Current second-instance player targets, already narrowed for legality.
    pub(crate) fn second_players(&self) -> baylee_core::ids::SeatSet {
        self.second_target_players
    }

    pub(crate) fn set_second_players(&mut self, players: baylee_core::ids::SeatSet) {
        self.second_target_players = players;
    }

    /// Preserve the independent object targets of the third target instance.
    pub(crate) fn with_third_targets(
        mut self,
        targets: smallvec::SmallVec<[baylee_core::ids::ObjectId; 2]>,
    ) -> Self {
        self.third_targets = targets;
        self
    }

    /// Preserve the independent player targets of the third target instance.
    pub(crate) fn with_third_players(mut self, players: baylee_core::ids::SeatSet) -> Self {
        self.third_target_players = players;
        self
    }

    /// Current third-instance object targets, already narrowed for legality.
    pub(crate) fn third_targets(&self) -> &[baylee_core::ids::ObjectId] {
        &self.third_targets
    }

    /// Current third-instance player targets, already narrowed for legality.
    pub(crate) fn third_players(&self) -> baylee_core::ids::SeatSet {
        self.third_target_players
    }

    pub(crate) fn set_third_targets(
        &mut self,
        targets: smallvec::SmallVec<[baylee_core::ids::ObjectId; 2]>,
    ) {
        self.third_targets = targets;
    }

    pub(crate) fn set_third_players(&mut self, players: baylee_core::ids::SeatSet) {
        self.third_target_players = players;
    }

    /// Original first-slot object targets even while nested inside
    /// `WithSecondTarget` or `WithThirdTarget`.
    pub(crate) fn first_slot_targets<'a>(
        &'a self,
        current_targets: &'a [baylee_core::ids::ObjectId],
    ) -> &'a [baylee_core::ids::ObjectId] {
        self.target_restore_frames
            .first()
            .map_or(current_targets, |frame| frame.saved_targets.as_slice())
    }

    pub(crate) const fn set_chosen_subtype(&mut self, subtype: baylee_core::ids::SubtypeId) {
        self.chosen_subtype = Some(subtype);
    }

    pub(crate) const fn set_chosen_color(&mut self, color: baylee_core::mana::ManaColor) {
        self.chosen_color = Some(color);
    }

    pub(crate) fn remembered(&self) -> Option<&[super::remembered::RememberedObject]> {
        self.remembered_objects.as_deref()
    }

    pub(crate) fn remembered_by_ref(
        &self,
        reference: DamageSourceRef,
    ) -> Option<&super::remembered::RememberedObject> {
        self.remembered_objects
            .as_deref()?
            .iter()
            .find(|r| r.reference == reference)
    }

    /// Record explicit picked/chosen object IDs (`Dig`, `PeekAndReveal`,
    /// `ChooseCard`, `ChooseCardEx`) into an enclosing `CaptureAffected`'s
    /// `first` phase so unchosen/bottomed cards are not conflated with kept ones.
    pub(crate) fn note_captured_ids(&mut self, ids: &[ObjectId]) {
        if let Some(super::remembered::CaptureFrame {
            phase: super::remembered::CapturePhase::RunningFirst { explicit_ids, .. },
            ..
        }) = self.capture_frames.last_mut()
        {
            for &id in ids {
                if !explicit_ids.contains(&id) {
                    explicit_ids.push(id);
                }
            }
        }
    }

    pub(crate) fn with_modal_steps(mut self, steps: Vec<ModalTargetStep>) -> Self {
        self.modal_steps = steps;
        self
    }

    pub(crate) fn modal_steps(&self) -> &[ModalTargetStep] {
        &self.modal_steps
    }

    pub(crate) fn take_modal_step(&mut self, left_from_end: usize) -> Option<ModalTargetStep> {
        if self
            .modal_steps
            .first()
            .is_some_and(|step| step.left_from_end == left_from_end)
        {
            Some(self.modal_steps.remove(0))
        } else {
            None
        }
    }

    /// A mana ability has no stack object to retain its cost record.
    pub(crate) fn after_cost(paid: &crate::object::PaidRecord) -> Self {
        Self {
            second_target_players: baylee_core::ids::SeatSet::new(),
            third_targets: smallvec::SmallVec::new(),
            third_target_players: baylee_core::ids::SeatSet::new(),
            modal_steps: Vec::new(),
            followed: paid.source_after_cost.into_iter().collect(),
            reanimation_links: Vec::new(),
            found: Vec::new(),
            inspected_library: None,
            guess: None,
            named: None,
            chosen_subtype: None,
            chosen_color: None,
            chosen_number: None,
            sacrificed: paid.sacrificed_lki,
            sacrifice_stats: SacrificeStats::default(),
            circled_colors: baylee_core::color::ColorSet::EMPTY,
            acquisition: None,
            source_version: None,
            mana_tap_source: None,
            event_player: None,
            event_identity: None,
            for_each_remaining: None,
            destruction_batches: Vec::new(),
            payment_batches: Vec::new(),
            counted: None,
            checkpoint: None,
            last_die_roll: None,
            coins_won: 0,
            repeat_frames: Vec::new(),
            repeat_processes: Vec::new(),
            vote_queue: None,
            branch_frames: Vec::new(),
            target_restore_frames: Vec::new(),
            remembered_objects: None,
            capture_frames: Vec::new(),
            damage_prevented: 0,
            demonstrate_opp: None,
            accumulated_number: 0,
            remembered_players: smallvec::SmallVec::new(),
            queued_player_number_steps: std::collections::VecDeque::new(),
            queued_player_number_saved: None,
        }
    }

    /// Deck-building words belong to the activated ability, even after costs move its source.
    pub(crate) fn with_circled_colors(mut self, colors: baylee_core::color::ColorSet) -> Self {
        self.circled_colors = colors;
        self
    }

    pub(super) fn circled_colors(&self) -> baylee_core::color::ColorSet {
        self.circled_colors
    }

    /// Immediate mana resolutions retain their linked family without a stack object.
    pub(crate) fn with_acquisition(mut self, acquisition: u64) -> Self {
        self.acquisition = Some(acquisition);
        self
    }

    /// Immediate mana abilities have no stack object to retain their source version.
    pub(crate) fn with_source_version(mut self, version: Option<u32>) -> Self {
        self.source_version = version;
        self
    }

    pub(crate) fn with_mana_tap_source(mut self, source: Option<DamageSourceRef>) -> Self {
        self.mana_tap_source = source;
        self
    }
    pub(super) fn mana_tap_source(&self) -> Option<DamageSourceRef> {
        self.mana_tap_source
    }

    pub(crate) fn with_event_player(mut self, player: Option<baylee_core::ids::PlayerId>) -> Self {
        self.event_player = player;
        self
    }

    /// Pre-populate `SubjectContext::found` with exact incarnations captured
    /// by a resolving triggered ability (`Rider::EventFoundObject`).
    pub(crate) fn with_found(mut self, found: Vec<DamageSourceRef>) -> Self {
        self.found = found;
        self
    }
    pub(super) fn set_event_player(&mut self, player: Option<baylee_core::ids::PlayerId>) {
        self.event_player = player;
    }
    pub(super) const fn event_player(&self) -> Option<baylee_core::ids::PlayerId> {
        self.event_player
    }

    /// Preserve the exact event object when an immediate ability has no stack object.
    pub(crate) fn with_event_identity(mut self, identity: Option<(DamageSourceRef, i16)>) -> Self {
        self.event_identity = identity;
        self
    }

    pub(super) fn set_event_identity(&mut self, identity: Option<(DamageSourceRef, i16)>) {
        self.event_identity = identity;
    }

    pub(super) const fn event_identity(&self) -> Option<(DamageSourceRef, i16)> {
        self.event_identity
    }

    /// Whether a `ForEachMatching` count is open (`resolve::for_each`).
    pub(crate) const fn count_open(&self) -> bool {
        self.counted.is_some()
    }

    pub(crate) fn destruction_progress(&self) -> u64 {
        if self.destruction_batches.is_empty() {
            0
        } else {
            crate::state::structural_fingerprint(&self.destruction_batches)
        }
    }

    pub(crate) fn fingerprint(&self) -> u64 {
        let links = crate::state::structural_fingerprint(&(
            &self.reanimation_links,
            self.second_target_players,
        ));
        let mut h = self
            .sacrificed
            .map_or(0, |lki| crate::state::structural_fingerprint(&lki));
        h = h
            .wrapping_mul(31)
            .wrapping_add(u64::from(self.circled_colors.bits()));
        h = h
            .wrapping_mul(31)
            .wrapping_add(crate::state::structural_fingerprint(&self.acquisition));
        h = h
            .wrapping_mul(31)
            .wrapping_add(crate::state::structural_fingerprint(&self.source_version));
        h = h
            .wrapping_mul(31)
            .wrapping_add(crate::state::structural_fingerprint(&self.mana_tap_source));
        h = h
            .wrapping_mul(31)
            .wrapping_add(crate::state::structural_fingerprint(&self.event_player));
        h = h
            .wrapping_mul(31)
            .wrapping_add(crate::state::structural_fingerprint(
                &self.for_each_remaining,
            ));
        h = h
            .wrapping_mul(31)
            .wrapping_add(crate::state::structural_fingerprint(
                &self.destruction_batches,
            ));
        h = h
            .wrapping_mul(31)
            .wrapping_add(crate::state::structural_fingerprint(&self.event_identity));
        h = h
            .wrapping_mul(31)
            .wrapping_add(crate::state::structural_fingerprint(&self.payment_batches));
        if !self.modal_steps.is_empty() {
            h = h
                .wrapping_mul(31)
                .wrapping_add(crate::state::structural_fingerprint(&self.modal_steps));
        }
        h = mix_damage_refs(h, &self.followed);
        h = h.wrapping_mul(31).wrapping_add(self.found.len() as u64);
        h = mix_damage_refs(h, &self.found);
        h = h
            .wrapping_mul(31)
            .wrapping_add(crate::state::structural_fingerprint(&self.guess));
        h = h
            .wrapping_mul(31)
            .wrapping_add(crate::state::structural_fingerprint(&self.named));
        // Only while a count is open, so a resolution without one hashes as it
        // did before there were counts.
        if let Some(count) = &self.counted {
            h = h
                .wrapping_mul(31)
                .wrapping_add(crate::state::structural_fingerprint(count));
        }
        h = self.fingerprint_tail(h);
        // Checkpoints exist only during synchronous resume calls. Every
        // public resume wrapper flushes before returning a held resolution.
        debug_assert!(self.checkpoint.is_none());
        h.wrapping_mul(31).wrapping_add(links)
    }

    fn fingerprint_tail(&self, mut h: u64) -> u64 {
        if let Some(inspected) = &self.inspected_library {
            let digest = crate::state::structural_fingerprint(&("inspected-library", inspected));
            h = h.wrapping_mul(31).wrapping_add(digest);
        }
        // Only once a creature type was chosen during this resolution, keeping
        // existing replay hashes unchanged for all other resolutions.
        if let Some(subtype) = self.chosen_subtype {
            h = h
                .wrapping_mul(31)
                .wrapping_add(crate::state::structural_fingerprint(&(91_u8, subtype)));
        }
        if let Some(color) = self.chosen_color {
            h = h
                .wrapping_mul(31)
                .wrapping_add(crate::state::structural_fingerprint(&(93_u8, color)));
        }
        if let Some(roll) = self.last_die_roll {
            h = h
                .wrapping_mul(31)
                .wrapping_add(crate::state::structural_fingerprint(&(94_u8, roll)));
        }
        if self.coins_won > 0 {
            h = h
                .wrapping_mul(31)
                .wrapping_add(crate::state::structural_fingerprint(&(
                    95_u8,
                    self.coins_won,
                )));
        }
        if !self.repeat_frames.is_empty() {
            h = h
                .wrapping_mul(31)
                .wrapping_add(crate::state::structural_fingerprint(&(
                    97_u8,
                    &self.repeat_frames,
                )));
        }
        if !self.repeat_processes.is_empty() {
            h = h
                .wrapping_mul(31)
                .wrapping_add(crate::state::structural_fingerprint(&(
                    98_u8,
                    &self.repeat_processes,
                )));
        }
        if self.damage_prevented > 0 {
            h = h
                .wrapping_mul(31)
                .wrapping_add(crate::state::structural_fingerprint(&(
                    255_u8,
                    351_u16,
                    self.damage_prevented,
                )));
        }
        if let Some(vq) = &self.vote_queue {
            h = h
                .wrapping_mul(31)
                .wrapping_add(crate::state::structural_fingerprint(&(99_u8, vq)));
        }
        if !self.branch_frames.is_empty() {
            h = h
                .wrapping_mul(31)
                .wrapping_add(crate::state::structural_fingerprint(&(
                    100_u8,
                    &self.branch_frames,
                )));
        }
        if !self.target_restore_frames.is_empty() {
            h = h
                .wrapping_mul(31)
                .wrapping_add(crate::state::structural_fingerprint(&(
                    340_u16,
                    &self.target_restore_frames,
                )));
        }
        if !self.third_targets.is_empty() || !self.third_target_players.is_empty() {
            h = h
                .wrapping_mul(31)
                .wrapping_add(crate::state::structural_fingerprint(&(
                    341_u16,
                    &self.third_targets,
                    self.third_target_players,
                )));
        }
        if self.sacrifice_stats.count > 0 {
            h = h
                .wrapping_mul(31)
                .wrapping_add(crate::state::structural_fingerprint(&(
                    360_u16,
                    self.sacrifice_stats,
                )));
        }
        if let Some(number) = self.chosen_number {
            h = h
                .wrapping_mul(31)
                .wrapping_add(crate::state::structural_fingerprint(&(370_u16, number)));
        }
        self.fingerprint_capture(h)
    }

    fn fingerprint_capture(&self, mut h: u64) -> u64 {
        if !self.capture_frames.is_empty() {
            h = h
                .wrapping_mul(31)
                .wrapping_add(crate::state::structural_fingerprint(&(
                    430_u16,
                    &self.capture_frames,
                )));
        }
        if let Some(remembered) = &self.remembered_objects {
            h = h
                .wrapping_mul(31)
                .wrapping_add(crate::state::structural_fingerprint(&(
                    431_u16,
                    remembered.as_ref(),
                )));
        }
        if let Some((original, opponent)) = self.demonstrate_opp {
            h = h
                .wrapping_mul(31)
                .wrapping_add(crate::state::structural_fingerprint(&(
                    500_u16, original, opponent,
                )));
        }
        if self.accumulated_number > 0 {
            h = h
                .wrapping_mul(31)
                .wrapping_add(crate::state::structural_fingerprint(&(
                    255_u8,
                    553_u16,
                    1_u8,
                    self.accumulated_number,
                )));
        }
        if !self.remembered_players.is_empty() {
            h = h
                .wrapping_mul(31)
                .wrapping_add(crate::state::structural_fingerprint(&(
                    255_u8,
                    553_u16,
                    2_u8,
                    &self.remembered_players,
                )));
        }
        if !self.queued_player_number_steps.is_empty() {
            h = h
                .wrapping_mul(31)
                .wrapping_add(crate::state::structural_fingerprint(&(
                    255_u8,
                    553_u16,
                    3_u8,
                    &self.queued_player_number_steps,
                )));
        }
        if let Some(saved) = self.queued_player_number_saved {
            h = h
                .wrapping_mul(31)
                .wrapping_add(crate::state::structural_fingerprint(&(
                    255_u8, 553_u16, 4_u8, saved,
                )));
        }
        h
    }
}

fn mix_damage_refs(mut h: u64, refs: &[baylee_core::ids::DamageSourceRef]) -> u64 {
    for r in refs {
        h = h.wrapping_mul(31).wrapping_add(u64::from(r.object.slot()));
        h = h
            .wrapping_mul(31)
            .wrapping_add(u64::from(r.object.generation()));
        h = h.wrapping_mul(31).wrapping_add(u64::from(r.version));
    }
    h
}

pub(super) fn acquisition(state: &GameState, res: &Resolution) -> u64 {
    res.subject.acquisition.unwrap_or_else(|| {
        state
            .object(res.on_stack)
            .or_else(|| state.object(res.source))
            .map_or(0, crate::object::GameObject::rules_acquisition)
    })
}

pub(super) fn source(state: &GameState, res: &Resolution) -> Option<DamageSourceRef> {
    res.subject
        .followed
        .iter()
        .rev()
        .find(|r| r.object == res.source)
        .copied()
        .or_else(|| paid_source(state, res))
        .or_else(|| {
            source_version(state, res).map(|version| DamageSourceRef {
                object: res.source,
                version,
            })
        })
        .or_else(|| state.source_identity(res.source))
}

// The payment records a successful public-zone successor, never an inferred
// relationship between whichever versions happen to exist now.
fn paid_source(state: &GameState, res: &Resolution) -> Option<DamageSourceRef> {
    state.object(res.on_stack)?.paid.as_ref()?.source_after_cost
}

/// Moving a departed source can find the precise public destination of its
/// own zone-change trigger (CR 400.7e). Characteristic changes and damage do
/// not use this exception merely because the event context was retained.
pub(super) fn moving_source(state: &GameState, res: &Resolution) -> Option<DamageSourceRef> {
    source(state, res)
        .filter(|reference| is_current(state, *reference))
        .or_else(|| {
            state
                .own_departure_successor(res.on_stack)
                .filter(|reference| reference.object == res.source && is_current(state, *reference))
        })
        .or_else(|| aura_successor(state, res).filter(|reference| is_current(state, *reference)))
        // A delayed trigger watching its own source die (Golden Guardian's
        // "when this creature dies this turn, return it"): the event object
        // is the source's card, as the graveyard has it now.
        .or_else(|| {
            event(state, res).filter(|reference| {
                reference.object == res.source
                    && is_current(state, *reference)
                    && state
                        .object(reference.object)
                        .is_some_and(|o| o.zone == Zone::Graveyard)
            })
        })
}

/// The Aura's graveyard incarnation its departure trigger recorded
/// (`Rider::SourceAuraSuccessor`, CR 400.7f).
fn aura_successor(state: &GameState, res: &Resolution) -> Option<DamageSourceRef> {
    let version = state
        .object(res.on_stack)?
        .riders
        .iter()
        .find_map(|r| match r {
            crate::object::Rider::SourceAuraSuccessor(v) => Some(*v),
            _ => None,
        })?;
    Some(DamageSourceRef {
        object: res.source,
        version,
    })
}

/// Event incarnation, or its successful public successor in this resolution.
pub(crate) fn event(state: &GameState, res: &Resolution) -> Option<DamageSourceRef> {
    let object = res
        .subject
        .event_identity
        .map(|(reference, _)| reference.object)
        .or(res.event_object)?;
    res.subject
        .followed
        .iter()
        .rev()
        .find(|r| r.object == object)
        .copied()
        .or_else(|| {
            super::event_object_identity(state, res)
                .map(|(version, _)| DamageSourceRef { object, version })
        })
        .or_else(|| state.source_identity(object))
}

/// The graveyard incarnation a death trigger is allowed to follow (CR 400.7).
/// Depending on when its identity was captured, a stack object remembers
/// either the battlefield incarnation or its immediate graveyard successor.
/// Only that exact transition is followed: a card which moved on, even back
/// into the same graveyard, is a different object and cannot be reached.
pub(super) fn event_in_graveyard(state: &GameState, res: &Resolution) -> Option<DamageSourceRef> {
    let object = res
        .subject
        .event_identity
        .map(|(reference, _)| reference.object)
        .or(res.event_object)?;
    let before = res
        .subject
        .followed
        .iter()
        .rev()
        .find(|reference| reference.object == object)
        .copied()
        .or_else(|| {
            super::event_object_identity(state, res)
                .map(|(version, _)| DamageSourceRef { object, version })
        })?;
    let current = state.object(object)?;
    let expected_version = match state.source_object(before)?.zone {
        Zone::Battlefield => before.version.wrapping_add(1),
        Zone::Graveyard => before.version,
        _ => return None,
    };
    (current.zone == Zone::Graveyard && current.is_card() && current.version == expected_version)
        .then_some(DamageSourceRef {
            object,
            version: current.version,
        })
}

/// The object an instruction names, retaining the event's incarnation for
/// implicit subjects. An explicit target still uses its resolution snapshot.
pub(super) fn target(
    state: &GameState,
    res: &Resolution,
    object: baylee_core::ids::ObjectId,
) -> Option<DamageSourceRef> {
    res.subject
        .followed
        .iter()
        .rev()
        .find(|r| r.object == object)
        .copied()
        .or_else(|| {
            let stack = state.object(res.on_stack)?;
            if res.event_object != Some(object)
                || (res.targeted
                    && !stack.target_req.is_some_and(|req| {
                        matches!(req.spec, baylee_cards_dsl::TargetSpec::EventObject)
                    }))
            {
                return None;
            }
            // Synthetic untargeted abilities can carry an implicit event
            // subject too. It remains the incarnation that caused the
            // trigger, even when the same card leaves and returns.
            let (version, _) = super::event_object_identity(state, res)?;
            Some(DamageSourceRef { object, version })
        })
        .or_else(|| {
            res.target_lki
                .as_ref()?
                .iter()
                .find(|r| r.id == object)
                .map(|r| DamageSourceRef {
                    object,
                    version: r.version,
                })
        })
        .or_else(|| state.source_identity(object))
}

pub(super) fn this(state: &GameState, res: &Resolution) -> Option<DamageSourceRef> {
    if names_targets(state, res) {
        res.targets
            .first()
            .and_then(|&object| target(state, res, object))
    } else {
        source(state, res)
    }
}

/// Whether "this" in the resolving instruction is its object target rather
/// than its source.
///
/// It is the target when the ability said "target" about an object, and
/// that is read off what was announced, which the stack object keeps, and
/// not off what is still legal: an object target that became illegal is
/// not affected (CR 608.2b), and the source does not stand in for it, even
/// while a player chosen beside it is still a legal target. It is also the
/// target, and so nothing, when the ability said "target" and nothing at
/// all was chosen (an "up to one" left empty: Karn's +1). Only a target that
/// is a player and nothing else (Jinxed Choker's "target opponent gains
/// control of this artifact and puts a charge counter on it") leaves "this"
/// the source.
pub(super) fn names_targets(state: &GameState, res: &Resolution) -> bool {
    if !res.targets.is_empty() {
        return true;
    }
    res.targeted
        && (res.target_players.is_empty()
            || state.object(res.on_stack).is_some_and(|announced| {
                match res
                    .subject
                    .target_restore_frames
                    .last()
                    .map(|f| f.active_group)
                {
                    None => !announced.targets.is_empty(),
                    Some(ActiveTargetGroup::Second) => !announced.second_targets().is_empty(),
                    Some(ActiveTargetGroup::Third) => !announced.third_targets().is_empty(),
                    Some(ActiveTargetGroup::All) => {
                        !announced.targets.is_empty()
                            || !announced.second_targets().is_empty()
                            || !announced.third_targets().is_empty()
                    }
                }
            }))
}
/// The creature the resolving trigger is about besides its event object
/// (`Rider::EventSubject`), as it was when the trigger fired.
pub(super) fn event_subject(state: &GameState, res: &Resolution) -> Option<DamageSourceRef> {
    state
        .object(res.on_stack)?
        .riders
        .iter()
        .find_map(|rider| match rider {
            crate::object::Rider::EventSubject(subject) => Some(*subject),
            _ => None,
        })
}
pub(crate) fn is_current(state: &GameState, reference: DamageSourceRef) -> bool {
    state.object(reference.object).is_some_and(|o| {
        o.version == reference.version && !o.status.contains(crate::object::Status::PHASED_OUT)
    })
}
pub(super) fn on_battlefield(state: &GameState, reference: DamageSourceRef) -> bool {
    state.object(reference.object).is_some_and(|o| {
        o.version == reference.version
            && o.zone == Zone::Battlefield
            && !o.status.contains(crate::object::Status::PHASED_OUT)
    })
}

pub(super) fn before(state: &GameState, res: &Resolution) -> (Vec<DamageSourceRef>, u64) {
    let mut refs: Vec<_> = source(state, res).into_iter().collect();
    if let Some(event) = res.event_object {
        let version = super::event_object_identity(state, res).map(|(v, _)| v);
        if let Some(version) = version {
            refs.push(DamageSourceRef {
                object: event,
                version,
            });
        }
    }
    if let Some(targets) = &res.target_lki {
        refs.extend(targets.iter().map(|t| DamageSourceRef {
            object: t.id,
            version: t.version,
        }));
    }
    refs.extend(res.subject.event_identity.map(|(reference, _)| reference));
    refs.extend(res.subject.followed.iter().copied());
    refs.retain(|r| state.source_identity(r.object) == Some(*r));
    refs.sort_unstable();
    refs.dedup();
    (refs, state.journal.last_seq())
}
/// A successor reached solely through actual public-zone moves of this operation.
pub(crate) fn public_successor(
    state: &GameState,
    original: DamageSourceRef,
    since: u64,
    cause: Cause,
) -> Option<DamageSourceRef> {
    let current = state.source_identity(original.object)?;
    if current == original {
        return None;
    }
    let mut moved = false;
    for entry in state.journal.entries().iter().filter(|e| e.seq > since) {
        if let GameEvent::ZoneChanged {
            object,
            to,
            cause: actual,
            ..
        } = entry.event
            && object == original.object
        {
            if to.is_hidden_by_default() || actual != cause {
                return None;
            }
            moved = true;
        }
    }
    moved.then_some(current)
}
pub(super) fn after(state: &GameState, res: &mut Resolution, before: &(Vec<DamageSourceRef>, u64)) {
    for original in &before.0 {
        let Some(current) = public_successor(state, *original, before.1, Cause::Effect) else {
            continue;
        };
        res.subject.followed.retain(|r| r.object != current.object);
        res.subject.followed.push(current);
    }
}
pub(super) fn begin_resume(state: &GameState, res: &mut Resolution) {
    flush(state, res);
    res.subject.checkpoint = Some(before(state, res));
}
pub(super) fn flush(state: &GameState, res: &mut Resolution) {
    if let Some(before) = res.subject.checkpoint.take() {
        after(state, res, &before);
    }
}
