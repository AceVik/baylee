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
//! - the cost paid leaving other mana untapped: for each kind of source the
//!   first plan leaves standing, the plan that taps that kind first, where
//!   what it leaves differs. The planner keeps the colour the hand needs most
//!   (`manaplan::prioritize`), and the seat cannot choose lands itself, so
//!   it is shown the other way too: `{3}{U}{G}` over a floating `{U}{G}` is
//!   paid by three Forests, keeping an Island and a Forest, or by two
//!   Forests and the Island, keeping two Forests (Carbon b4, attempt 4,
//!   q247);
//! - the cost paid without one nonbasic land the first plan taps.
//!
//! [`hybrid_others`] does the same for a cast option whose spell reads the
//! colours of mana spent to cast it (`Amount::ManaColorSpentToCast`,
//! `Amount::ColorsOfManaSpentToCast`) and whose cost has hybrid symbols
//! (CR 107.4e, 601.2g, 601.2h: Catharsis, Deceit, Firespout): beside the
//! planner's first plan it lists up to [`OTHERS`] concrete hybrid colour
//! assignments, distinct by the coloured mana counts they produce and
//! spending exactly what they tap.

use crate::wake::{Reach, ReachFrom, Way};
use baylee_cards_dsl::{AbilityDef, Amount, Condition, Effect, EnterModifier};
use baylee_client_core::manaplan::{self, Plan, Source};
use baylee_core::color::{Color, ColorPair};
use baylee_core::ids::ObjectId;
use baylee_core::mana::{ManaColor, ManaCost, ManaSymbol};
use baylee_core::types::SupertypeSet;
use baylee_engine::choice::{CastModeKind, LegalActions, Spend};
use baylee_view::{CardIdentity, ManaPoolView, PlayerView};

/// The most plans listed beside the first.
pub(super) const OTHERS: usize = 3;

