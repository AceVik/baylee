//! Which lands to tap, and in what colours.
//!
//! The engine offers a spell as `castable` only when the mana is *already
//! floating* (`LegalActions::castable`, checked against the pool in
//! `casting::can_cast`). That is the correct rules answer and a miserable
//! thing to play against: a hand full of spells and five untapped lands looks,
//! to a client, like a hand with nothing to do. Every physical player solves
//! this without thinking — they look at the cost, look at their lands, and tap
//! the ones that work.
//!
//! This module is that look. Given a cost, what is floating, and the sources
//! the engine *itself* listed as tappable right now, it returns the taps that
//! make the spell castable — or `None`, which is a real answer too: it is what
//! greys the card out.
//!
//! Three rules keep it honest, and they are the reason this is worth reading
//! before changing:
//!
//! 1. **Every step is an action the engine offered.** A [`Source`] is built
//!    from `LegalActions`, never from the client's own idea of what a land
//!    does, and the executor re-checks each step against the *current*
//!    `LegalActions` before sending it. A drifted plan aborts; it never
//!    guesses.
//! 2. **It never spends what a player would want to decide.** Phyrexian mana
//!    is read as its colour and never as two life; restricted mana (Cavern of
//!    Souls) is not counted, because what it may pay for is a rules question
//!    this side of the wire cannot answer.
//! 3. **It under-counts rather than over-counts.** A source that makes two
//!    mana *of one chosen colour* is worth one mana here, because two units
//!    that must share a colour are not two independent units and pretending
//!    otherwise would build a plan the engine rejects halfway through. The
//!    cost of being wrong in this direction is one extra land tapped.

use std::collections::BTreeSet;

use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::ObjectId;
use baylee_core::mana::{ManaColor, ManaCost, ManaSymbol};
use baylee_view::ManaPoolView;

/// How the client asks one source for its mana.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tap {
    /// `PlayerAction::ActivateManaAbility` — the engine's CR 305.6 shortcut
    /// for a land with exactly one basic land type.
    Intrinsic,
    /// `PlayerAction::ActivateAbility` — a printed mana ability, by index.
    Ability(u32),
}

/// A permanent the engine says can be tapped for mana right now.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Source {
    /// The permanent.
    pub id: ObjectId,
    /// Which action taps it.
    pub tap: Tap,
    /// What it may produce. One entry means there is nothing to choose.
    pub colors: Vec<ManaColor>,
    /// How much one activation makes.
    pub amount: u8,
}

impl Source {
    /// A source with no choice to make.
    #[must_use]
    pub fn fixed(id: ObjectId, tap: Tap, color: ManaColor) -> Self {
        Self {
            id,
            tap,
            colors: vec![color],
            amount: 1,
        }
    }

    /// How many independent mana this is worth to a plan.
    ///
    /// Only a source with a single colour can be counted more than once:
    /// "add two mana of any one colour" is two mana that must match, and a
    /// matcher that treated them as independent would happily plan `{W}{U}`
    /// out of one of them.
    const fn units(&self) -> usize {
        if self.colors.len() == 1 {
            self.amount as usize
        } else {
            1
        }
    }
}

/// One activation the plan calls for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Step {
    /// The permanent to tap.
    pub source: ObjectId,
    /// Which action taps it.
    pub tap: Tap,
    /// The colour to answer with, when the ability asks. `None` when the
    /// source has only one colour and the engine will not ask.
    pub color: Option<ManaColor>,
}

/// The taps that make a spell castable.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Plan {
    /// In the order they should be sent.
    pub steps: Vec<Step>,
}

impl Plan {
    /// Whether the mana is already floating and nothing needs tapping.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// How many permanents this taps.
    #[must_use]
    pub fn taps(&self) -> usize {
        self.steps.len()
    }
}

/// Finds the taps that pay `cost`, or `None` when nothing here can.
///
/// `pool` is what is already floating and is spent first — it costs nothing
/// and it is about to empty at the end of the step anyway.
#[must_use]
pub fn plan(cost: &ManaCost, pool: &ManaPoolView, sources: &[Source]) -> Option<Plan> {
    // `{2/C}` reads two ways and only the player knows which they meant, so
    // both are tried: the coloured half first (it is one mana, not two), then
    // the generic one. Everything else has a single reading.
    for generic_twobrid in [false, true] {
        let needs = needs(cost, generic_twobrid)?;
        if let Some(found) = assign(&needs, pool, sources) {
            return Some(found);
        }
    }
    None
}

