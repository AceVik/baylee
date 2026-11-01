/// Which of the four factorized policy heads answers a decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum HeadKind4 {
    /// Choose one candidate index `0..candidates.len()`.
    SingleChoice = 0,
    /// Independent binary inclusion over `0..candidates.len()` (or sequential multi-pick).
    SubsetPick = 1,
    /// Choose a pair `(a, b)` or multi-pair assignment (`DeclareBlockers`, multi-defender attacks).
    PairPick = 2,
    /// Scalar integer/bucket choice (`AnnounceX`, `ChooseNumber`, `DivideValues`).
    ScalarValue = 3,
}

/// All `Pending` variants mapped to a stable index `0..28` for the
/// [`CoverageMatrix4`] and global conditioning.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
pub enum PendingKind4 {
    /// `Pending::Priority`
    Priority = 0,
    /// `Pending::ChooseManaAbility`
    ChooseManaAbility = 1,
    /// `Pending::ChooseDamageSource`
    ChooseDamageSource = 2,
    /// `Pending::ChooseDamageEffect`
    ChooseDamageEffect = 3,
    /// `Pending::AllocatePrevention`
    AllocatePrevention = 4,
    /// `Pending::ChooseAttackers`
    DeclareAttackers = 5,
    /// `Pending::ChooseBlockers`
    DeclareBlockers = 6,
    /// `Pending::ChooseCards`
    ChooseCards = 7,
    /// `Pending::ChooseTargets`
    ChooseTargets = 8,
    /// `Pending::Arrange`
    Arrange = 9,
    /// `Pending::LegendChoice`
    LegendChoice = 10,
    /// `Pending::Mulligan`
    Mulligan = 11,
    /// `Pending::MulliganBottom`
    MulliganBottom = 12,
    /// `Pending::DiscardChoice`
    DiscardChoice = 13,
    /// `Pending::YesNo`
    YesNo = 14,
    /// `Pending::ChooseCastMode`
    ChooseCastMode = 15,
    /// `Pending::ChooseNumber`
    ChooseNumber = 16,
    /// `Pending::ChooseColor`
    ChooseColor = 17,
    /// `Pending::ChooseSubtype`
    ChooseSubtype = 18,
    /// `Pending::ChooseCardName`
    ChooseCardName = 19,
    /// `Pending::ChoosePlayer`
    ChoosePlayer = 20,
    /// `Pending::ChoosePile`
    ChoosePile = 21,
    /// `Pending::GameOver`
    GameOver = 27,
}

impl PendingKind4 {
    /// Total number of `PendingKind4` buckets.
    pub const COUNT: usize = 28;

    /// Maps any [`Pending`] to its [`PendingKind4`].
    #[must_use]
    pub const fn of(pending: &Pending) -> Self {
        match pending {
            Pending::Priority { .. } => Self::Priority,
            Pending::ChooseManaAbility { .. } => Self::ChooseManaAbility,
            Pending::ChooseDamageSource { .. } => Self::ChooseDamageSource,
            Pending::ChooseDamageEffect { .. } => Self::ChooseDamageEffect,
            Pending::AllocatePrevention { .. } => Self::AllocatePrevention,
            Pending::ChooseAttackers { .. } => Self::DeclareAttackers,
            Pending::ChooseBlockers { .. } => Self::DeclareBlockers,
            Pending::ChooseCards { .. } => Self::ChooseCards,
            Pending::ChooseTargets { .. } => Self::ChooseTargets,
            Pending::Arrange { .. } => Self::Arrange,
            Pending::LegendChoice { .. } => Self::LegendChoice,
            Pending::Mulligan { .. } => Self::Mulligan,
            Pending::MulliganBottom { .. } => Self::MulliganBottom,
            Pending::DiscardChoice { .. } => Self::DiscardChoice,
            Pending::YesNo { .. } => Self::YesNo,
            Pending::ChooseCastMode { .. } => Self::ChooseCastMode,
            Pending::ChooseNumber { .. } => Self::ChooseNumber,
            Pending::ChooseColor { .. } => Self::ChooseColor,
            // A card type is a type choice too; a bucket of its own would change
            // `COUNT` and every trained shape that reads it.
            Pending::ChooseSubtype { .. } | Pending::ChooseCardType { .. } => Self::ChooseSubtype,
            Pending::ChooseCardName { .. } => Self::ChooseCardName,
            Pending::ChoosePlayer { .. } => Self::ChoosePlayer,
            Pending::ChoosePile { .. } => Self::ChoosePile,
            Pending::GameOver(_) => Self::GameOver,
        }
    }
}

