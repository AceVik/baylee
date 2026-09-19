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

/// Every counter an effect list puts on anything.
///
/// The descent is [`Effect::walk`]'s, which is the crate that owns `Effect`
/// answering which effects carry another one. This file used to answer it
/// here, and was short by the two `&'static Effect` singles — so a counter
/// behind "unless you pay" was invisible, and `seen` was 2696 where it is
/// now 2731.
fn counters_put(effects: &'static [Effect], seen: &mut usize, found: &mut Vec<CounterKind>) {
    Effect::walk(effects, seen, &mut |effect| {
        if let Effect::AddCounter { kind, .. } | Effect::AddCounterFilter { kind, .. } = effect {
            found.push(*kind);
        }
    });
}

/// Every effect the whole pool reaches, and how many were visited.
fn pool_effects() -> (Vec<&'static Effect>, usize) {
    let mut seen = 0;
    let mut all = Vec::new();
    for def in baylee_cards::all() {
        for effects in abilities(def).flat_map(ability_effects) {
            Effect::walk(effects, &mut seen, &mut |effect| all.push(effect));
        }
    }
    (all, seen)
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

    // 2731 on 19.09.2026. The floor is the population and not the number,
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

/// The pool really does hide effects behind "unless you pay", and this is
/// the population that says so.
///
/// [`Effect::PlayerMayPayOr`] and [`Effect::PlayerMayPayCostOr`] carry a
/// single `&'static Effect` rather than a list, and every hand-rolled walker
/// in this workspace descended into the lists and stopped there. That is a
/// blind spot no reader reported, because an effect nobody walks looks
/// exactly like an effect that is not there — so this test asks the pool
/// what is behind the clause instead of asking a reader whether it looked.
///
/// Measured on 19.09.2026: **35 effects on 35 cards**, being 29
/// `SacrificeSelf` (every Karoo land and the upkeep creatures), 3
/// `DrawCards` (Esper Sentinel, Mystic Remora, Rhystic Study), 2
/// `CounterTargetSpell` (Flusterstorm, Malevolent Hermit) and 1
/// `CreateToken` (Smothering Tithe). Floors and not the numbers: a card
/// joining or leaving is not news, and a walk that stopped descending is.
///
/// The breakdown is the test's own, taken by tallying `behind` rather than
/// by grepping card files: the first reading of it said 29 and then wrote
/// 28, and 28 + 3 + 2 + 1 is 34 against a total of 35. A census whose parts
/// do not sum to its whole has a card in it nobody looked at.
#[test]
fn the_pool_hides_effects_behind_a_price() {
    let (all, seen) = pool_effects();
    let behind: Vec<&'static Effect> = all
        .iter()
        .flat_map(|effect| match effect {
            Effect::PlayerMayPayOr { .. } | Effect::PlayerMayPayCostOr { .. } => {
                effect.branches().0
            }
            _ => &[][..],
        })
        .collect();
    let sacrifices = behind
        .iter()
        .filter(|effect| matches!(effect, Effect::SacrificeSelf))
        .count();

    assert!(
        seen > 2_000,
        "the effect walk visited {seen} effects, which is too few to have \
         read this pool: the nesting shapes it follows have gone stale"
    );
    assert!(
        behind.len() >= 20,
        "the walk found {} effect(s) behind a price where 35 were measured. \
         Either the pool stopped printing the clause or `Effect::branches` \
         stopped descending into it — and the second is the one that makes \
         every reader in this workspace quietly short.",
        behind.len()
    );
    assert!(
        sacrifices >= 15,
        "{sacrifices} of the effects behind a price are `SacrificeSelf` \
         where 28 were measured. The Karoo lands are what that clause is \
         mostly made of here, so this reaching nought means the walk found \
         the wrapper and not what is inside it"
    );
}

/// A face states at most one *list* of modes.
///
/// The question the agent answers names no ability: a modal trigger asks
/// through the permanent and a mode number, and which of its abilities is on
/// the stack lives in the engine's trigger queue. So a face with two modal
/// abilities over two different mode lists is two lists and one number, and
/// `baylee_ai::filter::modal_modes` refuses it and falls back to the printed
/// order rather than picking a list.
///
/// What the pool prints is the harmless half of that, and it is why the rule
/// is about lists and not abilities: Derevi, Empyrial Tactician is one
/// printed sentence with two trigger conditions ("when this enters **and**
/// whenever a creature you control deals combat damage to a player"), written
/// as two `modal_triggered!` over the same `TAP_OR_UNTAP` modes. Naming that
/// shared list is unambiguous however the trigger arrived.
///
/// It asks through `abilities_for_face`, which is the lookup the agent uses
/// and not the union `abilities` builds: face 0 falls back to the card-level
/// list only when it states none of its own, and a probe reading both at once
/// would find two lists where the card has one.
///
/// Two injected findings, because the assertion is an emptiness. `modal`
/// proves the walk reaches modal abilities at all; `shared` proves it reaches
/// the two-abilities-one-list case, which is the case the assertion is a
/// statement about and would otherwise be indistinguishable from a pool where
/// every face has exactly one.
#[test]
fn no_pool_face_states_two_mode_lists() {
    let mut faces = 0;
    let mut modal = 0;
    let mut shared = 0;
    let mut split: Vec<&'static str> = Vec::new();
    for def in baylee_cards::all() {
        for (index, face) in def.faces.iter().enumerate() {
            faces += 1;
            let mut lists: Vec<&'static [baylee_cards::dsl::SpellMode]> = Vec::new();
            let mut here = 0;
            for ability in def.abilities_for_face(index) {
                let (AbilityDef::ModalSpell { modes } | AbilityDef::ModalTriggered { modes, .. }) =
                    ability
                else {
                    continue;
                };
                here += 1;
                if !lists.iter().any(|seen| std::ptr::eq(*seen, *modes)) {
                    lists.push(modes);
                }
            }
            if here > 0 {
                modal += 1;
            }
            if here > lists.len() {
                shared += 1;
            }
            if lists.len() > 1 {
                split.push(face.name);
            }
        }
    }

    // 2836 faces over 2716 cards on 19.09.2026, 10 of them modal and one —
    // Derevi — stating one list twice. The floor is the population and not
    // the number: a card added or removed is not news, a walk that stopped
    // descending into faces is.
    //
    // The 10 is also why the walk is compiled rather than textual, and it is
    // wrong in both directions. Grepping the card files for `ModalSpell` and
    // `ModalTriggered` finds **four**, because seven more are written with
    // the `modal_triggered!` macro and never spell a variant at all — and
    // grepping for the macro as well finds **eleven**, because Marionette
    // Apprentice names it in a `// NOT SUPPORTED:` comment about the
    // fabricate clause it does *not* have.
    assert!(
        faces > 2_500,
        "the walk visited {faces} faces, which is too few to have read this \
         pool: it has stopped descending into the cards it is given"
    );
    assert!(
        modal > 0,
        "the walk found no modal ability in the whole pool, so it is not \
         reaching `ModalSpell` at all and the emptiness below is empty"
    );
    assert!(
        shared > 0,
        "no face in the pool states one mode list twice, so this test no \
         longer distinguishes `modal_modes`'s rule from the stricter one it \
         replaced: check that Derevi still writes its two triggers over one \
         `TAP_OR_UNTAP` before weakening this"
    );
    assert!(
        split.is_empty(),
        "{split:?} state two different mode lists on one face. \
         `baylee_ai::filter::modal_modes` refuses such a face and the agent \
         takes the printed order there: give the mode list an ability handle, \
         or record here why the printed order is the right answer for it."
    );
}