/// A source of one mana in the matching: either floating, or a tap.
struct Unit {
    colors: ColorMask,
    /// The source this unit comes from; `None` for floating mana.
    from: Option<usize>,
}

/// The six mana colours as a bitmask, so a candidacy test is one `&`.
#[derive(Clone, Copy, PartialEq, Eq)]
struct ColorMask(u8);

impl ColorMask {
    const NONE: Self = Self(0);
    /// Every colour, which is what a generic symbol accepts.
    const ANY: Self = Self(0b11_1111);

    fn of(color: ManaColor) -> Self {
        Self(1 << color.index())
    }

    fn with(self, color: ManaColor) -> Self {
        Self(self.0 | Self::of(color).0)
    }

    const fn holds(self, color: ManaColor) -> bool {
        self.0 & (1 << color.index() as u8) != 0
    }

    const fn overlaps(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }

    const fn count(self) -> u32 {
        self.0.count_ones()
    }

    fn colors(self) -> impl Iterator<Item = ManaColor> {
        ManaColor::ALL.into_iter().filter(move |c| self.holds(*c))
    }
}

fn mask_of(colors: &[ManaColor]) -> ColorMask {
    colors.iter().fold(ColorMask::NONE, |m, c| m.with(*c))
}

fn mask_of_set(set: ColorSet) -> ColorMask {
    set.iter()
        .fold(ColorMask::NONE, |m, c| m.with(ManaColor::from_color(c)))
}

/// Turns a cost into one entry per mana it demands.
///
/// `None` for a cost this side of the wire must not guess at: `{X}` is a
/// number the player chooses, `{S}` is a property of the *source* rather than
/// of the mana, and the silver-bordered symbols are not mana at all.
fn needs(cost: &ManaCost, generic_twobrid: bool) -> Option<Vec<ColorMask>> {
    let mut out = Vec::new();
    for symbol in cost.symbols() {
        match symbol {
            ManaSymbol::Generic(n) => {
                for _ in 0..n {
                    out.push(ColorMask::ANY);
                }
            }
            ManaSymbol::Colorless => out.push(ColorMask::of(ManaColor::Colorless)),
            ManaSymbol::White => out.push(ColorMask::of(ManaColor::White)),
            ManaSymbol::Blue => out.push(ColorMask::of(ManaColor::Blue)),
            ManaSymbol::Black => out.push(ColorMask::of(ManaColor::Black)),
            ManaSymbol::Red => out.push(ColorMask::of(ManaColor::Red)),
            ManaSymbol::Green => out.push(ColorMask::of(ManaColor::Green)),
            // A hybrid is one mana of either colour. Phyrexian is read as its
            // colour only: paying two life is a decision, not a shortcut.
            ManaSymbol::Hybrid(pair) | ManaSymbol::HybridPhyrexian(pair) => {
                out.push(mask_of_set(ColorSet::of_pair(pair)));
            }
            ManaSymbol::Phyrexian(color) => {
                out.push(ColorMask::of(ManaColor::from_color(color)));
            }
            ManaSymbol::TwoOrColor(color) => {
                if generic_twobrid {
                    out.push(ColorMask::ANY);
                    out.push(ColorMask::ANY);
                } else {
                    out.push(ColorMask::of(ManaColor::from_color(color)));
                }
            }
            ManaSymbol::Snow
            | ManaSymbol::Variable(_)
            | ManaSymbol::HalfGeneric
            | ManaSymbol::Infinite => return None,
        }
    }
    Some(out)
}

/// Every mana available: what is floating, then what could be tapped.
///
/// Floating mana comes first so the preference order below reaches for it
/// before it taps anything.
fn units(pool: &ManaPoolView, sources: &[Source]) -> Vec<Unit> {
    let mut units = Vec::new();
    for (color, count) in [
        (ManaColor::White, pool.white),
        (ManaColor::Blue, pool.blue),
        (ManaColor::Black, pool.black),
        (ManaColor::Red, pool.red),
        (ManaColor::Green, pool.green),
        (ManaColor::Colorless, pool.colorless),
    ] {
        for _ in 0..count {
            units.push(Unit {
                colors: ColorMask::of(color),
                from: None,
            });
        }
    }
    for (index, source) in sources.iter().enumerate() {
        let colors = mask_of(&source.colors);
        if colors == ColorMask::NONE {
            continue;
        }
        for _ in 0..source.units() {
            units.push(Unit {
                colors,
                from: Some(index),
            });
        }
    }
    units
}

