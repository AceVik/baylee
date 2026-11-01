use super::{RulesFace, TokenAbility};
use serde::{Deserialize, Serialize};

/// One entry of a composed public ability list. Runtime indices need not be
/// indices in the object's primary copied face (CR 707.9a).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AbilitySource {
    /// Index used by the legal activation offer.
    pub index: u32,
    /// Printed origin, independent of the physical or primary copied card.
    pub rules: Option<RulesFace>,
    /// Index of the supplying clause in that printed face.
    pub ability: u32,
    /// Registry token origin, where no printed card supplies the clause.
    pub token: Option<TokenAbility>,
    /// Quoted ability inside the supplying clause's copy modifiers.
    pub copy_modifier: Option<u64>,
}
