//! Paying through a paid mana ability: a filter land, a Signet.
//!
//! `{1}, {T}: Add one mana of any color` is a trade, not a source. Its mana
//! has to come out of another permanent and be floating as it is activated,
//! so the plan is a chain with an order, and the plain matcher, which sees one
//! independent tap per permanent, cannot draw it: Meteor Storm's `{R}{G}`
//! beside Canyon Slough, Crucible of the Spirit Dragon and Shimmering Grotto
//! is payable (the Crucible pays the Grotto's `{1}`, the Grotto makes `{G}`,
//! the Slough makes `{R}`) and was offered to nobody.
//!
//! So [`plan`] is asked only when the plain matcher has said no, which leaves
//! every board that planned before planning the same taps. It tries the
//! converters one chain at a time, the fewest first, each a matching of Kuhn's
//! with their costs among the demands and their mana among the units:
//!
//! - A converter's permanent taps for its conversion and for nothing else,
//!   so its free mana leaves the units. A permanent taps once.
//! - **A converter's mana pays only what stands after it in the chain**: the
//!   spell, or a later converter. That is what keeps a filter land from paying
//!   for itself, and two of them from paying for each other.
//! - The steps run converter by converter, each one's payers first, and the
//!   plain taps last. The engine pays a generic cost out of whatever floats,
//!   in its own colour order (`mana_pay`, white first), so a Canyon Slough
//!   tapped ahead of the Grotto loses its `{R}` to the Grotto's `{1}`.
//! - Every plan is walked through before it is answered ([`holds`]), the pool
//!   spent as the engine spends it. The matching says what could pay; the
//!   walk says what is left when the engine has chosen. A chain whose walk
//!   fails is no plan, and the next order is tried.
//!
//! It is bounded: at most [`MAX_CONVERTERS`] permanents are read, the cheapest
//! to give up first, so at most fifteen chains of a dozen units each, and a
//! board that could not make the cost's mana whatever it converted is turned
//! away before the first ([`enough`]). Past the limit it under-counts, which
//! is `manaplan`'s third rule. Nothing here is random and nothing is iterated
//! out of a hash: the same board plans the same taps.
//!
//! Restricted mana is left alone: what the engine may spend it on is a rules
//! question (`manaplan`'s second rule), so a pool holding any plans no
//! conversion.

use std::collections::BTreeSet;

use baylee_core::mana::{ManaColor, ManaCost};
use baylee_view::ManaPoolView;

use super::{ColorMask, Plan, Source, Step, Unit, augment, mask_of, needs, permitted, slot};

/// How many permanents' conversions one plan reads.
const MAX_CONVERTERS: usize = 3;

/// A permanent's paid mana ability: what it spends and what it makes.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Converter {
    /// Which action activates it. Always a printed ability's.
    pub tap: super::Tap,
    /// The mana it spends, floating as it is activated.
    pub cost: ManaCost,
    /// What it makes: every colour listed, one mana each, for a
    /// [`Self::bundle`], or the colours each pick chooses among.
    pub colors: Vec<ManaColor>,
    /// Whether every colour listed arrives and nothing is asked: a Signet.
    pub bundle: bool,
    /// How many colours the engine asks for, one mana each: one for
    /// Shimmering Grotto, two for Mystic Gate, none for a bundle.
    pub picks: u8,
}

impl Converter {
    /// The mana it makes, one entry a mana, each holding what that mana may be.
    fn made(&self) -> Vec<ColorMask> {
        if self.bundle {
            self.colors.iter().map(|c| ColorMask::of(*c)).collect()
        } else {
            vec![mask_of(&self.colors); usize::from(self.picks)]
        }
    }

    /// How much mana it makes.
    #[must_use]
    pub fn mana(&self) -> usize {
        if self.bundle {
            self.colors.len()
        } else {
            usize::from(self.picks)
        }
    }

    /// What one activation leaves the pool with beyond what it spent, at its
    /// best: no more can be asked of this permanent.
    fn gain(&self) -> usize {
        needs(&self.cost, false).map_or(0, |cost| self.mana().saturating_sub(cost.len()))
    }
}