/// Matches every demand to a mana, and reports the taps that implies.
///
/// This is Kuhn's algorithm on a tiny bipartite graph — a dozen demands
/// against a dozen sources — which is exact rather than greedy: if any set of
/// taps pays the cost, it finds one. Greedy is what produces the classic
/// misplay of tapping the dual for the generic pip and then having no black.
///
/// Two orderings turn "a matching" into "the matching a player would make":
/// demands are taken most-constrained first, and each demand tries floating
/// mana before any tap, then the *least* flexible source that fits — so the
/// land that only makes green pays the green pip and the one that makes
/// anything is still untapped afterwards.
fn assign(needs: &[ColorMask], pool: &ManaPoolView, sources: &[Source]) -> Option<Plan> {
    let units = units(pool, sources);
    if needs.len() > units.len() {
        return None;
    }

    let mut order: Vec<usize> = (0..units.len()).collect();
    order.sort_by_key(|&u| {
        let unit = &units[u];
        (
            // Floating first: it is free and it empties at end of step.
            usize::from(unit.from.is_some()),
            unit.colors.count(),
            unit.from.unwrap_or(0),
        )
    });

    let mut demands: Vec<usize> = (0..needs.len()).collect();
    demands.sort_by_key(|&n| needs[n].count());

    // `taken[unit]` is the demand holding it.
    let mut taken: Vec<Option<usize>> = vec![None; units.len()];
    for &demand in &demands {
        let mut seen = vec![false; units.len()];
        if !augment(demand, needs, &units, &order, &mut taken, &mut seen) {
            return None;
        }
    }

    Some(steps(needs, &units, &taken, sources))
}

/// One augmenting step of Kuhn's algorithm.
fn augment(
    demand: usize,
    needs: &[ColorMask],
    units: &[Unit],
    order: &[usize],
    taken: &mut Vec<Option<usize>>,
    seen: &mut Vec<bool>,
) -> bool {
    for &unit in order {
        if seen[unit] || !needs[demand].overlaps(units[unit].colors) {
            continue;
        }
        seen[unit] = true;
        let free = taken[unit].is_none();
        if free
            || augment(
                taken[unit].expect("occupied"),
                needs,
                units,
                order,
                taken,
                seen,
            )
        {
            taken[unit] = Some(demand);
            return true;
        }
    }
    false
}

/// Turns a matching into the taps it calls for.
fn steps(needs: &[ColorMask], units: &[Unit], taken: &[Option<usize>], sources: &[Source]) -> Plan {
    // A source may back several units; it is tapped once, and the colour it
    // is asked for is the one its first assigned unit was matched on.
    let mut used: BTreeSet<usize> = BTreeSet::new();
    let mut chosen: Vec<Option<ManaColor>> = vec![None; sources.len()];
    for (unit, holder) in units.iter().zip(taken) {
        let (Some(source), Some(demand)) = (unit.from, *holder) else {
            continue;
        };
        used.insert(source);
        if chosen[source].is_none() {
            // The demand narrows the choice; anything in the overlap pays it,
            // and the lowest is as good as any and keeps the plan stable.
            chosen[source] = unit
                .colors
                .colors()
                .find(|c| needs[demand].holds(*c))
                .or_else(|| unit.colors.colors().next());
        }
    }

    Plan {
        steps: used
            .into_iter()
            .map(|index| {
                let source = &sources[index];
                Step {
                    source: source.id,
                    tap: source.tap,
                    // Only a source with a real choice is ever asked.
                    color: (source.colors.len() > 1)
                        .then(|| chosen[index].unwrap_or_else(|| source.colors[0])),
                }
            })
            .collect(),
    }
}

/// The colour a land with exactly one basic land type taps for (CR 305.6).
///
/// The same rule the engine applies in `casting::intrinsic_mana`, read off the
/// *projected* subtypes — which is the only correct source, because an
/// animated or type-changed land taps for what it is now, not for what it was
/// printed as. A land with two basic types has two mana abilities and the
/// engine does not offer the shortcut for it, so neither does this.
#[must_use]
pub fn basic_land_color(subtypes: &baylee_core::types::SubtypeSet) -> Option<ManaColor> {
    use baylee_core::generated::subtypes::land;
    let mut only = None;
    for (subtype, color) in [
        (land::PLAINS, ManaColor::White),
        (land::ISLAND, ManaColor::Blue),
        (land::SWAMP, ManaColor::Black),
        (land::MOUNTAIN, ManaColor::Red),
        (land::FOREST, ManaColor::Green),
    ] {
        if subtypes.contains(subtype) {
            if only.is_some() {
                return None;
            }
            only = Some(color);
        }
    }
    only
}

