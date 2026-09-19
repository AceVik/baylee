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

use baylee_cards::dsl::{AbilityDef, CardDef, CounterKind, Effect, KeywordSet, PartnerKind};

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

/// Every effect list an ability resolves through.
///
/// Exhaustive with no wildcard arm, for the reason `baylee_cards`'s own
/// lints give about `branches`: a new [`AbilityDef`] variant dropping into
/// `_ => Vec::new()` would leave the probe below reading nothing about it
/// and still reporting a number.
fn ability_effects(ability: &'static AbilityDef) -> Vec<&'static [Effect]> {
    match ability {
        AbilityDef::Spell { effects, .. }
        | AbilityDef::Triggered { effects, .. }
        | AbilityDef::Activated { effects, .. }
        | AbilityDef::ActivatedConditional { effects, .. }
        | AbilityDef::SagaChapter { effects, .. }
        | AbilityDef::Loyalty { effects, .. } => vec![effects],
        AbilityDef::ModalSpell { modes } | AbilityDef::ModalTriggered { modes, .. } => {
            modes.iter().map(|mode| mode.effects).collect()
        }
        AbilityDef::Unimplemented
        | AbilityDef::Ward { .. }
        | AbilityDef::Prepared { .. }
        | AbilityDef::Echo { .. }
        | AbilityDef::Static(_)
        | AbilityDef::Replacement(_)
        | AbilityDef::Suspend { .. }
        | AbilityDef::CopyOnEnterUntilEot { .. }
        | AbilityDef::CopyOnEnter { .. } => Vec::new(),
    }
}

/// Every counter an effect list puts on anything, following the nine shapes
/// the DSL nests an effect list inside.
///
/// This one **does** end in a wildcard, because `Effect` has 154 variants and
/// listing them here would be a second copy of that enum rather than a
/// reading of it. That is exactly why the test below counts the effects it
/// visited and holds the count against a floor: a wildcard that quietly
/// swallowed the nesting shapes would report zero counters over a pool full
/// of them, and a number with nothing underneath it is the failure this file
/// exists to prevent.
fn counters_put(effects: &'static [Effect], seen: &mut usize, found: &mut Vec<CounterKind>) {
    for effect in effects {
        *seen += 1;
        match effect {
            Effect::AddCounter { kind, .. } | Effect::AddCounterFilter { kind, .. } => {
                found.push(*kind);
            }
            Effect::Sequence(inner) | Effect::MayDo { effects: inner } => {
                counters_put(inner, seen, found);
            }
            Effect::IfControlGreatestCmc { then, .. }
            | Effect::IfCreaturesDiedAtLeast { then, .. }
            | Effect::IfNoCountersOnSelf { then, .. }
            | Effect::IfNotLostLifeThisTurn { then, .. } => counters_put(then, seen, found),
            Effect::IfEventPowerAtLeast {
                then, otherwise, ..
            }
            | Effect::IfKicked {
                then, otherwise, ..
            } => {
                counters_put(then, seen, found);
                counters_put(otherwise, seen, found);
            }
            _ => {}
        }
    }
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
            // Paid, #73: `a_lore_counter_goes_on_my_own_saga_and_never_the_\
            // opponents` in `baylee-ai`. The probe stays because the test
            // needs a Saga in the pool to be about anything — if Urza's Saga
            // leaves, that test starts proving nothing and this row says so.
            "Contextual counters (the lore half)",
            count(|def| abilities(def).any(|a| matches!(a, AbilityDef::SagaChapter { .. }))),
        ),
        (
            // Suspend is the half of that row the pool can already test:
            // a time counter on a suspended card is a clock the AI wants to
            // run down, and the same counter on vanishing is one it does not.
            //
            // Paid, #73: `a_time_counter_delays_the_suspended_card_that_is_\
            // about_to_cast`. Vanishing is still owed and cannot be written:
            // no card here prints it, and `clock_score` scores that case 0
            // rather than guessing at it.
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

/// The counter-clock rows of #73 are answered by two `baylee-ai` unit tests
/// and by no game, because no card in this pool can be *made* to put a lore
/// or a time counter anywhere: both clocks are advanced by the engine itself
/// (`progress.rs`), never by an effect a player targets.
///
/// So the agent's rule is proven against a constructed view, and this is the
/// tripwire for the day that stops being the whole story: when a card arrives
/// that prints "put a lore counter on target Saga" or "put a time counter on
/// target suspended card", the same decision becomes reachable in a real
/// game and that game is owed as a test.
///
/// The two assertions above the count are what make the zero mean something.
/// `signed` proves the walk reaches real `AddCounter` effects at all — the
/// pool is full of +1/+1 — and `seen` is the population it read, so a walker
/// blinded by a refactor fails here instead of reporting a quiet nought.
#[test]
fn no_pool_card_puts_a_lore_or_time_counter_on_anything() {
    let mut seen = 0;
    let mut kinds = Vec::new();
    for def in baylee_cards::all() {
        for effects in abilities(def).flat_map(ability_effects) {
            counters_put(effects, &mut seen, &mut kinds);
        }
    }
    let clocks = kinds
        .iter()
        .filter(|kind| matches!(kind, CounterKind::Lore | CounterKind::Time))
        .count();
    let signed = kinds
        .iter()
        .filter(|kind| matches!(kind, CounterKind::Plus { .. } | CounterKind::Minus { .. }))
        .count();

    // 2696 on 19.09.2026. The floor is the population and not the number,
    // so a card added or removed is not news and a walker that stopped
    // descending is.
    assert!(
        seen > 2_000,
        "the effect walk visited {seen} effects, which is too few to have \
         read this pool: the nesting shapes it follows have gone stale"
    );
    assert!(
        signed > 0,
        "the walk found no +1/+1 or -1/-1 counter in the whole pool, so it \
         is not reaching `AddCounter` at all and the zero below is empty"
    );
    assert_eq!(
        clocks, 0,
        "{clocks} effect(s) in the pool now put a lore or time counter on \
         something. The counter-clock rule in `tactics::clock_score` is \
         reachable in a real game: write the game, put it beside the two \
         `baylee-ai` unit tests, and delete this test."
    );
}
