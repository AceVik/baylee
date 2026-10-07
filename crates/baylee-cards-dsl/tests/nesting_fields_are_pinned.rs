//! Every field of `Effect` that holds another `Effect`, counted.
//!
//! #109 replaced five hand-rolled descents with one walker, and the property
//! that makes that worth doing is that a **new** carrier cannot go quiet. An
//! exhaustive `match` buys most of it: add a variant to `Effect` and the one
//! function that owns the type fails to build.
//!
//! It does not buy all of it, and the gap is why this file exists. A `match`
//! is exhaustive over *variants*, never over *fields* — an arm written
//! `Effect::IfKicked { then, otherwise, .. }` accepts a third
//! `&'static [Effect]` without a word, in every reader at once, which is the
//! exact defect #109 was opened for. `..` is the house idiom here, so this is
//! a live hazard rather than a hypothetical one.
//!
//! So the count is pinned. A fifteenth carrier reddens one named test that
//! says what to do about it, instead of being descended into by nobody.
//!
//! Written deliberately by a session that did not write the walker: a test by
//! the author of the thing it checks shares that author's blind spot, which
//! is the same house rule `scripts/llm/README.md` states for card batches.

/// What `Effect`'s body declares today: seventeen `&'static [Effect]` and three
/// `&'static Effect`.
///
/// Derived twice from the source rather than recalled — once here and once by
/// a script in the session that wrote the walker — and the two agreed.
///
/// Raised 14 → 15 on 2026-09-24 by `Effect::Reflexive`'s body. The walker was
/// shown to descend into it before the number moved:
/// `lints::a_reflexive_fault_fires_on_each_bad_shape` finds a reflexive
/// trigger written inside another one's body, which only a walk through that
/// body can reach.
/// Raised 15 → 16 for `PlayerMayPayLifeOr`; its fallback walk is tested in
/// `verb_tests::life_payment_fallback_is_visited`.
/// Raised 16 → 17 on 2026-09-29 for `MayDoOnceEachTurn` (The Reaper, King
/// No More); its body's walk is tested in
/// `verb_tests::once_each_turn_body_is_visited`.
/// Raised 17 → 18 the same day for `NthResolutionThisTurn` (Omnath, Locus of
/// Creation); `verb_tests::nth_resolution_effects_are_visited`.
/// Raised 18 → 19 the same day for `IfTargetMatches` (Prismatic Ending);
/// `verb_tests::if_target_matches_body_is_visited`.
/// Raised 19 → 20 the same day for `ChooseYoursThen` (Final Showdown);
/// `verb_tests::chosen_permanent_body_is_visited`.
/// Raised 20 → 21 by the merge with the library group's
/// `IfResolvedTimesThisTurn` (Nissa, Resurgent Animist); its branch walk is
/// `verb_tests::the_nth_resolution_branch_is_visited`.
/// Raised 21 → 22 for `PlayerMayPayThen` (Crystal Rod: "you may pay {1}. If
/// you do, …"); `verb_tests::a_price_paid_body_is_visited`.
/// Raised 22 → 24 for `PlayerMayPayManaOr` and `PlayerMayPayManaThen`
/// (Phantasmal Forces, Farmstead: a price with colour in it);
/// `verb_tests::a_coloured_price_body_is_visited`.
/// Raised 24 → 25 for `AtNextEndStep` (Stone Giant's delayed "destroy that
/// creature"); `verb_tests::at_next_end_step_body_is_visited`.
/// Raised 25 → 26 for `IfActivatedThisTurnAtLeast` (Dragon Whelp: "if this
/// ability has been activated four or more times this turn");
/// `verb_tests::activated_at_least_body_is_visited`.
/// Raised 26 → 27 for `AtEndOfCombat` (Cockatrice's delayed "destroy that
/// creature at end of combat"); `verb_tests::at_end_of_combat_body_is_visited`.
/// Raised 27 → 28 for `IfEventObjectMatches` (Berserk's delayed "destroy that
/// creature if it attacked this turn");
/// `verb_tests::if_event_object_matches_body_is_visited`.
/// Linked counter cleanup adds a delayed body; its traversal has a behavioral test.
/// Raised 29 → 30 on 2026-10-07 for `BottomCardFromHand::then` (Vendilion
/// Clique's "if you do"); `verb_tests::bottom_card_from_hand_body_is_visited`.
const NESTING_FIELDS: usize = 30;