/// The taps and activations that pay `cost` through at least one converter,
/// or `None` when no chain of them does. See the module header.
pub(super) fn plan(cost: &ManaCost, pool: &ManaPoolView, sources: &[Source]) -> Option<Plan> {
    let mut converters: Vec<usize> = sources
        .iter()
        .enumerate()
        .filter(|(_, source)| source.converter.is_some())
        .map(|(index, _)| index)
        .collect();
    if converters.is_empty() || pool.restricted_total() > 0 || !enough(cost, pool, sources) {
        return None;
    }
    converters.sort_by_key(|&index| (sources[index].priced, sources[index].preserve, index));
    converters.truncate(MAX_CONVERTERS);
    for size in 1..=converters.len() {
        for chain in chains(&converters, size) {
            for generic_twobrid in [false, true] {
                let found = attempt(cost, pool, sources, &chain, generic_twobrid);
                if found.is_some() {
                    return found;
                }
            }
        }
    }
    None
}

/// Whether the board could make as much mana as `cost` asks, by any chain at
/// all: what floats, and each permanent at its best. Most costs a chain is
/// asked about are simply too dear, and this answers those without a matching.
fn enough(cost: &ManaCost, pool: &ManaPoolView, sources: &[Source]) -> bool {
    let Some(asked) = needs(cost, false) else {
        return false;
    };
    let made: usize = sources
        .iter()
        .map(|source| {
            let free = if source.colors.is_empty() {
                0
            } else {
                source.units()
            };
            free.max(source.converter.as_ref().map_or(0, Converter::gain))
        })
        .sum();
    u64::try_from(made)
        .unwrap_or(u64::MAX)
        .saturating_add(pool.total())
        >= u64::try_from(asked.len()).unwrap_or(u64::MAX)
}

/// Every order of `size` of `converters`: the chains of that length.
fn chains(converters: &[usize], size: usize) -> Vec<Vec<usize>> {
    if size == 0 {
        return vec![Vec::new()];
    }
    let mut out = Vec::new();
    for (at, &first) in converters.iter().enumerate() {
        let mut rest = converters.to_vec();
        rest.remove(at);
        for mut tail in chains(&rest, size - 1) {
            tail.insert(0, first);
            out.push(tail);
        }
    }
    out
}

/// One chain's matching and the steps it comes to, if the engine would pay
/// them out as planned.
fn attempt(
    cost: &ManaCost,
    pool: &ManaPoolView,
    sources: &[Source],
    chain: &[usize],
    generic_twobrid: bool,
) -> Option<Plan> {
    // The spell's demands and then each converter's; `owner` is the place in
    // the chain a demand pays for, past every place for the spell's own.
    let mut demands: Vec<ColorMask> = needs(cost, generic_twobrid)?
        .into_iter()
        .map(|need| permitted(need, pool))
        .collect();
    let mut owner = vec![usize::MAX; demands.len()];
    for (at, &index) in chain.iter().enumerate() {
        for need in needs(&sources[index].converter.as_ref()?.cost, false)? {
            demands.push(permitted(need, pool));
            owner.push(at);
        }
    }
    let mut rest = sources.to_vec();
    for &index in chain {
        rest[index].colors.clear();
        rest[index].dear = None;
    }
    let mut units = super::units(pool, &rest, demands.len(), false);
    // The place in the chain a unit was made at; `None` for plain mana.
    let mut made_at: Vec<Option<usize>> = vec![None; units.len()];
    for (at, &index) in chain.iter().enumerate() {
        for colors in sources[index].converter.as_ref()?.made() {
            units.push(Unit {
                colors,
                from: Some(index),
            });
            made_at.push(Some(at));
        }
    }
    if demands.len() > units.len() {
        return None;
    }
    let taken = matching(&demands, &owner, &units, &made_at, sources)?;
    let steps = ordered(&demands, &owner, &units, &taken, &rest, sources, chain);
    holds(cost, pool, &steps, sources).then_some(Plan { steps, cost: *cost })
}

