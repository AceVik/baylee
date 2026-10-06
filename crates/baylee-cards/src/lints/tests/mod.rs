use super::*;
use crate::dsl::effect::TargetReq;
use crate::dsl::static_ability::{Duration, Layer, Modifier};

mod abilities;
mod costs;
mod entries;
mod faces;
mod targets;

/// The filter Karn's `+1` targeted with, and then swept the board with.
static NONCREATURE_ARTIFACT: Filter = Filter::And(&[
    Filter::ARTIFACT,
    Filter::LacksType(baylee_core::types::TypeSet::CREATURE),
]);

/// Animating every artifact on the table: the bug, as written.
static SWEEP: [Effect; 1] = [Effect::CreateContinuousEffect {
    layer: Layer::Type,
    filter: &NONCREATURE_ARTIFACT,
    modifier: Modifier::AddType(baylee_core::types::TypeSet::CREATURE),
    duration: Duration::UntilEndOfTurn,
}];

/// The same effect pointed at the target, which is the fix.
static ON_THE_TARGET: [Effect; 1] = [Effect::CreateContinuousEffect {
    layer: Layer::Type,
    filter: &Filter::This,
    modifier: Modifier::AddType(baylee_core::types::TypeSet::CREATURE),
    duration: Duration::UntilEndOfTurn,
}];

/// A wrath: a sweep with nothing targeted.
static WRATH: [Effect; 1] = [Effect::destroy_all(&Filter::CREATURE)];

/// A sweep beside a target, of two different kinds.
static OTHER_SWEEP: [Effect; 1] = [Effect::destroy_all(&Filter::LAND)];
