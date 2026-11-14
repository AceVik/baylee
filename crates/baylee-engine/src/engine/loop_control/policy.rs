//! Statically selected loop control; the disabled implementation is zero-sized.
use baylee_core::preset::LoopPolicy;

mod sealed {
    pub trait Detection {}
    pub trait Segment {}
}

/// A statically dispatched segment watch. Only engine-owned implementations exist.
pub trait LoopSegment: sealed::Segment + Default {
    /// Obtain a signature only when this compiled watch needs one.
    fn observe(&mut self, signature: impl FnOnce() -> u64) -> Option<u64>;
}

/// The loop capability compiled into an engine instance.
///
/// Sealed deliberately: extensions cannot substitute rule adjudication. Setup
/// checks the preset against the type, and checkpoints retain the same type.
pub trait LoopDetection:
    sealed::Detection + Clone + Default + std::fmt::Debug + std::hash::Hash + Send + Sync + 'static
{
    /// Fresh watch for a decision-free segment.
    type Segment: LoopSegment;
    /// Whether this compiled implementation accepts the frozen table policy.
    fn supports(policy: LoopPolicy) -> bool;
    /// Whether a detected loop is currently suppressing triggers.
    fn is_breaking(&self) -> bool;
    /// Finish suppression after the stack and trigger queue have drained.
    fn clear_break_if(&mut self, drained: impl FnOnce() -> bool);
    /// Observe a player answer, lazily resetting or sampling its state.
    fn observe_action(
        &mut self,
        reset: impl FnOnce() -> bool,
        signature: impl FnOnce() -> u64,
    ) -> Option<u64>;
    /// Record a detection, returning break/draw or no detection for the off type.
    fn detected(&mut self, policy: LoopPolicy) -> Option<bool>;
    /// Diagnostic number of broken loops.
    fn broken_count(&self) -> u32;
    /// Whether a fair simulation can copy this state without hidden signatures.
    #[cfg(any(test, feature = "fork"))]
    fn allows_fork(&self) -> bool;
    /// Preserve the historical enabled fingerprint fields for refusal auditing.
    #[cfg(any(test, feature = "fuzz"))]
    fn fingerprint_fields(&self) -> [(&'static str, String); 3];
}

/// Repetition detection and the existing break/draw adjudication.
#[derive(Clone, Debug, Default, Hash)]
pub struct DetectLoops {
    pub(in crate::engine) breaking_loop: bool,
    pub(in crate::engine) action_loops: crate::loops::LoopWatch,
    pub(in crate::engine) loops_broken: u32,
}

impl sealed::Detection for DetectLoops {}
impl sealed::Segment for crate::loops::LoopWatch {}

impl LoopSegment for crate::loops::LoopWatch {
    fn observe(&mut self, signature: impl FnOnce() -> u64) -> Option<u64> {
        let sampled = self.wants_sample().then(signature);
        self.step(sampled)
    }
}

impl LoopDetection for DetectLoops {
    type Segment = crate::loops::LoopWatch;

    fn supports(policy: LoopPolicy) -> bool {
        matches!(
            policy,
            LoopPolicy::RunOnceThenBreak | LoopPolicy::CompRulesDraw
        )
    }

    fn is_breaking(&self) -> bool {
        self.breaking_loop
    }

    fn clear_break_if(&mut self, drained: impl FnOnce() -> bool) {
        if self.breaking_loop && drained() {
            self.breaking_loop = false;
        }
    }

    fn observe_action(
        &mut self,
        reset: impl FnOnce() -> bool,
        signature: impl FnOnce() -> u64,
    ) -> Option<u64> {
        if reset() {
            self.action_loops = Self::Segment::default();
        }
        self.action_loops.observe(signature)
    }

    fn detected(&mut self, policy: LoopPolicy) -> Option<bool> {
        let break_it = policy == LoopPolicy::RunOnceThenBreak && !self.breaking_loop;
        self.action_loops = Self::Segment::default();
        if break_it {
            self.loops_broken += 1;
            self.breaking_loop = true;
        }
        Some(break_it)
    }

    fn broken_count(&self) -> u32 {
        self.loops_broken
    }

    #[cfg(any(test, feature = "fork"))]
    fn allows_fork(&self) -> bool {
        // Exhaustive: a new field requires an explicit hidden-information decision.
        let Self {
            breaking_loop,
            action_loops,
            loops_broken: _,
        } = self;
        !*breaking_loop && !action_loops.holds_signature()
    }

    #[cfg(any(test, feature = "fuzz"))]
    fn fingerprint_fields(&self) -> [(&'static str, String); 3] {
        [
            ("breaking_loop", format!("{:?}", self.breaking_loop)),
            ("action_loops", format!("{:?}", self.action_loops)),
            ("loops_broken", format!("{:?}", self.loops_broken)),
        ]
    }
}

/// No detector, counters or saved signature, even in an unoptimized build.
///
/// The closures are never called. Ordinary game/replay hashing and numeric
/// capacity checks remain independent of this capability.
#[derive(Clone, Copy, Debug, Default, Hash)]
pub struct NoLoopDetection;

impl sealed::Detection for NoLoopDetection {}
impl sealed::Segment for NoLoopDetection {}

impl LoopSegment for NoLoopDetection {
    fn observe(&mut self, _: impl FnOnce() -> u64) -> Option<u64> {
        None
    }
}

impl LoopDetection for NoLoopDetection {
    type Segment = Self;

    fn supports(policy: LoopPolicy) -> bool {
        policy == LoopPolicy::Off
    }

    fn is_breaking(&self) -> bool {
        false
    }

    fn clear_break_if(&mut self, _: impl FnOnce() -> bool) {}

    fn observe_action(&mut self, _: impl FnOnce() -> bool, _: impl FnOnce() -> u64) -> Option<u64> {
        None
    }

    fn detected(&mut self, _: LoopPolicy) -> Option<bool> {
        None
    }

    fn broken_count(&self) -> u32 {
        0
    }

    #[cfg(any(test, feature = "fork"))]
    fn allows_fork(&self) -> bool {
        true
    }

    #[cfg(any(test, feature = "fuzz"))]
    fn fingerprint_fields(&self) -> [(&'static str, String); 3] {
        [
            ("breaking_loop", "false".into()),
            ("action_loops", "disabled".into()),
            ("loops_broken", "0".into()),
        ]
    }
}
