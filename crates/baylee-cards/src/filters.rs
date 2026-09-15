//! Central filter definitions shared by more than one card.
//!
//! The split from [`baylee_cards_dsl::Filter`]'s own constants is where the
//! knowledge lives, not how complicated the filter is: "a creature" is
//! vocabulary the DSL owns and every pool would want, while "an Ally you
//! control" is a fact about *this* card pool and belongs beside
//! [`crate::tokens`], which draws the same line for the same reason.
//!
//! A filter earns a place here by being written twice. One card's own
//! compound filter stays in that card's file, where the oracle text it
//! encodes is one line above it.
//!
//! All three are **noun first**, like every constant on [`Filter`] and like
//! [`f!`](baylee_cards_dsl::f), which is a rule and not a habit: `f!` expands
//! `f!(your Filter::HasSubtype(creature::ALLY))` noun first, so a constant
//! written the other way round would be the same objects and different data,
//! and the card that reached for the macro instead would be writing the
//! filter a second time. They were adjective first until the clauses were
//! reordered wholesale; the cards that changed are named in that commit.

use baylee_cards_dsl::Filter;
use baylee_core::generated::subtypes::creature;

/// "Allies you control", counting the source itself.
///
/// The Ally rally trigger is worded "Whenever this creature or another Ally
/// enters under your control", so the source is deliberately part of the
/// match — six card files had written this out, and a seventh writing
/// `Another` instead would have been a silent rules bug.
pub static YOUR_ALLIES: Filter = Filter::And(&[
    Filter::Or(&[Filter::This, Filter::HasSubtype(creature::ALLY)]),
    Filter::ControlledByYou,
]);

/// "An Ally you control" — the tribe, the source included, nothing else.
///
/// The third of the three, and the one seven card files had written out
/// under four different names: `ALLIES_YOU` four times, `ALLY_YOU` once,
/// `ALLIES_YOU_CONTROL` once, and `YOUR_ALLIES` once — that last one
/// shadowing the constant above it, which is a *different* filter. Which is
/// the whole argument for this file: the same name meaning two things is
/// worse than no name at all.
pub static YOUR_ALLY: Filter =
    Filter::And(&[Filter::HasSubtype(creature::ALLY), Filter::ControlledByYou]);

/// "Another Ally you control" — the same tribe, excluding the source.
pub static ANOTHER_ALLY: Filter = Filter::And(&[
    Filter::HasSubtype(creature::ALLY),
    Filter::ControlledByYou,
    Filter::Another,
]);
