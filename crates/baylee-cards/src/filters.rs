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

#[cfg(test)]
mod tests {
    use super::{ANOTHER_ALLY, YOUR_ALLIES, YOUR_ALLY};
    use baylee_cards_dsl::{Filter, f};
    use baylee_core::generated::subtypes::creature;

    /// **Three names, three filters**, which is the defect this file was
    /// made out of. Seven card files had written the Ally tribe out under
    /// four different names, and one of them called it `YOUR_ALLIES` — the
    /// name the constant above already had for a *different* filter, the one
    /// that counts the source itself. The same name meaning two things is
    /// worse than no name at all, and a file of shared constants is only
    /// worth having while no two of them are the same thing under two names.
    #[test]
    fn no_two_of_these_names_are_one_filter() {
        assert_ne!(
            YOUR_ALLIES, YOUR_ALLY,
            "the rally wording counts the source and the tribe does not"
        );
        assert_ne!(YOUR_ALLY, ANOTHER_ALLY, "`Another` is the whole difference");
        assert_ne!(YOUR_ALLIES, ANOTHER_ALLY);
    }

    /// **Noun first, so `f!` and the constant are the same data.**
    ///
    /// `f!` expands to `Filter::And(&[noun, adjectives…])` in written order,
    /// so a constant written adjective first would be the same objects and
    /// different *data* — and a card that reached for the macro instead of
    /// the name would be writing the filter a second time, which is what
    /// this file exists to stop. These were adjective first until the
    /// clauses were reordered wholesale, and nothing held them there
    /// afterwards.
    ///
    /// `YOUR_ALLIES` is deliberately absent: its noun is an `Or`, which is
    /// not a spelling `f!` has, so there is no second way to write it and
    /// nothing for it to drift from.
    #[test]
    fn each_constant_is_what_the_macro_would_have_written() {
        static ADJECTIVE_FIRST: Filter =
            Filter::And(&[Filter::ControlledByYou, Filter::HasSubtype(creature::ALLY)]);

        assert_eq!(
            YOUR_ALLY,
            f!(your Filter::HasSubtype(creature::ALLY)),
            "an Ally you control"
        );
        assert_eq!(
            ANOTHER_ALLY,
            f!(your another Filter::HasSubtype(creature::ALLY)),
            "another Ally you control"
        );
        assert_ne!(
            YOUR_ALLY, ADJECTIVE_FIRST,
            "and the other order is a different filter, not a different style"
        );
    }

    /// The relationship between the two tribe filters, rather than a second
    /// copy of either: **`ANOTHER_ALLY` is `YOUR_ALLY` and one more clause**,
    /// and that clause is the one word a rules bug turns on.
    ///
    /// The Ally rally trigger reads "whenever this creature **or another
    /// Ally** enters under your control", so a card reaching for
    /// `ANOTHER_ALLY` where the printing says the source counts loses its own
    /// arrival — six card files had written that sentence out, and a seventh
    /// writing `Another` instead would have been silent.
    #[test]
    fn another_ally_is_the_tribe_with_one_clause_more() {
        let clauses = |f: &Filter| match f {
            Filter::And(parts) => parts.to_vec(),
            other => panic!("expected a conjunction, got {other:?}"),
        };
        let tribe = clauses(&YOUR_ALLY);
        let another = clauses(&ANOTHER_ALLY);

        assert_eq!(another[..tribe.len()], tribe[..], "the same clauses first");
        assert_eq!(
            &another[tribe.len()..],
            &[Filter::Another],
            "and exactly one more"
        );
        assert!(
            !tribe.contains(&Filter::Another) && !tribe.contains(&Filter::This),
            "the tribe on its own says nothing about the source"
        );
        assert!(
            clauses(&YOUR_ALLIES)
                .iter()
                .any(|c| matches!(c, Filter::Or(parts) if parts.contains(&Filter::This))),
            "and the rally wording says the source counts"
        );
    }
}
