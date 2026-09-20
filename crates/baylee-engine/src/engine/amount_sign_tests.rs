//! The **sign** of an `Amount`, and where a card may put one.
//!
//! `eval::amount` answers a magnitude. The engine consumes one at eighteen
//! sites, and seventeen of them are counting cards to draw, tokens to make or
//! life to gain, where a negative number has nothing to mean — so the sign is
//! not in the evaluated value at all, and `Amount::is_negative` is the
//! separate question. That split is the
//! reason this file exists: a field that reads the magnitude and never asks
//! the second question silently makes every negative amount a bonus.
//!
//! It was four questions before `Amount::Negated` was added. `resolve::counters`
//! spelled `matches!(a, Amount::NegX | Amount::NegXFixed(_))` in three
//! separate closures — one per P/T arm — and `baylee-ai`'s `tactics` had a
//! fourth of its own. Four positive lists over an enum is four chances for a
//! new negative variant to be read as a bonus, and nothing in a test suite
//! reads a sign as a bug: the card resolves, the creature changes size, and
//! only the direction is wrong. The three closures are one
//! `resolve::counters::signed` now, and it is the only caller of
//! `is_negative` on that side.
//!
//! Irradiate, Feeding Frenzy and Wirewood Pride are played in
//! `card_tests::instants` — the cards are where the rule is visible. What is
//! here is the claim no card can make on its own: that **every** `Negated` in
//! the compiled pool sits in a field something reads the sign of.
//!
//! Checked against injected defects rather than trusted for passing.
//! Reverting `signed` to the old `matches!` turns Irradiate red at 8/8 where
//! it prints 4/4 and Feeding Frenzy at 9/9 where it prints 3/3 — the exact
//! shape of the silence above, a shrink handed out as a pump, on a 6/6 that
//! ends up bigger than it started. Wirewood Pride and Aurochs stay green
//! through that injection, which is why the positive cards are separate
//! tests: they share every line of the reader except the sign. And dropping
//! `toughness: Negated(` from the list below turns this test red naming both
//! negatives, at one read `Negated` of the two each of them writes.

/// Every `Amount::Negated` in the pool is in a field that reads the sign.
///
/// `Effect` carries twenty-two `Amount` fields across nineteen variants, and
/// the sign means anything in six of them: `SetPTFilter`, `PumpFilter` and
/// `PumpTarget`, each through `power` and `toughness`, all three reaching
/// `resolve::counters::signed`. The other sixteen are counts — `DrawCards`,
/// `GainLife`, `CreateTokenN`, `PlayerMayPayOr`'s price. A card writing
/// `Negated` into one of those compiles, claims `Coverage::Implemented`,
/// resolves — and draws cards, or charges mana, by the magnitude, with the
/// minus sign thrown away. That is the same silence `ThisObject` had before
/// `resolve::zones::spec_object`, and it is invisible to `xtask validate`
/// for the same reason: the card says exactly the right thing.
///
/// Read off `Debug` rather than a match table over the enum. A table has to
/// be kept in step with `Effect` by hand, and a reader that has gone blind on
/// a variant reports zero and passes; `Debug` prints every field of every
/// nested effect, which is what a granted ability two levels down needs. The
/// floor is the other half of that guard.
///
/// A double negative would be reported here as one unread `Negated` — the
/// inner one — which is a false finding only if a card ever prints one. None
/// does. `Amount::is_negative` answers parity anyway, so the day one appears
/// the rule is right and this list is what needs the sentence.
#[test]
fn every_negated_amount_in_the_pool_sits_in_a_field_that_reads_the_sign() {
    // Exactly the fields `resolve::counters::signed` is reached from, as the
    // `Debug` of the effect prints them. `SetPTFilter`, `PumpFilter` and
    // `PumpTarget` all spell their two the same way, so two entries cover
    // all six.
    const READ: &[&str] = &["power: Negated(", "toughness: Negated("];

    let mut unread = Vec::new();
    let mut seen = 0_usize;
    for def in baylee_cards::all() {
        let text = format!("{def:?}");
        let total = text.matches("Negated(").count();
        if total == 0 {
            continue;
        }
        let read: usize = READ.iter().map(|shape| text.matches(shape).count()).sum();
        seen += total;
        if read < total {
            unread.push(format!(
                "{}: {total} `Negated`, {read} of them in a field that reads the sign",
                def.name()
            ));
        }
    }
    assert!(
        seen >= 4,
        "only {seen} `Negated` found in the whole pool — Irradiate and Feeding \
         Frenzy write two each, so this reader has gone blind and an empty \
         sweep proves nothing"
    );
    assert!(
        unread.is_empty(),
        "{} card(s) negate an amount in a field that reads the magnitude and \
         never asks the sign, so the card hands out the opposite of what it \
         prints. Either read `Amount::is_negative` where that field is \
         evaluated and add the spelling above, or say the sentence a way the \
         DSL already means.\n{}",
        unread.len(),
        unread.join("\n")
    );
}