/// Who produced a recorded decision step (`StepProvenance4`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum SourceKind4 {
    /// Answered directly by the neural policy (`Playernet v4`).
    Net = 0,
    /// Answered by the house heuristic agent (teacher or fallback).
    House = 1,
    /// Forced single-option decision (`<= 1` legal choice).
    Forced = 2,
    /// Answered by belief-consistent lookahead search over `Playernet v4`.
    Search = 3,
    /// An ordinary external socket answer; detailed origin is separately verified.
    /// Used only by explicit BC conversion, never neural behavior probabilities.
    External = 4,
}

/// Provenance metadata stored with every decision step in v4 datasets and RL rollouts.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct StepProvenance4 {
    /// Who produced the action.
    pub source: SourceKind4,
    /// Sampling temperature `tau` used at the decision (`0.0` for greedy/teacher).
    pub temperature: f32,
    /// Number of latent thinking iterations `K` executed (`1..=8`).
    pub thinking_steps: u8,
    /// Exact conditional log-probability under the behavior policy at rollout time.
    pub behavior_logprob: f32,
    /// Whether `behavior_logprob` was recorded by an active neural rollout policy (`P0-2`).
    #[serde(default)]
    pub has_logprob: bool,
}

impl Default for StepProvenance4 {
    fn default() -> Self {
        Self {
            source: SourceKind4::House,
            temperature: 0.0,
            thinking_steps: 1,
            behavior_logprob: 0.0,
            has_logprob: false,
        }
    }
}

/// Coverage matrix tracking per-`PendingKind4` counts of `Net`, `House`, `Forced`,
/// `Search`, and explicit `External` answers across a run or arena evaluation.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CoverageMatrix4 {
    /// Counts `[pending_kind][source_kind]`.
    pub counts: [[u64; 5]; PendingKind4::COUNT],
}

impl CoverageMatrix4 {
    /// Records one answered decision.
    pub fn record(&mut self, kind: PendingKind4, source: SourceKind4) {
        self.counts[kind as usize][source as usize] += 1;
    }

    /// Total non-forced decisions answered by the neural net (`Net` + `Search`).
    #[must_use]
    pub fn neural_decisions(&self) -> u64 {
        self.counts.iter().map(|r| r[0] + r[3]).sum()
    }

    /// Total non-forced decisions that fell back to `House`.
    #[must_use]
    pub fn fallback_decisions(&self) -> u64 {
        self.counts.iter().map(|r| r[1]).sum()
    }
}

/// One encoded action candidate (`Candidate4`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Candidate4 {
    /// Numeric candidate features (`48` floats).
    pub feat: Vec<f32>,
    /// Referenced entity index in `EncodedState4` (`-1` if none).
    pub entity_idx: i16,
    /// Referenced relative seat index (`0..7`, `-1` if none).
    pub rel_seat_idx: i8,
    /// Referenced `CardIndex` (`UNKNOWN_CARD` if none; zero is a valid card).
    pub card_id: u32,
    /// Referenced ability index (`-1` if none); retains the full u32 range.
    pub ability_idx: i64,
    /// Additional exact face/mode semantics supplied by a public offer.
    pub extra_rules: Vec<crate::cardstructure::refs::RuleRef>,
    /// Concrete `PlayerAction` when `head == SingleChoice` (or the atomic item
    /// meaning for `SubsetPick`).
    pub action: Option<PlayerAction>,
    /// Atomic object ID for `SubsetPick`.
    pub object_id: Option<ObjectId>,
    /// Atomic player ID for `SubsetPick` (`ChooseTargets`).
    pub player_id: Option<PlayerId>,
    /// Atomic mode/color index for `SubsetPick`.
    pub index_id: Option<u8>,
    /// Atomic defender target for `ChooseAttackers` (`P1-8`).
    #[serde(default)]
    pub defender: Option<Defender>,
    /// All member `ObjectId`s when a `SingleChoice` candidate represents a deduplicated permanent group.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub group_object_ids: Vec<ObjectId>,
}