/// Every demand matched to a unit it may take: `taken[unit]` is the demand.
fn matching(
    demands: &[ColorMask],
    owner: &[usize],
    units: &[Unit],
    made_at: &[Option<usize>],
    sources: &[Source],
) -> Option<Vec<Option<usize>>> {
    // What floats, then clean taps, then what a conversion makes, so a
    // converter's mana is spent on what nothing else can pay.
    let mut order: Vec<usize> = (0..units.len()).collect();
    order.sort_by_key(|&unit| {
        let from = units[unit].from;
        (
            made_at[unit].is_some(),
            from.is_some(),
            from.is_some_and(|index| sources[index].priced),
            from.map_or(0, |index| sources[index].preserve),
            units[unit].colors.count(),
            from.unwrap_or(0),
        )
    });
    let mut asked: Vec<usize> = (0..demands.len()).collect();
    asked.sort_by_key(|&demand| demands[demand].count());
    let fits = |demand: usize, unit: usize| {
        demands[demand].overlaps(units[unit].colors)
            && made_at[unit].is_none_or(|made| made < owner[demand])
    };
    let mut taken: Vec<Option<usize>> = vec![None; units.len()];
    for &demand in &asked {
        let mut seen = vec![false; units.len()];
        if !augment(demand, &fits, &order, &mut taken, &mut seen) {
            return None;
        }
    }
    Some(taken)
}

/// The steps a matching calls for: converter by converter, each one's payers
/// first, and then the plain taps, in source order.
fn ordered(
    demands: &[ColorMask],
    owner: &[usize],
    units: &[Unit],
    taken: &[Option<usize>],
    rest: &[Source],
    sources: &[Source],
    chain: &[usize],
) -> Vec<Step> {
    let plain = super::steps(demands, units, taken, rest, false).steps;
    let step_of = |index: usize| {
        plain
            .iter()
            .find(|step| step.source == sources[index].id)
            .copied()
    };
    let mut steps = Vec::new();
    let mut tapped: BTreeSet<usize> = chain.iter().copied().collect();
    for (at, &index) in chain.iter().enumerate() {
        for (unit, holder) in units.iter().zip(taken) {
            if let (Some(from), Some(demand)) = (unit.from, *holder)
                && owner[demand] == at
                && tapped.insert(from)
            {
                steps.extend(step_of(from));
            }
        }
        steps.extend(conversion(index, demands, units, taken, sources));
    }
    let used: BTreeSet<usize> = units
        .iter()
        .zip(taken)
        .filter_map(|(unit, holder)| holder.and(unit.from))
        .collect();
    steps.extend(
        used.difference(&tapped)
            .copied()
            .collect::<Vec<_>>()
            .into_iter()
            .filter_map(step_of),
    );
    steps
}

/// The step that activates `index`'s converter, with the colours its mana was
/// matched on.
fn conversion(
    index: usize,
    demands: &[ColorMask],
    units: &[Unit],
    taken: &[Option<usize>],
    sources: &[Source],
) -> Option<Step> {
    let source = &sources[index];
    let converter = source.converter.as_ref()?;
    let mut picks = units
        .iter()
        .zip(taken)
        .filter(|(unit, _)| unit.from == Some(index))
        .map(|(unit, holder)| {
            holder
                .and_then(|demand| {
                    unit.colors
                        .colors()
                        .find(|color| demands[demand].holds(*color))
                })
                .or_else(|| unit.colors.colors().next())
        });
    let (color, second) = if converter.bundle {
        (None, None)
    } else {
        (picks.next().flatten(), picks.next().flatten())
    };
    Some(Step {
        source: source.id,
        tap: converter.tap,
        color,
        second,
    })
}

/// Whether `steps` leave `cost` payable, the pool spent as the engine spends
/// it: the matching chose what pays, the engine chooses what is spent.
fn holds(cost: &ManaCost, pool: &ManaPoolView, steps: &[Step], sources: &[Source]) -> bool {
    let mut floating = *pool;
    for step in steps {
        let Some(after) = after_step(&floating, step, sources) else {
            return false;
        };
        floating = after;
    }
    super::plain_plan(cost, &floating, &[]).is_some()
}

/// The pool once `step` is made: what a converter spends taken out, and what
/// the step makes added. `None` when a converter's cost does not float.
#[must_use]
pub fn after_step(pool: &ManaPoolView, step: &Step, sources: &[Source]) -> Option<ManaPoolView> {
    let mut out = *pool;
    if let Some(converter) = sources
        .iter()
        .find(|source| source.id == step.source)
        .and_then(|source| source.converter.as_ref())
        .filter(|converter| converter.tap == step.tap)
    {
        out = spent(&out, &converter.cost)?;
    }
    for (color, count) in made_by(step, sources) {
        *slot(&mut out, color) = slot(&mut out, color).saturating_add(count);
    }
    Some(out)
}

