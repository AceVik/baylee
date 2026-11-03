//! Other ways to pay a payment owed than the planner's first.
//!
//! A payment window (CR 605.3a) is answered with one option that taps a
//! plan's sources and passes: one answer, one call. The planner's plan is
//! the cheapest and spends as few colours as it can, which is what most
//! payments want and exactly what some do not. Converge (an ability word,
//! CR 207.2c) counts "the number of colors of mana spent to cast this
//! spell", which the caster chooses as it pays (CR 601.2h), and the window
//! offered Prismatic Ending one way to pay {1}{W} with {W} floating: Tundra
//! for {W}, one colour, so its target was not exiled (agy attempt 108,
//! q106, and ten more reports). It also chose a source that hurts or does
//! more than make mana where another would have paid (City of Brass over a
//! Plains).
//!
//! So the window lists a few other plans beside the first, each one answer
//! as the first is, within the actions the protocol already has
//! ([`super::Act::Taps`] ending on the pass that settles the payment):
//!
//! - the generic part paid in colours the cost does not name, most colours
//!   first, only where the taps and the pool make exactly what is owed (so
//!   every unit is spent and the colours tapped are the colours spent);
//! - the cost paid without one nonbasic land the first plan taps.

use baylee_client_core::manaplan::{self, Plan, Source};
use baylee_core::color::Color;
use baylee_core::ids::ObjectId;
use baylee_core::mana::{ManaColor, ManaCost, ManaSymbol};
use baylee_core::types::SupertypeSet;
use baylee_view::{ManaPoolView, PlayerView};

/// The most plans listed beside the first.
pub(super) const OTHERS: usize = 3;

/// Plans other than `first` that pay `owed` from `pool` and `sources`, in
/// the order they are listed: by colour, then without a nonbasic land; no
/// two tap the same sources for the same colours, and none is `first`.
pub(super) fn others(
    view: &PlayerView,
    owed: &ManaCost,
    pool: &ManaPoolView,
    sources: &[Source],
    first: &Plan,
) -> Vec<Plan> {
    let mut seen = vec![taps(first)];
    let mut out: Vec<Plan> = Vec::new();
    let mut keep = |plan: Plan, out: &mut Vec<Plan>| {
        let key = taps(&plan);
        if !seen.contains(&key) {
            seen.push(key);
            out.push(plan);
        }
    };
    let mut colours_seen = vec![colours(first, pool, sources)];
    for plan in by_colour(owed, pool, sources) {
        // One plan for each set of colours spent: two lands that make the
        // same colours are the same choice to a model counting colours.
        let made = colours(&plan, pool, sources);
        if !colours_seen.contains(&made) {
            colours_seen.push(made);
            keep(plan, &mut out);
        }
    }
    for step in &first.steps {
        let basic = view
            .object(step.source)
            .is_some_and(|o| o.supertypes.contains(SupertypeSet::BASIC));
        if basic {
            continue;
        }
        let rest: Vec<Source> = sources
            .iter()
            .filter(|s| s.id != step.source)
            .cloned()
            .collect();
        if let Some(plan) = manaplan::plan(owed, pool, &rest).filter(|p| !p.steps.is_empty()) {
            keep(plan, &mut out);
        }
    }
    out.truncate(OTHERS);
    out
}

/// Plans that pay `owed`'s generic part, as far as it goes, in colours the
/// cost does not name and some source makes: most colours spent first,
/// then fewest taps.
fn by_colour(owed: &ManaCost, pool: &ManaPoolView, sources: &[Source]) -> Vec<Plan> {
    let named = owed.colors();
    let extra: Vec<Color> = Color::ALL
        .into_iter()
        .filter(|&c| !named.contains(c))
        .filter(|&c| sources.iter().any(|s| makes(s, ManaColor::from_color(c))))
        .collect();
    let generic = owed.generic_total();
    let mut plans: Vec<Plan> = Vec::new();
    for mask in 1_u32..(1 << extra.len()) {
        let n = mask.count_ones();
        if n > generic {
            continue;
        }
        let cost = extra
            .iter()
            .enumerate()
            .filter(|(i, _)| mask & (1 << i) != 0)
            .fold(owed.with_less_generic(n), |cost, (_, &c)| {
                cost.with_symbol(ManaSymbol::of_color(c), 1)
            });
        let Some(mut plan) = manaplan::plan(&cost, pool, sources) else {
            continue;
        };
        if plan.steps.is_empty() || !spends_exactly(&plan, owed, pool, sources) {
            continue;
        }
        plan.cost = *owed;
        plans.push(plan);
    }
    plans.sort_by_key(|plan| {
        (
            std::cmp::Reverse(colours(plan, pool, sources).len()),
            plan.taps(),
        )
    });
    plans
}

/// Whether `plan`'s taps and the pool make exactly the mana `owed` costs:
/// then the payment spends every unit, and the colours they are is what it
/// spends (CR 601.2h).
fn spends_exactly(plan: &Plan, owed: &ManaCost, pool: &ManaPoolView, sources: &[Source]) -> bool {
    let needed: u64 = owed
        .symbols()
        .map(|symbol| match symbol {
            ManaSymbol::Generic(n) => u64::from(n),
            _ => 1,
        })
        .sum();
    let made: u64 = plan
        .steps
        .iter()
        .map(|step| {
            sources
                .iter()
                .find(|s| s.id == step.source)
                .map_or(1, |s| u64::from(s.amount))
        })
        .sum();
    pool.total() - pool.restricted_total() + made == needed
}

/// The colours (of the five) in the pool and made by `plan`'s taps.
fn colours(plan: &Plan, pool: &ManaPoolView, sources: &[Source]) -> Vec<Color> {
    let made: Vec<ManaColor> = plan
        .steps
        .iter()
        .filter_map(|step| {
            step.color.or_else(|| {
                let source = sources.iter().find(|s| s.id == step.source)?;
                let colors = match &source.dear {
                    Some(dear) if dear.tap == step.tap => &dear.colors,
                    _ => &source.colors,
                };
                (colors.len() == 1).then(|| colors[0])
            })
        })
        .collect();
    Color::ALL
        .into_iter()
        .filter(|&c| {
            let mana = ManaColor::from_color(c);
            pooled(pool, mana) > 0 || made.contains(&mana)
        })
        .collect()
}

/// Whether `source` can make `colour`, in either of its modes.
fn makes(source: &Source, colour: ManaColor) -> bool {
    source.colors.contains(&colour)
        || source
            .dear
            .as_ref()
            .is_some_and(|dear| dear.colors.contains(&colour))
}

/// A plan's taps, as a key: which sources, for which colours.
fn taps(plan: &Plan) -> Vec<(ObjectId, Option<usize>)> {
    let mut key: Vec<(ObjectId, Option<usize>)> = plan
        .steps
        .iter()
        .map(|s| (s.source, s.color.map(ManaColor::index)))
        .collect();
    key.sort_unstable();
    key
}

/// The unrestricted mana of `colour` floating in `pool`.
const fn pooled(pool: &ManaPoolView, colour: ManaColor) -> u32 {
    match colour {
        ManaColor::White => pool.white,
        ManaColor::Blue => pool.blue,
        ManaColor::Black => pool.black,
        ManaColor::Red => pool.red,
        ManaColor::Green => pool.green,
        ManaColor::Colorless => pool.colorless,
    }
}
