//! The coverage table in `docs/ai-coverage-todo.md`, as tripwires.
//!
//! That table lists the AI tests this pool cannot carry yet, one row per
//! mechanic. As prose it is a second truth beside the code: nobody reads it
//! and the compiler does not check it — which is the argument that turned
//! `data/card-index.tsv` into a compiled table, and the reason
//! `no_card_claims_a_keyword_the_engine_ignores` is a test rather than a
//! paragraph. A row that only a person can notice is a row nobody notices.
//!
//! So every row asks the **compiled pool** whether its precondition has
//! arrived, and the failure message says what is then owed. Two shapes, and
//! the difference between them is the whole point:
//!
//! - A mechanic the pool cannot yet print asserts **zero**. It is green
//!   today and fails on the day somebody implements that mechanic, naming
//!   the test that has become writable. This is the "TODO test" in the only
//!   form that works here: an `#[ignore]` never runs, and a green test that
//!   asserts nothing is worse than none at all.
//! - A mechanic the pool already prints asserts **more than zero**. It never
//!   fires on growth — seven planeswalkers becoming nine is not news — but it
//!   records in code that the row is owed *now* rather than later, and it
//!   fails if the mechanic ever leaves the pool and quietly takes the debt
//!   with it.
//!
//! The pool is read through `baylee_cards::all()` and never by grepping the
//! card files. Scouting this table with a regular expression first reported
//! **no** two-faced cards, where the compiled pool has 116: a textual probe
//! answers a question it cannot see, which is the same fault `knob` exists
//! to prevent in `xtask`. The same scouting pass then missed the one card in
//! the pool that *is* a Partner commander, because `partner` is a field on
//! `CardDef` and not a bit in `KeywordSet`: the first run of this test is
//! what found Sakashima of a Thousand Faces, and it found it by asking the
//! type rather than the spelling.
//!
//! Two rows of the table have no probe here, deliberately. Proliferate and
//! poison counters are not a mechanic the pool is missing, they are words
//! the DSL cannot say at all, so the thing that would have to change first
//! is `Effect` rather than a card. Their tripwire is the vocabulary, and a
//! test over the pool would assert zero against a population that can never
//! be anything else.

use baylee_cards::dsl::{AbilityDef, CardDef, KeywordSet, PartnerKind};

/// Every ability a card carries, card-level and on either face.
///
/// Both, because the split is a layout detail: a two-faced card puts its
/// abilities on the faces and a single-faced one may use either list, so a
/// probe reading one of them alone measures the layout instead of the card.
fn abilities(def: &'static CardDef) -> impl Iterator<Item = &'static AbilityDef> {
    def.abilities
        .iter()
        .chain(def.faces.iter().flat_map(|face| face.abilities.iter()))
}

fn count(probe: impl Fn(&'static CardDef) -> bool) -> usize {
    baylee_cards::all().filter(|def| probe(def)).count()
}

/// The pool prints none of these, so the AI test each one would need cannot
/// be written against a real card yet. When one of these fails, the mechanic
/// has arrived and the named row of `docs/ai-coverage-todo.md` is due.
#[test]
fn a_mechanic_the_pool_cannot_print_yet_has_no_ai_test_to_write() {
    let rows: [(&str, &str, usize); 3] = [
        (
            "Commander pair rules (every pairing but plain Partner)",
            "a game pairing the commander kinds that are not plain Partner, \
             where each one's tax and combat damage stay separate and a \
             partner's damage never counts toward the other's 21",
            // Written as an exclusion rather than a list of the four, so a
            // pairing added to `PartnerKind` later is caught by this test
            // instead of being silently left out of it.
            count(|def| !matches!(def.partner, PartnerKind::None | PartnerKind::Partner)),
        ),
        (
            "Proliferate and replacement effects (the poison half)",
            "an AI decision that treats poison counters as hostile and \
             +1/+1 counters as friendly, rather than assigning one sign to \
             every counter kind",
            count(|def| {
                def.all_keywords()
                    .contains(KeywordSet::INFECT.union(KeywordSet::WITHER))
            }),
        ),
        (
            "Contextual counters (the −1/−1 payoff)",
            "a decision where a counter is good on one card and bad on the \
             next: persist wants the creature to die once, undying does not \
             want it shrunk first",
            count(|def| {
                def.all_keywords()
                    .contains(KeywordSet::PERSIST.union(KeywordSet::UNDYING))
            }),
        ),
    ];

    for (row, owed, found) in rows {
        assert_eq!(
            found, 0,
            "{found} card(s) in the pool now reach `{row}` in \
             docs/ai-coverage-todo.md. That row is no longer a TODO: write \
             {owed}, then delete this entry. A mechanic that arrives \
             without its AI test is a strategy nobody measured."
        );
    }
}

/// The pool already prints these, so the row is owed **now**. The assertion
/// is "more than none" and not an exact number on purpose: a row does not
/// stop being due because a second card joined it, and pinning the count
/// would train whoever adds a card to bump a number instead of writing a
/// test.
#[test]
fn a_mechanic_the_pool_already_prints_is_owed_now_and_not_later() {
    let rows: [(&str, usize); 9] = [
        (
            "Commander pair rules (plain Partner)",
            count(|def| matches!(def.partner, PartnerKind::Partner)),
        ),
        (
            "Planeswalker survival",
            count(|def| def.faces.iter().any(|face| face.loyalty.is_some())),
        ),
        (
            "Modal and multi-target effects",
            count(|def| {
                abilities(def).any(|a| {
                    matches!(
                        a,
                        AbilityDef::ModalSpell { .. } | AbilityDef::ModalTriggered { .. }
                    )
                })
            }),
        ),
        (
            "Transformation and alternate zones (a second face)",
            count(|def| def.faces.len() >= 2),
        ),
        (
            "Transformation and alternate zones (adventure and disturb)",
            count(|def| def.faces.iter().any(|face| face.adventure || face.disturb)),
        ),
        (
            "Variable costs (convoke and delve)",
            count(|def| def.faces.iter().any(|face| face.convoke || face.delve)),
        ),
        (
            "Contextual counters (the lore half)",
            count(|def| abilities(def).any(|a| matches!(a, AbilityDef::SagaChapter { .. }))),
        ),
        (
            // Suspend is the half of that row the pool can already test:
            // a time counter on a suspended card is a clock the AI wants to
            // run down, and the same counter on vanishing is one it does not.
            "Contextual counters (the suspend half)",
            count(|def| abilities(def).any(|a| matches!(a, AbilityDef::Suspend { .. }))),
        ),
        (
            "Stack strategy (ward and taxes)",
            count(|def| abilities(def).any(|a| matches!(a, AbilityDef::Ward { .. }))),
        ),
    ];

    for (row, found) in rows {
        assert!(
            found > 0,
            "no card in the pool reaches `{row}` in docs/ai-coverage-todo.md \
             any more. Either the mechanic left the pool — then move this \
             entry back to the tripwire test above, so its return is \
             noticed — or this probe stopped matching, which is the failure \
             the compiled pool is read for."
        );
    }
}
