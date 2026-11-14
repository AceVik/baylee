//! Global authority orchestration. Duel rules live only in owned native workers.
#![forbid(unsafe_code)]

mod artifact;
mod process;
mod queue;
mod verifier;
pub use artifact::Artifact;
pub use queue::{Queue, QueueResult, QueueSummary};
pub use verifier::Verifier;

/// Value-free operational failures. None is a game outcome or permission to award.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// Owner shutdown or cancellation; the owned process has been reaped.
    #[error("Tower verification cancelled")]
    Cancelled,
    /// All locally configured worker slots remain occupied.
    #[error("Tower verifier capacity exhausted")]
    Busy,
    /// Invalid configuration, private framing, process output or artifact mismatch.
    #[error("Tower verifier contract refused")]
    Contract,
    /// A process could not be started, stopped, observed or reaped.
    #[error("Tower verifier process unavailable")]
    Process,
    /// An independently recomputed replay was rejected; never a rules defeat.
    #[error("Tower recorded play refused")]
    Record,
    /// A technical allocation was exhausted; retained evidence remains unresolved.
    #[error("Tower verification budget exhausted")]
    Budget,
    /// Existing safe store errors contain no database messages or private inputs.
    #[error(transparent)]
    Store(#[from] baylee_tower_store::Error),
}
