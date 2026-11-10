//! Pool-wide lints: shapes in the card data that are almost always a
//! mistake, checked against the DSL alone.
//!
//! # Why these are not tests about the engine
//!
//! Everything here reads `CardDef` and nothing else — no `Engine`, no
//! Scryfall payload, no oracle text. That makes them the cheapest tier there
//! is (the whole module runs in milliseconds over 1365 cards) and it fixes
//! what they can prove: they cannot say a card does the *right* thing, only
//! that it is built in a shape that cannot be right.
//!
//! That is a narrower claim than it sounds, and it is worth having because
//! the pool's most expensive bug so far was exactly such a shape. Karn, the
//! Great Creator's `+1` reads "until end of turn, target noncreature
//! artifact becomes a 0/0 artifact creature". It was written with the
//! ability's *target filter* handed to the effect as well, so it animated
//! every noncreature artifact on the table — and then state-based actions
//! put all of them in a graveyard. Nothing in the engine was wrong, the
//! card's header matched its `CardDef`, and no gate had anything to say.
//! [`target_reuse`] is that shape, and it is decidable without playing a
//! single turn.
//!
//! # The shape of every lint here
//!
//! One `#[test]`, sweeping [`crate::all`], collecting *every* offender and
//! failing with the whole list — the shape
//! `no_card_claims_a_keyword_the_engine_ignores` already has, because a
//! sweep that stops at the first card makes a pool-wide problem look like
//! one card's problem.
//!
//! And each lint is a **pure function** over the data, with a unit test that
//! hands it the broken shape and watches it fire. A sweep over 1365 cards
//! that finds nothing is otherwise indistinguishable from one that checks
//! nothing.
//!
//! # The one lint that reads text
//!
//! `no_card_writes_an_enter_trigger_out_by_hand` breaks the rule above, and
//! the reason it has to is the reason it is worth having.
//! `Trigger::ETB` is a `const` holding exactly
//! `Trigger::EntersBattlefield(&Filter::This)`, so the two are the same
//! bytes and no lint reading `CardDef` can tell them apart — the difference
//! exists only in the source, which is the only place it matters. It is the
//! same bargain the DSL's named filters make: a shape with two spellings is
//! a shape with two names, and the cheapest moment to refuse the second one
//! is before it is written a hundredth time.

use crate::dsl::ability::{AbilityDef, SpellMode};
use crate::dsl::cost::{Cost, CostPart};
use crate::dsl::effect::{Effect, ManaSource, ReflexiveEvent, TargetSpec};
use crate::dsl::filter::Filter;
use crate::dsl::static_ability::{Layer, Modifier};
use crate::dsl::{CardDef, Color, ColorSet, FaceDef, ManaColor, ManaCost, TypeSet};

mod branches;
mod costs;
mod faces;
mod layers;
mod shapes;

use branches::*;
use costs::*;
use faces::*;
use layers::*;
use shapes::*;

#[cfg(test)]
mod tests;