impl Default for Candidate4 {
    fn default() -> Self {
        Self {
            feat: vec![0.0; CAND_FEAT_WIDTH],
            entity_idx: -1,
            rel_seat_idx: -1,
            card_id: UNKNOWN_CARD,
            ability_idx: -1,
            extra_rules: Vec::new(),
            action: None,
            object_id: None,
            player_id: None,
            index_id: None,
            defender: None,
            group_object_ids: Vec::new(),
        }
    }
}

impl Candidate4 {
    /// Explicit named face, or 255 to use the referenced entity's rules face.
    #[must_use]
    pub const fn named_face(&self) -> u8 {
        match self.action {
            Some(PlayerAction::ChooseCardName { face, .. }) => face,
            _ => 255,
        }
    }

    /// Card structure for this action, including cards absent from the table.
    #[must_use]
    pub fn dsl_summary(&self, state: &EncodedState4) -> [f32; CARD_DSL_WIDTH] {
        let face = self.named_face();
        if face == 255 && self.entity_idx >= 0 {
            return state.entity_dsl_summary(self.entity_idx as usize);
        }
        if self.card_id == UNKNOWN_CARD {
            return [0.0; CARD_DSL_WIDTH];
        }
        baylee_cards::by_index(CardIndex::new(self.card_id)).map_or([0.0; CARD_DSL_WIDTH], |def| {
            encode_card_face(def, if face == 255 { 0 } else { face as usize }).summary
        })
    }
}

/// Encoded legal candidates for one decision (`EncodedCandidates4`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EncodedCandidates4 {
    /// Policy head kind (`0..4`).
    pub head: HeadKind4,
    /// `Pending` variant kind (`0..28`).
    pub pending_kind: PendingKind4,
    /// Minimum required selections for `SubsetPick` (or min value for `ScalarValue`).
    pub min_choices: u32,
    /// Maximum allowed selections for `SubsetPick` (or max value for `ScalarValue`).
    pub max_choices: u32,
    /// Candidate list (`0..=MAX_CANDIDATES`).
    pub candidates: Vec<Candidate4>,
    /// Legal `(src_cand_or_entity, dst_cand_or_entity)` pairs for `PairPick`.
    pub pairs: Vec<[i16; 2]>,
    /// Underlying pair actions (`(blocker, attacker)` or `(attacker, defender)`).
    pub pair_objects: Vec<(ObjectId, ObjectId)>,
}

/// Target label for supervised training or RL log-prob evaluation (`TeacherTarget4`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TeacherTarget4 {
    /// Chosen candidate index for `SingleChoice` (`0..candidates.len()`).
    pub single_idx: u16,
    /// Binary inclusion mask over `candidates` (`SubsetPick`) or `pairs` (`PairPick`).
    pub multi_mask: Vec<u8>,
    /// Binary mask (`1` = free Bernoulli decision, `0` = forced/forbidden by constraints, `P0-3`).
    #[serde(default)]
    pub free_mask: Vec<u8>,
    /// Ordered sequence of visited/chosen indices (`SubsetPick` / `PairPick` trace).
    pub pick_sequence: Vec<u16>,
    /// Normalized scalar target in `[0.0, 1.0]` for `ScalarValue`.
    pub scalar_norm: f32,
}

impl Default for TeacherTarget4 {
    fn default() -> Self {
        Self {
            single_idx: 0,
            multi_mask: Vec::new(),
            free_mask: Vec::new(),
            pick_sequence: Vec::new(),
            scalar_norm: 0.0,
        }
    }
}