/// Every colour in a [`ColorSet`], as mana colours.
#[must_use]
pub fn colors_of(set: ColorSet) -> Vec<ManaColor> {
    set.iter().map(ManaColor::from_color).collect()
}

/// The mana colour a [`Color`] is.
#[must_use]
pub const fn mana_color(color: Color) -> ManaColor {
    ManaColor::from_color(color)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn land(id: u32, color: ManaColor) -> Source {
        Source::fixed(ObjectId::new(id, 0), Tap::Intrinsic, color)
    }

    fn any_land(id: u32) -> Source {
        Source {
            id: ObjectId::new(id, 0),
            tap: Tap::Ability(0),
            colors: ManaColor::ALL.to_vec(),
            amount: 1,
        }
    }

    fn cost(src: &str) -> ManaCost {
        ManaCost::try_parse(src).expect("a valid cost")
    }

    fn empty() -> ManaPoolView {
        ManaPoolView::default()
    }

    fn tapped(plan: &Plan) -> Vec<u32> {
        let mut ids: Vec<u32> = plan.steps.iter().map(|s| s.source.slot()).collect();
        ids.sort_unstable();
        ids
    }

    #[test]
    fn a_cost_already_floating_needs_no_taps() {
        let pool = ManaPoolView {
            green: 1,
            colorless: 1,
            ..empty()
        };
        let found = plan(&cost("{1}{G}"), &pool, &[]).expect("already payable");
        assert!(found.is_empty());
    }

    #[test]
    fn it_taps_exactly_as_many_lands_as_the_cost_asks_for() {
        let sources = [
            land(1, ManaColor::Green),
            land(2, ManaColor::Green),
            land(3, ManaColor::Green),
            land(4, ManaColor::Green),
        ];
        let found = plan(&cost("{2}{G}"), &empty(), &sources).expect("three lands is enough");
        assert_eq!(found.taps(), 3);
    }

    #[test]
    fn not_enough_lands_is_a_real_answer() {
        let sources = [land(1, ManaColor::Green)];
        assert!(plan(&cost("{1}{G}"), &empty(), &sources).is_none());
    }

    #[test]
    fn the_wrong_colours_are_as_good_as_no_lands() {
        let sources = [
            land(1, ManaColor::Red),
            land(2, ManaColor::Red),
            land(3, ManaColor::Red),
        ];
        assert!(plan(&cost("{2}{G}"), &empty(), &sources).is_none());
        // …and the generic part alone is payable by anything.
        assert!(plan(&cost("{2}"), &empty(), &sources).is_some());
    }

    /// The misplay this module exists to avoid: a greedy matcher pays the
    /// generic pip with the only land that makes black and then cannot pay
    /// `{B}`. Kuhn's algorithm backtracks, so it does not happen.
    #[test]
    fn the_generic_pip_does_not_eat_the_only_black_source() {
        let sources = [
            land(1, ManaColor::Black), // the only black
            land(2, ManaColor::Red),
        ];
        let found =
            plan(&cost("{1}{B}"), &empty(), &sources).expect("both lands, correctly paired");
        assert_eq!(tapped(&found), vec![1, 2]);
    }

    /// The preference, rather than the correctness: with a choice, the pip is
    /// paid by the land that can do nothing else, and the flexible one is left
    /// alone.
    #[test]
    fn a_flexible_land_is_saved_for_when_it_is_needed() {
        let sources = [any_land(1), land(2, ManaColor::Green)];
        let found = plan(&cost("{G}"), &empty(), &sources).expect("the forest pays it");
        assert_eq!(tapped(&found), vec![2]);
    }

    #[test]
    fn a_land_with_a_choice_is_told_which_colour_to_make() {
        let sources = [any_land(1)];
        let found = plan(&cost("{U}"), &empty(), &sources).expect("it can make blue");
        assert_eq!(found.steps[0].color, Some(ManaColor::Blue));
        // A land with one colour is never asked — the engine does not ask.
        let found = plan(&cost("{G}"), &empty(), &[land(2, ManaColor::Green)]).expect("a forest");
        assert_eq!(found.steps[0].color, None);
    }

    #[test]
    fn floating_mana_is_spent_before_anything_is_tapped() {
        let pool = ManaPoolView {
            green: 1,
            ..empty()
        };
        let sources = [land(1, ManaColor::Green), land(2, ManaColor::Green)];
        let found = plan(&cost("{1}{G}"), &pool, &sources).expect("one land plus the float");
        assert_eq!(found.taps(), 1);
    }

    #[test]
    fn a_hybrid_takes_either_half() {
        let sources = [land(1, ManaColor::Red)];
        assert!(plan(&cost("{G/R}"), &empty(), &sources).is_some());
        let sources = [land(1, ManaColor::Green)];
        assert!(plan(&cost("{G/R}"), &empty(), &sources).is_some());
        let sources = [land(1, ManaColor::Blue)];
        assert!(plan(&cost("{G/R}"), &empty(), &sources).is_none());
    }

    /// `{2/W}` is one white *or* two of anything, and only the player knows
    /// which they meant — so both readings are tried.
    #[test]
    fn a_twobrid_is_paid_the_cheaper_way_it_can_be() {
        let white = [land(1, ManaColor::White)];
        let found = plan(&cost("{2/W}"), &empty(), &white).expect("one white");
        assert_eq!(found.taps(), 1);

        let islands = [land(1, ManaColor::Blue), land(2, ManaColor::Blue)];
        let found = plan(&cost("{2/W}"), &empty(), &islands).expect("two of anything");
        assert_eq!(found.taps(), 2);

        assert!(plan(&cost("{2/W}"), &empty(), &islands[..1]).is_none());
    }

    /// Phyrexian mana is read as its colour and never as two life. Life is a
    /// decision; a shortcut that spends it is a shortcut that loses games.
    #[test]
    fn phyrexian_mana_never_spends_life_behind_the_players_back() {
        let sources = [land(1, ManaColor::Blue)];
        assert!(plan(&cost("{U/P}"), &empty(), &sources).is_some());
        let sources = [land(1, ManaColor::Green)];
        assert!(plan(&cost("{U/P}"), &empty(), &sources).is_none());
    }

    /// `{X}` is a number the player picks and `{S}` is a property of the
    /// source rather than of the mana. Neither is this module's to guess.
    #[test]
    fn costs_this_side_of_the_wire_must_not_guess_at_are_refused() {
        let sources = [land(1, ManaColor::Green), land(2, ManaColor::Green)];
        assert!(plan(&cost("{X}{G}"), &empty(), &sources).is_none());
        assert!(plan(&cost("{S}"), &empty(), &sources).is_none());
    }

    /// A source that makes two of *one chosen* colour is worth one mana here.
    /// Counting it as two would build a plan asking it for `{W}` and `{U}`,
    /// which the engine would refuse halfway through — with the land already
    /// tapped.
    #[test]
    fn two_mana_of_one_colour_counts_once_and_a_fixed_pair_counts_twice() {
        let coupled = [Source {
            id: ObjectId::new(1, 0),
            tap: Tap::Ability(0),
            colors: vec![ManaColor::White, ManaColor::Blue],
            amount: 2,
        }];
        assert!(plan(&cost("{2}"), &empty(), &coupled).is_none());

        let sol_ring = [Source {
            id: ObjectId::new(1, 0),
            tap: Tap::Ability(0),
            colors: vec![ManaColor::Colorless],
            amount: 2,
        }];
        let found = plan(&cost("{2}"), &empty(), &sol_ring).expect("two colourless");
        assert_eq!(found.taps(), 1);
    }

    #[test]
    fn a_zero_cost_spell_taps_nothing() {
        let sources = [land(1, ManaColor::Green)];
        let found = plan(&ManaCost::ZERO, &empty(), &sources).expect("free");
        assert!(found.is_empty());
    }

    /// Colourless `{C}` is a colour like any other here, and generic is not
    /// payable by wishing: a five-colour land pays `{C}` only if it makes it.
    #[test]
    fn colourless_is_not_the_same_as_generic() {
        let five = [Source {
            id: ObjectId::new(1, 0),
            tap: Tap::Ability(0),
            colors: Color::ALL.iter().copied().map(mana_color).collect(),
            amount: 1,
        }];
        assert!(plan(&cost("{1}"), &empty(), &five).is_some());
        assert!(plan(&cost("{C}"), &empty(), &five).is_none());
    }

    #[test]
    fn a_land_taps_for_the_one_basic_type_it_projects() {
        use baylee_core::generated::subtypes::land;
        use baylee_core::types::SubtypeSet;

        let forest = SubtypeSet::from_slice(&[land::FOREST]);
        assert_eq!(basic_land_color(&forest), Some(ManaColor::Green));

        // A dual has two mana abilities and the engine offers neither as the
        // CR 305.6 shortcut, so this does not answer for it either.
        let shrine = SubtypeSet::from_slice(&[land::PLAINS, land::SWAMP]);
        assert_eq!(basic_land_color(&shrine), None);
        assert_eq!(basic_land_color(&SubtypeSet::EMPTY), None);
    }
}
