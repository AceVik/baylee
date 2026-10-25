//! Opt-in foundations of the next Playernet v4 input contract.
//!
//! These builders are not wired into archived encoder-5/6 inference, replay
//! conversion or training. They must not be substituted behind an old model's
//! unchanged metadata. Model/export/dataset integration and the remaining
//! information requirements in the live-information audit are still pending.

pub mod actions;
pub mod inventory;
