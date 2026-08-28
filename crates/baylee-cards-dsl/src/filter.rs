//! Declarative object filters — the targeting/selection algebra.
//!
//! Filters are pure data, composable, and const-constructible, so cards can
//! declare them in `static`s and the engine can evaluate them without any
//! per-card code. `you` in evaluations is the controller of the ability or
//! spell; `this` is its source object.

use crate::KeywordSet;
use baylee_core::color::ColorSet;
use baylee_core::ids::SubtypeId;
use baylee_core::types::{SupertypeSet, TypeSet};

/// A predicate over game objects.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Filter {
    /// Every object matches.
    Any,
    /// The source object itself.
    This,
    /// Anything except the source object ("another").
    Another,
    /// All must match.
    And(&'static [Filter]),
    /// At least one must match.
    Or(&'static [Filter]),
    /// Must not match.
    Not(&'static Filter),
    /// Has all of these types.
    HasType(TypeSet),
    /// Has none of these types.
    LacksType(TypeSet),
    /// Has all of these supertypes.
    HasSupertype(SupertypeSet),
    /// Has this subtype.
    HasSubtype(SubtypeId),
    /// Has at least one of these colors.
    HasColor(ColorSet),
    /// Is colorless.
    IsColorless,
    /// Controlled by `you`.
    ControlledByYou,
    /// Controlled by an opponent of `you`.
    ControlledByOpponent,
    /// Owned by `you`.
    OwnedByYou,
    /// Currently tapped.
    Tapped,
    /// Currently untapped.
    Untapped,
    /// Has this keyword.
    HasKeyword(KeywordSet),
    /// Converted mana cost at most N.
    CmcAtMost(u32),
    /// Converted mana cost at least N.
    CmcAtLeast(u32),
}

impl Filter {
    // Note: compose filters inline (`Filter::And(&[A, B])`) — in `static`
    // context the slice promotes to `'static` automatically.
}
