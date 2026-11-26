//! State-transition effect operands (`Effect::SetState`, CR 701.27, CR 708, CR 710).

use crate::filter::Filter;
use baylee_core::ids::SubtypeId;
use baylee_core::types::TypeSet;

/// Which object(s) [`crate::Effect::SetState`] acts on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StateSubject {
    /// The resolving ability's source permanent (`Defined$ Self`).
    Source,
    /// Every legal object target in the ability's first target slot.
    Targets,
    /// The triggering event's object (`TriggeredCard`, `TriggeredSourceLKICopy`).
    EventObject,
    /// Object(s) selected by an enclosing `ChooseCardEx` / `SeparateIntoTwoPiles` (`res.found`).
    Found,
    /// Objects remembered by an enclosing [`crate::Effect::CaptureAffected`]
    /// (`res.subject.remembered_objects`), falling back to `res.subject.found`
    /// when no capture frame is active (`Defined$ Remembered`).
    Remembered,
    /// Every permanent on the battlefield matching this filter at resolution.
    BattlefieldFilter(&'static Filter),
    /// Every card currently exiled linked to the resolving source (`Defined$ ExiledWithSource`).
    LinkedExile,
}

/// Characteristics of a permanent turned face down by an effect (CR 708.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FaceDownSpec {
    /// Card types of the face-down permanent (default: `TypeSet::CREATURE`).
    pub types: TypeSet,
    /// Subtypes of the face-down permanent (default: `&[]`, e.g. Cyberman or Horror).
    pub subtypes: &'static [SubtypeId],
    /// Power of the face-down permanent (default: `Some(2)`).
    pub power: Option<i16>,
    /// Toughness of the face-down permanent (default: `Some(2)`).
    pub toughness: Option<i16>,
}

impl FaceDownSpec {
    /// Standard CR 708.2 face-down 2/2 colorless nameless creature with no subtypes or abilities.
    pub const DEFAULT: Self = Self {
        types: TypeSet::CREATURE,
        subtypes: &[],
        power: Some(2),
        toughness: Some(2),
    };
}

impl Default for FaceDownSpec {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Orientation / face transition performed by [`crate::Effect::SetState`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StateTransition {
    /// Flip an unflipped Kamigawa flip permanent (CR 710.3).
    Flip,
    /// Transform a transforming double-faced permanent or double-faced token (CR 701.27a).
    Transform,
    /// Turn a face-down permanent or face-down exiled card face up (CR 708.8, CR 701.40g).
    TurnFaceUp,
    /// Reveal a face-down permanent; if its face-up characteristics match `filter`,
    /// turn it face up (asking `you` first when `optional` is `true`, *Hauntwoods Shrieker*).
    RevealAndTurnFaceUpIf {
        /// Filter evaluated against the permanent's face-up characteristics.
        filter: &'static Filter,
        /// Whether turning the matching permanent face up is optional ("you may").
        optional: bool,
    },
    /// Turn a face-up non-DFC permanent face down with the given characteristics (CR 708.2, CR 712.16).
    TurnFaceDown(FaceDownSpec),
}