/// Plans other than `first` that pay `owed` from `pool` and `sources`, in
/// the order they are listed: by colour, then leaving other mana untapped,
/// then without a nonbasic land; no two tap the same sources for the same
/// colours, and none is `first`.
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
    for plan in leaving_other_mana(owed, pool, sources, first) {
        keep(plan, &mut out);
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

/// What a source makes, as the seat reads what is left up: its colours and
/// how much one tap makes.
type Kind = (Vec<usize>, u8);

fn kind(source: &Source) -> Kind {
    let mut colours: Vec<usize> = source.colors.iter().map(|c| c.index()).collect();
    colours.sort_unstable();
    colours.dedup();
    (colours, source.amount)
}

/// The kinds of the sources `plan` leaves untapped, sorted: two plans that
/// leave the same kinds standing leave the same mana up.
fn left_up(plan: &Plan, sources: &[Source]) -> Vec<Kind> {
    let mut left: Vec<Kind> = sources
        .iter()
        .filter(|source| !plan.steps.iter().any(|step| step.source == source.id))
        .map(kind)
        .collect();
    left.sort_unstable();
    left
}

/// For each kind of source `first` leaves untapped, in source order, the
/// plan that taps sources of that kind ahead of every other (they are made
/// the least worth keeping, `Source::preserve`), where it leaves other mana
/// up than `first` and the plans before it.
fn leaving_other_mana(
    owed: &ManaCost,
    pool: &ManaPoolView,
    sources: &[Source],
    first: &Plan,
) -> Vec<Plan> {
    let mut seen = vec![left_up(first, sources)];
    let mut kinds: Vec<Kind> = Vec::new();
    for source in sources {
        let standing = !first.steps.iter().any(|step| step.source == source.id);
        if standing && !kinds.contains(&kind(source)) {
            kinds.push(kind(source));
        }
    }
    let mut out = Vec::new();
    for wanted in kinds {
        let ranked: Vec<Source> = sources
            .iter()
            .map(|source| Source {
                preserve: if kind(source) == wanted {
                    0
                } else {
                    source.preserve.saturating_add(1 << 24)
                },
                ..source.clone()
            })
            .collect();
        let Some(plan) = manaplan::plan(owed, pool, &ranked).filter(|p| !p.steps.is_empty()) else {
            continue;
        };
        let left = left_up(&plan, sources);
        if !seen.contains(&left) {
            seen.push(left);
            out.push(plan);
        }
    }
    out
}

/// Walks `plan`'s steps over `pool` and `sources` (a paid mana ability spends
/// as it makes) and returns the pool they leave.
fn pool_after(plan: &Plan, pool: &ManaPoolView, sources: &[Source]) -> Option<ManaPoolView> {
    let mut floating = *pool;
    for step in &plan.steps {
        floating = manaplan::after_step(&floating, step, sources)?;
    }
    Some(floating)
}

/// The pool `plan` leaves when `plan`'s taps and `pool` make exactly the mana
/// `owed` costs: then the payment spends every unit, and the colours floating
/// are the colours spent (CR 601.2h).
fn exact_pool(
    plan: &Plan,
    owed: &ManaCost,
    pool: &ManaPoolView,
    sources: &[Source],
) -> Option<ManaPoolView> {
    let needed: u64 = owed
        .symbols()
        .map(|symbol| match symbol {
            ManaSymbol::Generic(n) => u64::from(n),
            _ => 1,
        })
        .sum();
    let floating = pool_after(plan, pool, sources)?;
    (floating.total() - floating.restricted_total() == needed).then_some(floating)
}

/// Whether `plan`'s taps and the pool make exactly the mana `owed` costs:
/// then the payment spends every unit, and the colours they are is what it
/// spends (CR 601.2h).
fn spends_exactly(plan: &Plan, owed: &ManaCost, pool: &ManaPoolView, sources: &[Source]) -> bool {
    exact_pool(plan, owed, pool, sources).is_some()
}

/// The five coloured mana counts floating in `pool`.
const fn colour_counts(pool: &ManaPoolView) -> [u32; 5] {
    [pool.white, pool.blue, pool.black, pool.red, pool.green]
}

/// Up to [`OTHERS`] alternative plans for casting a hybrid spell whose own
/// abilities read the colours of mana spent to cast it (CR 107.4e, 601.2g,
/// 601.2h: Catharsis, Deceit, Firespout).
///
/// Each plan resolves every hybrid symbol in `reach.plan.cost` to one of its
/// colours, spends exactly what it taps (so the colours tapped are the colours
/// spent), and differs from `reach.plan` and earlier alternatives in the
/// coloured mana counts it produces. Ordinary hybrid spells without such a
/// reader keep a single option per cast way.
pub(super) fn hybrid_others(view: &PlayerView, legal: &LegalActions, reach: &Reach) -> Vec<Plan> {
    if reach.face_down || reach.plan.steps.is_empty() {
        return Vec::new();
    }
    if !reads_mana_spent(reach.card, reach_face(reach)) {
        return Vec::new();
    }
    let readings = hybrid_readings(&reach.plan.cost);
    if readings.is_empty() {
        return Vec::new();
    }
    let me = baylee_client_core::decision::resource_player(view);
    let Some(raw_pool) = view.seat(me).map(|seat| seat.mana_pool) else {
        return Vec::new();
    };
    let mode = reach_mode(view, reach);
    let spend = Spend::cast(reach.object, mode);
    let pool = baylee_ai::spend::pool_for(&raw_pool, legal, spend);
    let mut plain = baylee_ai::seat_mana_sources(view, legal);
    baylee_ai::spend::repeat_counter_sources(view, &mut plain);
    let restricted = baylee_ai::spend::restricted_sources(view, legal);
    let sources = baylee_ai::spend::sources_for(
        legal,
        spend,
        baylee_ai::spend::Mana {
            pool: &raw_pool,
            plain: &plain,
            restricted: &restricted,
        },
        None,
    );
    let mut seen_taps = vec![taps(&reach.plan)];
    let mut seen_colours: Vec<[u32; 5]> = pool_after(&reach.plan, &pool, &sources)
        .map(|p| colour_counts(&p))
        .into_iter()
        .collect();
    let mut out = Vec::new();
    for concrete in readings {
        let Some(mut plan) =
            baylee_ai::plan_for_spell(&concrete, &raw_pool, view, legal, reach.object, mode)
        else {
            continue;
        };
        if plan.steps.is_empty() {
            continue;
        }
        let Some(floating) = exact_pool(&plan, &reach.plan.cost, &pool, &sources) else {
            continue;
        };
        let counts = colour_counts(&floating);
        let key = taps(&plan);
        if seen_colours.contains(&counts) || seen_taps.contains(&key) {
            continue;
        }
        seen_colours.push(counts);
        seen_taps.push(key);
        plan.cost = reach.plan.cost;
        out.push(plan);
        if out.len() >= OTHERS {
            break;
        }
    }
    out
}

/// The face index a [`Reach`] casts.
fn reach_face(reach: &Reach) -> usize {
    match reach.way {
        Some(Way::Mode(
            CastModeKind::Face(face)
            | CastModeKind::EscapeGranted { face, .. }
            | CastModeKind::LibraryTopLife { face, .. },
        )) => face,
        _ => usize::from(reach.card.face),
    }
}

/// The [`CastModeKind`] a [`Reach`] plans with.
fn reach_mode(view: &PlayerView, reach: &Reach) -> CastModeKind {
    match reach.way {
        Some(Way::Mode(kind)) => kind,
        None if reach.face_down => CastModeKind::Disguise,
        None if reach.from == ReachFrom::Graveyard
            && view
                .graveyards
                .iter()
                .flatten()
                .find(|o| o.id == reach.object)
                .is_some_and(|o| o.flashback.is_some()) =>
        {
            CastModeKind::Flashback
        }
        _ => CastModeKind::Normal,
    }
}

/// Every concrete colour assignment for the hybrid symbols in `cost` (CR
/// 107.4e), with all-one-colour assignments first so colour-threshold spells
/// (`{R}{R}` / `{W}{W}`) are always offered before mixed splits. Empty when
/// `cost` has no hybrid symbols.
fn hybrid_readings(cost: &ManaCost) -> Vec<ManaCost> {
    const MAX_READINGS: usize = 32;
    let mut base = ManaCost::ZERO;
    let mut hybrids: Vec<(ColorPair, u32)> = Vec::new();
    for (symbol, count) in cost.runs() {
        let count = u32::from(count);
        match symbol {
            ManaSymbol::Hybrid(pair) | ManaSymbol::HybridPhyrexian(pair) => {
                if let Some((_, existing)) = hybrids.iter_mut().find(|(p, _)| *p == pair) {
                    *existing = existing.saturating_add(count);
                } else {
                    hybrids.push((pair, count));
                }
            }
            other => base = base.with_symbol(other, count),
        }
    }
    if hybrids.is_empty() {
        return Vec::new();
    }
    let mut out = vec![base];
    for (pair, n) in hybrids {
        let mut splits = Vec::with_capacity(usize::try_from(n).unwrap_or(0) + 1);
        splits.push(n);
        splits.push(0);
        for k in 1..n {
            splits.push(k);
        }
        let mut next = Vec::new();
        for partial in &out {
            for &first_n in &splits {
                let candidate = partial
                    .with_symbol(ManaSymbol::of_color(pair.first()), first_n)
                    .with_symbol(ManaSymbol::of_color(pair.second()), n - first_n);
                if !next.contains(&candidate) {
                    next.push(candidate);
                    if next.len() >= MAX_READINGS {
                        break;
                    }
                }
            }
            if next.len() >= MAX_READINGS {
                break;
            }
        }
        out = next;
    }
    out
}

/// Whether `card`'s face `face_idx` has any ability or enter modifier that
/// reads the colours of mana spent to cast it (`Amount::ManaColorSpentToCast`
/// or `Amount::ColorsOfManaSpentToCast`).
fn reads_mana_spent(card: CardIdentity, face_idx: usize) -> bool {
    let Some(def) = baylee_cards::by_index(card.index) else {
        return false;
    };
    let enter_reads = def.faces.get(face_idx).is_some_and(|face| {
        face.enter_modifiers.iter().any(|modifier| match modifier {
            EnterModifier::TappedUnlessCondition(c) => cond_reads_mana_spent(c, 0),
            EnterModifier::WithCounters { amount, .. }
            | EnterModifier::PayLifeUpTo(amount)
            | EnterModifier::Bloodthirst(amount) => amount_reads_mana_spent(amount, 0),
            EnterModifier::WithCountersIf {
                count, condition, ..
            } => amount_reads_mana_spent(count, 0) || cond_reads_mana_spent(condition, 0),
            _ => false,
        })
    });
    enter_reads
        || def
            .abilities_for_face(face_idx)
            .iter()
            .any(ability_reads_mana_spent)
}

fn ability_reads_mana_spent(ability: &AbilityDef) -> bool {
    match ability {
        AbilityDef::Triggered {
            condition, effects, ..
        }
        | AbilityDef::Spell {
            condition, effects, ..
        } => {
            condition
                .as_ref()
                .is_some_and(|c| cond_reads_mana_spent(c, 0))
                || effects_read_mana_spent(effects)
        }
        AbilityDef::ModalTriggered {
            condition, modes, ..
        }
        | AbilityDef::ModalActivated {
            condition, modes, ..
        } => {
            condition
                .as_ref()
                .is_some_and(|c| cond_reads_mana_spent(c, 0))
                || modes.iter().any(|m| effects_read_mana_spent(m.effects))
        }
        AbilityDef::ModalSpell { modes, .. } | AbilityDef::Modal { modes, .. } => {
            modes.iter().any(|m| effects_read_mana_spent(m.effects))
        }
        AbilityDef::Activated { effects, .. }
        | AbilityDef::SagaChapter { effects, .. }
        | AbilityDef::Loyalty { effects, .. } => effects_read_mana_spent(effects),
        AbilityDef::ActivatedConditional {
            condition, effects, ..
        } => cond_reads_mana_spent(condition, 0) || effects_read_mana_spent(effects),
        AbilityDef::Static(sa) => sa
            .condition
            .as_ref()
            .is_some_and(|c| cond_reads_mana_spent(c, 0)),
        _ => false,
    }
}

fn cond_reads_mana_spent(cond: &Condition, depth: u8) -> bool {
    if depth >= 8 {
        return false;
    }
    match cond {
        Condition::AmountAtLeast(amount, _) | Condition::AmountIsEven(amount) => {
            amount_reads_mana_spent(amount, depth + 1)
        }
        Condition::AmountCompare { left, right, .. } => {
            amount_reads_mana_spent(left, depth + 1) || amount_reads_mana_spent(right, depth + 1)
        }
        Condition::Not(inner) | Condition::TriggerOnly(inner) => {
            cond_reads_mana_spent(inner, depth + 1)
        }
        Condition::All(parts) | Condition::Any(parts) => parts
            .iter()
            .any(|part| cond_reads_mana_spent(part, depth + 1)),
        _ => false,
    }
}

fn amount_reads_mana_spent(amount: &Amount, depth: u8) -> bool {
    if depth >= 8 {
        return false;
    }
    match amount {
        Amount::ManaColorSpentToCast(_) | Amount::ColorsOfManaSpentToCast => true,
        Amount::IfCondition {
            condition,
            then,
            otherwise,
        } => {
            cond_reads_mana_spent(condition, depth + 1)
                || amount_reads_mana_spent(then, depth + 1)
                || amount_reads_mana_spent(otherwise, depth + 1)
        }
        Amount::Negated(a)
        | Amount::Plus { base: a, .. }
        | Amount::SaturatingSub { base: a, .. }
        | Amount::ConstantMinus { subtract: a, .. }
        | Amount::Half { base: a, .. }
        | Amount::Divided { base: a, .. }
        | Amount::Times { base: a, .. } => amount_reads_mana_spent(a, depth + 1),
        Amount::Sum { left, right } | Amount::Difference { left, right } => {
            amount_reads_mana_spent(left, depth + 1) || amount_reads_mana_spent(right, depth + 1)
        }
        _ => false,
    }
}

#[allow(clippy::too_many_lines)] // One arm per Effect variant carrying an Amount or Condition.
fn effects_read_mana_spent(effects: &'static [Effect]) -> bool {
    let mut found = false;
    Effect::walk(effects, &mut 0, &mut |effect| {
        if found {
            return;
        }
        found = match effect {
            Effect::IfCondition { condition, .. } | Effect::RepeatWhile { condition, .. } => {
                cond_reads_mana_spent(condition, 0)
            }
            Effect::RepeatProcess {
                max_times,
                pre_condition,
                post_condition,
                ..
            } => {
                max_times
                    .as_ref()
                    .is_some_and(|a| amount_reads_mana_spent(a, 0))
                    || pre_condition
                        .as_ref()
                        .is_some_and(|c| cond_reads_mana_spent(c, 0))
                    || post_condition
                        .as_ref()
                        .is_some_and(|c| cond_reads_mana_spent(c, 0))
            }
            Effect::ReflexiveTrigger { condition, .. }
            | Effect::RegisterDelayedTrigger { condition, .. } => {
                condition.is_some_and(|c| cond_reads_mana_spent(c, 0))
            }
            Effect::DrawCards { amount }
            | Effect::DrawCardsFor { amount, .. }
            | Effect::DiscardAmountForPlayers { amount, .. }
            | Effect::GainLife { amount }
            | Effect::GainLifeFor { amount, .. }
            | Effect::LoseLife { amount, .. }
            | Effect::DealDamage { amount, .. }
            | Effect::DealDamageEach { amount, .. }
            | Effect::DealDamageEvenly { amount, .. }
            | Effect::AddCounter { amount, .. }
            | Effect::AddCounterFilter { amount, .. }
            | Effect::AddCounterSource { amount, .. }
            | Effect::PutCountersDivided { amount, .. }
            | Effect::PutCounterChoice { amount, .. }
            | Effect::CreateTokenN { amount, .. }
            | Effect::CreateTokensFor { amount, .. }
            | Effect::CreateTokenAttached { amount, .. }
            | Effect::Mill { amount, .. }
            | Effect::Scry { amount, .. }
            | Effect::Surveil { amount, .. }
            | Effect::AddMana { amount, .. }
            | Effect::Bolster(amount)
            | Effect::Monstrosity(amount)
            | Effect::Firebend(amount)
            | Effect::Mobilize(amount)
            | Effect::Endure(amount)
            | Effect::Repeat { times: amount, .. }
            | Effect::ChooseCard { max: amount, .. }
            | Effect::ChooseCardConstrained { max: amount, .. } => {
                amount_reads_mana_spent(amount, 0)
            }
            Effect::PumpTarget {
                power, toughness, ..
            }
            | Effect::PumpFilter {
                power, toughness, ..
            }
            | Effect::PumpEventObject {
                power, toughness, ..
            } => amount_reads_mana_spent(power, 0) || amount_reads_mana_spent(toughness, 0),
            Effect::CreateTokenSized {
                amount,
                power,
                toughness,
                ..
            } => {
                amount_reads_mana_spent(amount, 0)
                    || amount_reads_mana_spent(power, 0)
                    || amount_reads_mana_spent(toughness, 0)
            }
            Effect::DamagePermanentAndController {
                permanent_amount,
                controller_amount,
                ..
            } => {
                amount_reads_mana_spent(permanent_amount, 0)
                    || amount_reads_mana_spent(controller_amount, 0)
            }
            Effect::ChooseNumberEx { min, max, .. } => {
                amount_reads_mana_spent(min, 0) || amount_reads_mana_spent(max, 0)
            }
            Effect::MoveAndModify { counters, .. } => counters
                .iter()
                .any(|(_, amt)| amount_reads_mana_spent(amt, 0)),
            Effect::SetDynamicPT {
                power, toughness, ..
            } => {
                power
                    .as_ref()
                    .is_some_and(|a| amount_reads_mana_spent(a, 0))
                    || toughness
                        .as_ref()
                        .is_some_and(|a| amount_reads_mana_spent(a, 0))
            }
            _ => false,
        };
    });
    found
}

/// The colours (of the five) in the pool and made by `plan`'s taps.
fn colours(plan: &Plan, pool: &ManaPoolView, sources: &[Source]) -> Vec<Color> {
    let made: Vec<ManaColor> = plan
        .steps
        .iter()
        .flat_map(|step| manaplan::made_by(step, sources))
        .map(|(color, _)| color)
        .collect();
    Color::ALL
        .into_iter()
        .filter(|&c| {
            let mana = ManaColor::from_color(c);
            pooled(pool, mana) > 0 || made.contains(&mana)
        })
        .collect()
}

/// Whether `source` can make `colour`, in any of its modes.
fn makes(source: &Source, colour: ManaColor) -> bool {
    source.colors.contains(&colour)
        || source
            .dear
            .as_ref()
            .is_some_and(|dear| dear.colors.contains(&colour))
        || source
            .converter
            .as_ref()
            .is_some_and(|converter| converter.colors.contains(&colour))
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
