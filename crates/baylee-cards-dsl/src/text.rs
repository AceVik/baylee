//! Typed words eligible for rules-text changes (CR 612).

use serde::{Deserialize, Serialize};

/// The vocabulary a text-changing effect may replace.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TextWordKind {
    /// White, blue, black, red or green when used as a color word.
    Color,
    /// Plains, Island, Swamp, Mountain or Forest when used as a land type.
    BasicLandType,
}
