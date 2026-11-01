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
//! **no** two-faced cards, where the compiled pool has 120: a textual probe
//! answers a question it cannot see, which is the same fault `knob` exists
//! to prevent in `xtask`. The same scouting pass then missed the one card in
//! the pool that *is* a Partner commander, because `partner` is a field on
//! `CardDef` and not a bit in `KeywordSet`: the first run of this test is
//! what found Sakashima of a Thousand Faces, and it found it by asking the
//! type rather than the spelling.
//!
//! Four things in that table have no probe here, and the reasons are
//! three different ones.
//!
//! **Proliferate, madness and meld are words the DSL cannot say at all.**
//! There is no `Effect` for proliferate and no handle anywhere for the other
//! two, so the thing that would have to change first is the vocabulary
//! rather than a card. A probe would assert zero against a population that
//! can never be anything else, which is a green test measuring nothing.
//!
//! **Infect and wither can be said and are not printed**, so they *are*
//! probed — in the tripwire test above, where a zero is the finding.
//!
//! **Full deck families and full-game lookahead are not card shapes.** One
//! is a paired-seed benchmark and the other a property of the search model;
//! no count over `baylee_cards::all()` could open or close either, and
//! pretending otherwise would put a number beside a row that the number
//! says nothing about.

use baylee_cards::dsl::{
    AbilityDef, CardDef, CostPart, CounterKind, Effect, KeywordSet, Modifier, PartnerKind,
};
use baylee_core::types::TypeSet;

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

/// Cards that *reach* an effect this probe accepts, at any depth.
///
/// The descent is [`Effect::walk`]'s, so this reads what a card does inside
/// a `Sequence`, in either half of a kicker clause and behind "unless you
/// pay". A probe matching the top-level list alone would answer a narrower
/// question than the row it is standing for asks, and would answer it
/// without saying so — which is the fault #109 closed and not one to
/// reintroduce one file away from it.
fn count_reaching(probe: impl Fn(&'static Effect) -> bool) -> usize {
    baylee_cards::all()
        .filter(|def| {
            let mut hit = false;
            let mut seen = 0;
            for effects in abilities(def).flat_map(ability_effects) {
                Effect::walk(effects, &mut seen, &mut |effect| hit |= probe(effect));
            }
            hit
        })
        .count()
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
        AbilityDef::ModalSpell { modes, .. }
        | AbilityDef::ModalTriggered { modes, .. }
        | AbilityDef::ModalActivated { modes, .. } => {
            modes.iter().map(|mode| mode.effects).collect()
        }
        AbilityDef::Unimplemented
        | AbilityDef::StartYourEngines
        | AbilityDef::Ward { .. }
        | AbilityDef::Toxic { .. }
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
        if let Effect::AddCounter { kind, .. }
        | Effect::AddCounterFilter { kind, .. }
        | Effect::DoubleCountersFilter { kind, .. } = effect
        {
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

#[path = "ai_coverage_guards/effect_shapes.rs"]
mod effect_shapes;
#[path = "ai_coverage_guards/mechanic_debt.rs"]
mod mechanic_debt;
#[path = "ai_coverage_guards/priced_grants.rs"]
mod priced_grants;
#[path = "ai_coverage_guards/time_counters.rs"]
mod time_counters;