/// How many variants those twenty-one fields are spread across.
///
/// Pinned **beside** the field count rather than instead of it, because the
/// two move for different reasons and only one of them describes the defect
/// this came from. A new variant that nests moves both. A second branch added
/// to a variant that already nests — `IfKicked` growing a third arm — moves
/// the field count alone, and that is the case `PlayerMayPayOr` was: not a
/// new variant at all, but a field on an existing one that nobody descended
/// into. A test watching only the variant count would have passed through the
/// whole of #109.
///
/// `IfCondition` is the first variant to move **both** since this was
/// written: one new carrier, two new branches. That is the shape the pin is
/// for — it is a limitation whose breaking is the success, so the number is
/// raised once the walker is shown to descend into the newcomer, and never
/// to make a red test quiet.
///
/// `Reflexive` moved both again on 2026-09-24: one new carrier, one new
/// branch. `MayDoOnceEachTurn`, `NthResolutionThisTurn`, `IfTargetMatches`
/// and `ChooseYoursThen` moved both on 2026-09-29, the same shape, and so
/// did `IfResolvedTimesThisTurn` the same day.
///
/// `PlayerMayPayThen` moved both on 2026-09-29: one new carrier, one branch.
/// `PlayerMayPayManaOr` and `PlayerMayPayManaThen` moved both on 2026-09-30:
/// two new carriers, one branch each. `AtNextEndStep` moved both on
/// 2026-09-30: one new carrier, one branch.
/// `IfActivatedThisTurnAtLeast` moved both on 2026-09-30: one new carrier,
/// one branch. `AtEndOfCombat` moved both on 2026-09-30: one new carrier,
/// one branch. `IfEventObjectMatches` moved both on 2026-09-30: one new
/// carrier, one branch.
/// `ScheduleLinkedCounterCleanup` is the twenty-sixth carrier.
/// `BottomCardFromHand` is the twenty-seventh (2026-10-07): one new
/// carrier, one branch.
const CARRYING_VARIANTS: usize = 27;

/// The floor under the reader itself.
///
/// A textual reader that finds nothing must fail loudly, not report zero and
/// pass something downstream. Eleven readers of the card pool in this repo
/// have now been found answering a question they could not see, and the one
/// that said so carried a bound exactly like this one. `Effect` has well over
/// eighty variants, so a body that yields fewer than sixty has not been read.
const MIN_VARIANTS: usize = 60;

/// The body of `pub enum Effect { … }`, by brace balance.
///
/// Balance rather than a regular expression because the body contains braces
/// of its own on nearly every line, and a lazy match would stop at the first
/// variant.
fn enum_body(src: &str) -> String {
    let head = src
        .find("pub enum Effect {")
        .expect("`pub enum Effect {` is not in effect/mod.rs — has it been renamed?");
    let open = src[head..].find('{').expect("checked above") + head;
    let mut depth = 0_usize;
    for (i, c) in src[open..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return src[open + 1..open + i].to_string();
                }
            }
            _ => {}
        }
    }
    panic!("`enum Effect` is never closed");
}

/// The same body with every comment line dropped.
///
/// The prose above these variants discusses `&'static [Effect]` by name, so a
/// reader that counted doc comments would count the documentation as fields.
fn without_comments(body: &str) -> String {
    body.lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn every_field_that_nests_an_effect_is_one_the_walker_knows_about() {
    let src = include_str!("../src/effect/mod.rs");
    let body = without_comments(&enum_body(src));

    // A variant opens with an upper-case name at one level of indentation.
    // Only a floor is read off this, so an approximation is honest here in a
    // way it would not be if a number were derived from it.
    let variants = body
        .lines()
        .filter(|l| {
            let t = l.trim_start();
            l.len() - t.len() == 4
                && t.starts_with(|c: char| c.is_ascii_uppercase())
                && (t.ends_with('{') || t.contains('(') || t.ends_with(','))
        })
        .count();
    assert!(
        variants >= MIN_VARIANTS,
        "only {variants} variants found in `enum Effect`, under a floor of \
         {MIN_VARIANTS}. The reader has gone blind rather than the enum \
         having shrunk — fix the reader before believing the count below."
    );

    // `&'static [Effect]` cannot be mistaken for `&'static Effect`: the
    // bracket sits where the `E` would, so plain counting separates them.
    let slices = body.matches("&'static [Effect]").count();
    let singles = body.matches("&'static Effect").count();
    let found = slices + singles;

    // Which variants those sit on. The variant is read first and the field
    // second on the same line, so a tuple variant — `Sequence(&'static
    // [Effect])`, where both are one line — is attributed to itself rather
    // than to whatever came before it.
    let mut carrying: Vec<&str> = Vec::new();
    let mut current = "";
    for line in body.lines() {
        let t = line.trim_start();
        if line.len() - t.len() == 4 && t.starts_with(|c: char| c.is_ascii_uppercase()) {
            current = t
                .split(|c: char| !c.is_ascii_alphanumeric())
                .next()
                .unwrap_or("");
        }
        if (t.contains("&'static [Effect]") || t.contains("&'static Effect"))
            && !carrying.contains(&current)
        {
            carrying.push(current);
        }
    }
    assert_eq!(
        carrying.len(),
        CARRYING_VARIANTS,
        "{} variants of `Effect` nest another one, and this test is pinned to \
         {CARRYING_VARIANTS}: {carrying:?}",
        carrying.len()
    );

    assert_eq!(
        found, NESTING_FIELDS,
        "`Effect` now declares {found} fields that hold another `Effect` \
         ({slices} slices, {singles} singles), and this test is pinned to \
         {NESTING_FIELDS}.\n\
         \n\
         If you added one: `Effect::branches` in this crate has to descend \
         into it, and then this number moves. An exhaustive `match` will not \
         tell you — a new *field* on an existing variant is accepted by any \
         arm that ends in `..`, silently, in every reader at once. That is \
         what #109 closed and what this number keeps closed.\n\
         \n\
         If you removed one, the same, downwards."
    );
}