/// The pool after `cost` is paid from it as the engine's auto-payment pays:
/// the narrow symbols first and the generic part in `ManaColor` order, which
/// is `mana_pay::generic`'s, white first and colourless last. A hybrid takes
/// the first of its colours in that order where the engine tries the printed
/// half first; both float only in a pool a plan did not make, and a walk that
/// is wrong there stops the taps, it spends nothing that was not asked for.
fn spent(pool: &ManaPoolView, cost: &ManaCost) -> Option<ManaPoolView> {
    let mut out = *pool;
    let mut demands: Vec<ColorMask> = needs(cost, false)?
        .into_iter()
        .map(|need| permitted(need, pool))
        .collect();
    demands.sort_by_key(|demand| demand.count());
    for demand in demands {
        let color = ManaColor::ALL
            .into_iter()
            .find(|color| demand.holds(*color) && *slot(&mut out, *color) > 0)?;
        *slot(&mut out, color) -= 1;
    }
    Some(out)
}

/// What one step makes, by the planner's reading of its source: the colours
/// and counts it adds to the pool, and nothing for a tap this reading does
/// not know, so a count built on it never over-states.
#[must_use]
pub fn made_by(step: &Step, sources: &[Source]) -> Vec<(ManaColor, u32)> {
    let Some(source) = sources.iter().find(|s| s.id == step.source) else {
        return Vec::new();
    };
    if source.tap == step.tap && !source.colors.is_empty() {
        if source.bundle {
            return source.colors.iter().map(|&c| (c, 1)).collect();
        }
        let color = match (step.color, source.colors.as_slice()) {
            (Some(color), colors) if colors.contains(&color) => color,
            (None, [only]) => *only,
            _ => return Vec::new(),
        };
        return vec![(color, u32::from(source.amount.max(1)))];
    }
    if let Some(converter) = source.converter.as_ref().filter(|c| c.tap == step.tap) {
        if converter.bundle {
            return converter.colors.iter().map(|&c| (c, 1)).collect();
        }
        return [step.color, step.second]
            .into_iter()
            .flatten()
            .filter(|color| converter.colors.contains(color))
            .take(usize::from(converter.picks))
            .map(|color| (color, 1))
            .collect();
    }
    match &source.dear {
        Some(dear) if dear.tap == step.tap => {
            let color = match (step.color, dear.colors.as_slice()) {
                (Some(color), colors) if colors.contains(&color) => color,
                (None, [only]) => *only,
                _ => return Vec::new(),
            };
            vec![(color, 1)]
        }
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Tap, plan};
    use super::*;
    use baylee_core::ids::ObjectId;

    fn id(slot: u32) -> ObjectId {
        ObjectId::new(slot, 0)
    }

    fn land(slot: u32, color: ManaColor) -> Source {
        Source::fixed(id(slot), Tap::Intrinsic, color)
    }

    fn cost(src: &str) -> ManaCost {
        ManaCost::try_parse(src).expect("a valid cost")
    }

    fn any_color() -> Vec<ManaColor> {
        ManaColor::ALL[..5].to_vec()
    }

    /// `{T}: Add {C}` beside `{1}, {T}: Add one mana of any color`.
    fn grotto(slot: u32) -> Source {
        Source {
            converter: Some(Converter {
                tap: Tap::Ability(1),
                cost: cost("{1}"),
                colors: any_color(),
                bundle: false,
                picks: 1,
            }),
            ..Source::fixed(id(slot), Tap::Ability(0), ManaColor::Colorless)
        }
    }

    /// `{1}, {T}: Add {W}{U}`, and no free mana at all.
    fn signet(slot: u32) -> Source {
        Source::converting(
            id(slot),
            Converter {
                tap: Tap::Ability(0),
                cost: cost("{1}"),
                colors: vec![ManaColor::White, ManaColor::Blue],
                bundle: true,
                picks: 0,
            },
        )
    }

    /// `{W/U}, {T}: Add {W}{W}, {W}{U} or {U}{U}` beside `{T}: Add {C}`.
    fn mystic_gate(slot: u32) -> Source {
        Source {
            converter: Some(Converter {
                tap: Tap::Ability(1),
                cost: cost("{W/U}"),
                colors: vec![ManaColor::White, ManaColor::Blue],
                bundle: false,
                picks: 2,
            }),
            ..Source::fixed(id(slot), Tap::Ability(0), ManaColor::Colorless)
        }
    }

    fn swamp_mountain(slot: u32) -> Source {
        Source {
            colors: vec![ManaColor::Black, ManaColor::Red],
            ..land(slot, ManaColor::Black)
        }
    }

    fn order(found: &Plan) -> Vec<u32> {
        found.steps.iter().map(|s| s.source.slot()).collect()
    }

    fn none() -> ManaPoolView {
        ManaPoolView::default()
    }

    /// The board of the report: Meteor Storm's `{R}{G}`, Canyon Slough,
    /// Crucible of the Spirit Dragon, Shimmering Grotto.
    #[test]
    fn the_meteor_storm_board_converts_first_and_taps_the_red_last() {
        let sources = [
            swamp_mountain(1),
            Source::fixed(id(2), Tap::Ability(0), ManaColor::Colorless),
            grotto(3),
        ];
        let found = plan(&cost("{R}{G}"), &none(), &sources).expect("it is payable");
        assert_eq!(
            order(&found),
            [2, 3, 1],
            "the Crucible's {{C}} pays the Grotto, and the Slough's {{R}} is made after, or \
             the Grotto's {{1}} spends it"
        );
        assert_eq!(found.steps[1].tap, Tap::Ability(1), "the paid ability");
        assert_eq!(found.steps[1].color, Some(ManaColor::Green));
        assert_eq!(found.steps[2].color, Some(ManaColor::Red));
        assert_eq!(found.cost, cost("{R}{G}"));
    }

    /// The house re-plans after every tap: the Crucible's mana is floating
    /// now, and the same chain goes on from there.
    #[test]
    fn a_plan_goes_on_from_the_mana_it_already_made() {
        let sources = [swamp_mountain(1), grotto(3)];
        let pool = ManaPoolView {
            colorless: 1,
            ..none()
        };
        let found = plan(&cost("{R}{G}"), &pool, &sources).expect("the {C} floats");
        assert_eq!(order(&found), [3, 1]);
        let pool = ManaPoolView { green: 1, ..none() };
        assert_eq!(
            order(&plan(&cost("{R}{G}"), &pool, &[swamp_mountain(1)]).unwrap()),
            [1]
        );
    }

    #[test]
    fn a_filter_land_cannot_pay_for_itself() {
        assert_eq!(plan(&cost("{G}"), &none(), &[grotto(3)]), None);
        assert_eq!(plan(&cost("{1}"), &none(), &[signet(4)]), None);
        assert!(
            plan(&cost("{1}"), &none(), &[grotto(3)]).is_some(),
            "its free {{C}} still pays a generic mana"
        );
    }

    /// Two converters each waiting on the other's mana pay for nothing, and
    /// a free tap beside one is what starts the chain.
    #[test]
    fn two_filter_lands_cannot_pay_for_each_other() {
        assert_eq!(plan(&cost("{1}"), &none(), &[signet(3), signet(4)]), None);
        let found = plan(&cost("{G}"), &none(), &[grotto(3), grotto(4)]).expect("one taps free");
        assert_eq!(order(&found), [4, 3]);
        assert_eq!(found.steps[0].tap, Tap::Ability(0), "the free {{C}}");
        assert_eq!(found.steps[1].tap, Tap::Ability(1), "the paid {{G}}");
    }

    /// The Forest pays the Grotto, the Grotto's `{W}` pays the Gate, and the
    /// Gate makes what no other permanent could.
    #[test]
    fn two_converters_chain() {
        let sources = [land(1, ManaColor::Green), grotto(2), mystic_gate(3)];
        let found = plan(&cost("{W}{U}"), &none(), &sources).expect("a chain of two");
        assert_eq!(order(&found), [1, 2, 3]);
        assert_eq!(
            found.steps[1].color,
            Some(ManaColor::White),
            "the Grotto makes what the Gate's {{W/U}} takes"
        );
        let gate = found.steps[2];
        let mut colors = [gate.color.unwrap(), gate.second.unwrap()];
        colors.sort_by_key(|c| c.index());
        assert_eq!(
            colors,
            [ManaColor::White, ManaColor::Blue],
            "one pick a mana"
        );
    }

    /// A Signet nets a mana, and its two colours are both there.
    #[test]
    fn a_signet_adds_a_mana_and_both_its_colours() {
        let sources = [land(1, ManaColor::Green), signet(2)];
        let found = plan(&cost("{W}{U}"), &none(), &sources).expect("the Forest pays the Signet");
        assert_eq!(order(&found), [1, 2]);
        assert_eq!(found.steps[1].color, None, "a bundle is asked nothing");
    }

    /// `{1}` is paid out of what floats, white first and colourless last, and
    /// the seat does not choose: with `{W}` floating beside the Crucible's
    /// `{C}`, the Grotto's cost takes the `{W}` the spell was counting on.
    /// The matching would call that board paid; the walk does not.
    #[test]
    fn floating_mana_the_conversion_would_spend_is_not_counted_on() {
        let sources = [
            Source::fixed(id(2), Tap::Ability(0), ManaColor::Colorless),
            grotto(3),
        ];
        let pool = ManaPoolView { white: 1, ..none() };
        assert_eq!(plan(&cost("{W}{G}"), &pool, &sources), None);
        let pool = ManaPoolView { green: 1, ..none() };
        assert_eq!(plan(&cost("{R}{G}"), &pool, &sources), None);
    }

    /// The same spending, on the mana a chain makes for itself: the second
    /// Signet's `{1}` takes the first's `{W}`, so two Signets and two Forests
    /// come to `{W}{U}{U}` and never to two white.
    #[test]
    fn a_chain_is_answered_only_for_what_the_engine_leaves_it() {
        let sources = [
            land(1, ManaColor::Green),
            land(2, ManaColor::Green),
            signet(3),
            signet(4),
        ];
        assert_eq!(plan(&cost("{W}{W}{U}{U}"), &none(), &sources), None);
        let found = plan(&cost("{W}{U}{U}"), &none(), &sources).expect("what is left pays this");
        assert_eq!(order(&found), [2, 3, 1, 4], "each Signet after its Forest");
    }

    /// Restricted mana is a rules question, and a chain does not reason
    /// about what it may be spent on.
    #[test]
    fn a_pool_with_restricted_mana_converts_nothing() {
        let mut pool = none();
        pool.restricted[ManaColor::White.index()] = 1;
        let sources = [swamp_mountain(1), land(2, ManaColor::Green), grotto(3)];
        assert_eq!(plan(&cost("{W}{U}{R}"), &pool, &sources), None);
    }

    /// Plain lands plan what they planned: a converter is asked only when
    /// the matcher has said no.
    #[test]
    fn a_board_that_pays_without_converting_is_not_converted() {
        let sources = [land(1, ManaColor::Green), grotto(2)];
        let found = plan(&cost("{1}{G}"), &none(), &sources).expect("both tap for free");
        assert_eq!(order(&found), [1, 2]);
        assert_eq!(found.steps[1].tap, Tap::Ability(0), "the free {{C}}");
    }

    #[test]
    fn a_dear_board_is_turned_away_before_any_chain() {
        let sources = [land(1, ManaColor::Green), grotto(2)];
        assert!(!enough(&cost("{5}"), &none(), &sources));
        assert_eq!(plan(&cost("{5}"), &none(), &sources), None);
    }

    /// More converters than are read: the plan under-counts and never
    /// answers wrongly, and the same board always answers the same.
    #[test]
    fn past_the_limit_the_search_stays_bounded_and_the_answer_stays_the_same() {
        let sources: Vec<Source> = (0..8)
            .map(signet)
            .chain([land(20, ManaColor::Green), land(21, ManaColor::Green)])
            .collect();
        let first = plan(&cost("{W}{U}{U}"), &none(), &sources);
        assert!(first.is_some(), "two Signets, each paid by a Forest");
        assert_eq!(first, plan(&cost("{W}{U}{U}"), &none(), &sources));
        assert_eq!(chains(&[0, 1, 2], 3).len(), 6);
    }

    #[test]
    fn a_walk_spends_what_a_converter_costs() {
        let sources = [grotto(3)];
        let pool = ManaPoolView {
            colorless: 1,
            ..none()
        };
        let step = Step {
            source: id(3),
            tap: Tap::Ability(1),
            color: Some(ManaColor::Green),
            second: None,
        };
        let after = after_step(&pool, &step, &sources).expect("the {C} floats");
        assert_eq!((after.colorless, after.green), (0, 1));
        assert_eq!(after_step(&none(), &step, &sources), None);
    }
}
