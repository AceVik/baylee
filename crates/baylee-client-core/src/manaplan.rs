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
    /// Whether `colors` is a **list of what one tap makes** rather than a
    /// list of what it may be asked for.
    ///
    /// `{T}: Add {W}{U}` and `{T}: Add {W} or {U}` carry the same two
    /// colours and are opposite offers, and nothing else on this struct can
    /// tell them apart. A Karoo makes both, so it pays two pips of different
    /// colours and the engine asks nothing; a dual land makes one, so it pays
    /// one pip and the engine asks which.
    ///
    /// Read it in the direction of failure. Left false on a bundle the plan
    /// merely under-counts, and the client is shy about a land that could
    /// have paid. Set true on a choice it over-counts, and the plan taps a
    /// board that then cannot pay — which is the half a player cannot undo,
    /// because the mana is already floating and the permanents are already
    /// tapped.
    pub bundle: bool,
    /// Whether tapping this costs anything **beyond** the tap: a land or a
    /// Treasure sacrificed, a life paid, a damage dealt beside the mana.
    ///
    /// The planner reaches for a priced source only where no clean one fits
    /// the pip, ahead of how many colours either makes — see [`assign`]. It
    /// is a field and not the caller's sort order because the order it
    /// decides is the matcher's, over every permanent at once; a caller can
    /// rank one permanent's modes, and did, but not which of two permanents
    /// pays a pip. Who sets it: `manasources::priced` for the client, and
    /// `policy::sources` for the house AI.
    pub priced: bool,
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
            bundle: false,
            priced: false,
        }
    }

    /// How many independent mana this is worth to a plan.
    ///
    /// Only a source with a single colour can be counted more than once:
    /// "add two mana of any one colour" is two mana that must match, and a
    /// matcher that treated them as independent would happily plan `{W}{U}`
    /// out of one of them.
    const fn units(&self) -> usize {
        if self.bundle {
            // Every colour listed arrives, so each one is its own unit — and
            // `amount` is not a multiplier here, it is 1 per entry.
            self.colors.len()
        } else if self.colors.len() == 1 {
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

/// What one tap of a permanent offers a **mana bubble**.
///
/// Deliberately not a [`Source`], and the missing field is still the point: a
/// bubble never counts mana, so how much a tap makes has no bearing on which
/// pip is drawn — and an ability whose amount is a count of the board ("add X
/// mana of any one color, where X is the number of Allies you control") has
/// no amount to give and still asks the player a colour.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Offer {
    /// Which action taps it.
    pub tap: Tap,
    /// The colours it offers a choice between.
    pub colors: Vec<ManaColor>,
    /// Whether what one press pours is a number at all.
    ///
    /// Not *which* number — that is the field this deliberately does not
    /// have, and every offer here pours something for the colour pressed.
    /// What a pip needs is only whether the tap behind it is predictable,
    /// and it needs that for one reason: a permanent with two mana taps of
    /// the same colours reduces to one pip per colour, so the pip has to
    /// stand for the tap a player can foresee and the other has to be given
    /// its sentence back rather than deleted. Harabaz Druid under a Great
    /// Divide Guide is that permanent — the grant makes one mana of any
    /// colour, its own ability makes X, and X is a count of the battlefield
    /// nothing this side of the engine reads.
    pub fixed: bool,
}

/// One pip of a mana bubble: a colour, and the tap that pours it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Pour {
    /// The colour the pip is drawn in.
    pub color: ManaColor,
    /// The tap that makes it, and the answer to the engine's question —
    /// [`Step::color`] is `None` where that tap makes this colour and no
    /// other, because then the engine does not ask and an answer to a
    /// question nobody posed aborts the run.
    pub step: Step,
}

/// The pips a permanent's taps come to: one per colour, in `ManaColor` order.
///
/// **A permanent taps once**, which is why this is a colour-wise union and
/// not a list of abilities. A Forest under a Chromatic Lantern makes `{G}`
/// twice over — from its own CR 305.6 shortcut and from the grant — and that
/// is one pip, not two.
///
/// Which of the taps wins a colour is the only judgement here, and it is made
/// in three parts, each of which is the same question asked one step further
/// down.
///
/// **A pip a player can foresee**, first. A pip is a promise of that colour
/// and says nothing about how much, so the tap behind it had better be one
/// whose pour is a number — Harabaz Druid under a Great Divide Guide offers
/// all five colours twice over, once as "one mana" and once as "X mana, where
/// X is the number of Allies you control", and a header built out of the
/// second is a header nobody can read. Where an unpredictable tap is the
/// **only** cover for a colour it takes that pip all the same, which is not
/// an exception: a Harabaz Druid on its own is a bubble of five, and that is
/// what the owner asked for.
///
/// Then the tap that will **not stop to ask**: a source of one colour is one
/// action, where a source of five is an action and then an answer. Where that
/// ties, the intrinsic shortcut wins, for the reason [`plan`] already prefers
/// it — one fewer round trip.
///
/// What this does *not* decide is what becomes of the taps that win nothing.
/// They are not interchangeable with the winner — that is the whole of the
/// first part above — so `abilities::pour_out` keeps each of them as a
/// written row rather than folding it into the header.
#[must_use]
pub fn pours(id: ObjectId, offers: &[Offer]) -> Vec<Pour> {
    /// How much a tap makes a player guess and then wait, lower being better.
    fn patience(offer: &Offer) -> (bool, usize, bool) {
        (
            !offer.fixed,
            offer.colors.len(),
            matches!(offer.tap, Tap::Ability(_)),
        )
    }
    ManaColor::ALL
        .into_iter()
        .filter_map(|color| {
            let best = offers
                .iter()
                .filter(|offer| offer.colors.contains(&color))
                .min_by_key(|offer| patience(offer))?;
            Some(Pour {
                color,
                step: Step {
                    source: id,
                    tap: best.tap,
                    color: (best.colors.len() > 1).then_some(color),
                },
            })
        })
        .collect()
}

/// The taps that make a spell castable.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Plan {
    /// In the order they should be sent.
    pub steps: Vec<Step>,
    /// What the taps are for.
    ///
    /// Carried rather than re-derived, because the cost [`plan`] was asked
    /// about is not always the one printed on the card: a commander's is the
    /// printed cost plus its tax, and a prompt that went back to the face to
    /// name a price would quote a player two mana under what they are about
    /// to pay. [`plan`] is the only thing that sets it.
    pub cost: ManaCost,
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
            return Some(Plan {
                cost: *cost,
                ..found
            });
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
        if source.bundle {
            // One unit per colour, each holding only *its own* colour. The
            // shared mask above is what makes a choice a choice; a bundle has
            // no choice in it, and a unit that claimed the whole mask could
            // be matched against a pip its colour cannot pay.
            for color in &source.colors {
                units.push(Unit {
                    colors: ColorMask::of(*color),
                    from: Some(index),
                });
            }
        } else {
            for _ in 0..source.units() {
                units.push(Unit {
                    colors,
                    from: Some(index),
                });
            }
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
/// mana before any tap, then a **clean** tap before a priced one, then the
/// *least* flexible source that fits — so the land that only makes green pays
/// the green pip and the one that makes anything is still untapped
/// afterwards, and Ancient Tomb's two damage are taken only where no free
/// land fits.
///
/// Price comes before breadth, and the case that decides it is Ancient Tomb
/// beside a Command Tower paying `{1}`: by breadth alone the Tomb goes first
/// (one colour against five) and the player takes two damage with a free
/// land untapped. Payability does not move — Kuhn finds a matching whenever
/// one exists, whatever the order — only which taps it picks.
///
/// An order is per pip, though, and a plan is per permanent, so the matching
/// is followed by [`consolidate`]: price first would otherwise tap the Tower
/// *and* the Tomb for `{2}`, where the Tomb alone pays it and costs the same
/// two damage.
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
            unit.from.is_some_and(|source| sources[source].priced),
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

    consolidate(needs, &units, &mut taken, sources);
    Some(steps(needs, &units, &taken, sources))
}

/// Gives back every tap whose mana the rest of the plan already makes.
///
/// A permanent that makes two mana is tapped whole, so once it is in the plan
/// its second mana is free — and the matching cannot know that, because its
/// order is fixed before anything is tapped. Two places that shows: price
/// first taps a Command Tower and then an Ancient Tomb for `{2}`, where the
/// Tomb alone pays it for the same two damage; and a Forest listed before a
/// Sol Ring pays half of `{2}` beside it.
///
/// So each round drops one tapped source whose pips all fit on spare mana
/// the plan already has — floating, or the unused half of a permanent it
/// taps anyway — and stops when none does. A priced source is tried first,
/// then the roomiest clean one, which is the one most worth keeping untapped.
/// Every round removes a source, so it ends; and every move is onto a unit
/// whose colours cover the pip, so the plan stays a plan.
fn consolidate(
    needs: &[ColorMask],
    units: &[Unit],
    taken: &mut [Option<usize>],
    sources: &[Source],
) {
    loop {
        let tapped: BTreeSet<usize> = units
            .iter()
            .zip(taken.iter())
            .filter_map(|(unit, holder)| holder.and(unit.from))
            .collect();
        let mut candidates: Vec<usize> = tapped.iter().copied().collect();
        candidates.sort_by_key(|&source| {
            (
                !sources[source].priced,
                std::cmp::Reverse(mask_of(&sources[source].colors).count()),
                source,
            )
        });
        let Some(moves) = candidates
            .into_iter()
            .find_map(|candidate| rehome(candidate, needs, units, taken, &tapped))
        else {
            return;
        };
        for (from, to, demand) in moves {
            taken[from] = None;
            taken[to] = Some(demand);
        }
    }
}

/// Where each pip `candidate` pays could go instead, or `None` if one of
/// them has nowhere: `(unit it leaves, unit it takes, the pip)`.
///
/// Greedy, most-constrained pip first, over what is spare. A pip it fails to
/// place only means the plan keeps that tap, which is where it started.
fn rehome(
    candidate: usize,
    needs: &[ColorMask],
    units: &[Unit],
    taken: &[Option<usize>],
    tapped: &BTreeSet<usize>,
) -> Option<Vec<(usize, usize, usize)>> {
    let mut held: Vec<(usize, usize)> = units
        .iter()
        .zip(taken)
        .enumerate()
        .filter_map(|(unit, (u, holder))| (u.from == Some(candidate)).then_some((unit, (*holder)?)))
        .collect();
    held.sort_by_key(|&(_, demand)| needs[demand].count());
    let mut spare: Vec<usize> = units
        .iter()
        .enumerate()
        .filter(|(unit, u)| {
            taken[*unit].is_none()
                && u.from
                    .is_none_or(|source| source != candidate && tapped.contains(&source))
        })
        .map(|(unit, _)| unit)
        .collect();
    let mut moves = Vec::with_capacity(held.len());
    for (from, demand) in held {
        let at = spare
            .iter()
            .position(|&unit| needs[demand].overlaps(units[unit].colors))?;
        moves.push((from, spare.remove(at), demand));
    }
    Some(moves)
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
        // `plan` overwrites this with the cost the taps were found for;
        // the matching itself has no idea what it was asked to pay.
        cost: ManaCost::ZERO,
        steps: used
            .into_iter()
            .map(|index| {
                let source = &sources[index];
                Step {
                    source: source.id,
                    tap: source.tap,
                    // Only a source with a real choice is ever asked. A
                    // bundle has several colours and no choice among them,
                    // so answering one would be answering a question the
                    // engine never asks.
                    color: (!source.bundle && source.colors.len() > 1)
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
            bundle: false,
            priced: false,
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

    /// A plan remembers the price it was found for.
    ///
    /// The prompt bar quotes it — "Pay {2}{G} and cast", drawn as mana pips
    /// — and it must be the cost `plan` was *asked* about rather than the one
    /// printed on the card: a commander's is the printed cost plus two mana
    /// per previous cast, and a bar that went back to the face would quote a
    /// player a price they are not being charged. The bar used to say "Tap
    /// 3", which named the client's own lands instead of the spell's cost.
    #[test]
    fn a_plan_carries_the_cost_it_was_found_for() {
        let sources = [
            land(1, ManaColor::Green),
            land(2, ManaColor::Green),
            land(3, ManaColor::Green),
        ];
        let asked = cost("{2}{G}");
        let found = plan(&asked, &empty(), &sources).expect("three lands is enough");
        assert_eq!(found.cost, asked);
        assert_eq!(found.cost.to_string(), "{2}{G}");

        // And a plan that taps nothing carries it too, because the price is
        // what the row says and floating mana is still spent.
        let pool = ManaPoolView {
            green: 3,
            ..empty()
        };
        let free = plan(&asked, &pool, &[]).expect("already payable");
        assert!(free.is_empty());
        assert_eq!(free.cost, asked);
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
            bundle: false,
            priced: false,
        }];
        assert!(plan(&cost("{2}"), &empty(), &coupled).is_none());

        let sol_ring = [Source {
            id: ObjectId::new(1, 0),
            tap: Tap::Ability(0),
            colors: vec![ManaColor::Colorless],
            amount: 2,
            bundle: false,
            priced: false,
        }];
        let found = plan(&cost("{2}"), &empty(), &sol_ring).expect("two colourless");
        assert_eq!(found.taps(), 1);
    }

    /// A Karoo adds both its colours and is asked nothing; a dual land
    /// carrying the same two is asked which.
    ///
    /// The pair is the whole test, because a single assertion here passes for
    /// the wrong reason. `{W}{U}` is payable only if two units arrive, one of
    /// each colour — and `{W}{W}` must stay unpayable, because a bundle is not
    /// a choice and no amount of needing white makes the blue one white. Drop
    /// `bundle` and the two go wrong in *opposite* directions: the first
    /// becomes unpayable (one unit for a two-pip cost), and the same colours
    /// read as a choice with `amount: 2` would make the second payable, which
    /// taps the land and then cannot pay.
    #[test]
    fn a_bundle_pays_one_pip_of_each_colour_and_never_two_of_one() {
        let chancery = [Source {
            id: ObjectId::new(1, 0),
            tap: Tap::Ability(0),
            colors: vec![ManaColor::White, ManaColor::Blue],
            amount: 2,
            bundle: true,
            priced: false,
        }];
        let found = plan(&cost("{W}{U}"), &empty(), &chancery).expect("a Karoo pays {W}{U}");
        assert_eq!(found.taps(), 1, "one permanent is one tap");
        assert_eq!(
            found.steps[0].color, None,
            "a bundle offers no choice, so the engine asks for none"
        );
        assert!(
            plan(&cost("{W}{W}"), &empty(), &chancery).is_none(),
            "it makes one of each, not two of either"
        );
    }

    /// Every colour but colourless, which is what "any color" makes.
    fn five() -> Vec<ManaColor> {
        Color::ALL.iter().copied().map(mana_color).collect()
    }

    /// A Command Tower in a five-colour deck: any colour, for the tap.
    fn tower(id: u32) -> Source {
        Source {
            id: ObjectId::new(id, 0),
            tap: Tap::Ability(0),
            colors: five(),
            amount: 1,
            bundle: false,
            priced: false,
        }
    }

    /// A Treasure: any colour, and the token goes with it.
    fn treasure(id: u32) -> Source {
        Source {
            priced: true,
            ..tower(id)
        }
    }

    /// Ancient Tomb: `{C}{C}`, and two damage to its controller.
    fn tomb(id: u32) -> Source {
        Source {
            colors: vec![ManaColor::Colorless],
            amount: 2,
            priced: true,
            ..tower(id)
        }
    }

    fn sol_ring(id: u32) -> Source {
        Source {
            colors: vec![ManaColor::Colorless],
            amount: 2,
            ..tower(id)
        }
    }

    /// #165 promised that a tap with a price is reached only where nothing
    /// else can pay, and kept it only between the modes of one permanent.
    /// Across permanents the order was breadth alone, so Ancient Tomb — one
    /// colour against the Tower's five — paid `{1}` and dealt two damage with
    /// a free land standing untapped beside it.
    #[test]
    fn a_priced_tap_waits_behind_every_clean_one_that_fits() {
        let found = plan(&cost("{1}"), &empty(), &[tomb(1), tower(2)]).expect("either pays {1}");
        assert_eq!(tapped(&found), [2], "two damage, with the Tower untapped");

        let sources = [tomb(1), tower(2), land(3, ManaColor::Green)];
        let found = plan(&cost("{2}"), &empty(), &sources).expect("Tower and Forest pay {2}");
        assert_eq!(
            tapped(&found),
            [2, 3],
            "the Tomb paid what two free lands could"
        );
    }

    /// The case that asked for it (#210). A Treasure makes any colour, so it
    /// ties a Command Tower on breadth, and a tie fell to whichever was listed
    /// first — here, on purpose, the Treasure.
    ///
    /// The second half is the counter-proof that price orders and never
    /// refuses: where the Tower alone cannot pay, the Treasure is spent.
    #[test]
    fn a_treasure_is_kept_while_a_free_land_makes_the_colour() {
        let sources = [treasure(1), tower(2)];
        let found = plan(&cost("{G}"), &empty(), &sources).expect("either makes green");
        assert_eq!(
            tapped(&found),
            [2],
            "sacrificed a Treasure for a free land's mana"
        );

        let found = plan(&cost("{G}{G}"), &empty(), &sources).expect("both make green");
        assert_eq!(tapped(&found), [1, 2]);
    }

    /// A permanent that makes two mana is tapped whole, so once it is in a
    /// plan its second mana costs nothing. Price first on its own would tap
    /// the Tower for one pip and then the Tomb for the other, taking the same
    /// two damage and spending the Tower as well; a Forest listed before a
    /// Sol Ring did the same without any price in it at all.
    ///
    /// The last case is the counter-proof: a tap the plan needs stays in it.
    #[test]
    fn a_plan_taps_no_permanent_whose_mana_another_tap_already_makes() {
        let found = plan(&cost("{2}"), &empty(), &[tomb(1), tower(2)]).expect("the Tomb pays {2}");
        assert_eq!(
            tapped(&found),
            [1],
            "tapped the Tower for mana the Tomb made anyway"
        );

        let sources = [land(1, ManaColor::Green), sol_ring(2)];
        let found = plan(&cost("{2}"), &empty(), &sources).expect("Sol Ring pays {2}");
        assert_eq!(tapped(&found), [2], "tapped the Forest beside a Sol Ring");

        let found = plan(&cost("{2}{G}"), &empty(), &sources).expect("both pay {2}{G}");
        assert_eq!(tapped(&found), [1, 2], "dropped a tap the plan needed");
    }

    /// The steps come back in the order the sources were given, whatever the
    /// matcher preferred and whatever `consolidate` gave back.
    ///
    /// A contract rather than an accident of `steps` walking a set of
    /// indices: the house AI puts a restricted source last and relies on it
    /// being the last tap (#223), because mana that may only be spent on some
    /// spells, once floating, is counted by no plan. The ids run against the
    /// slice here so a sorted answer cannot pass for an ordered one.
    #[test]
    fn the_steps_come_back_in_the_order_the_sources_were_given() {
        let order =
            |found: &Plan| -> Vec<u32> { found.steps.iter().map(|s| s.source.slot()).collect() };

        // The Tower pays white, the Forest green and the Sol Ring both
        // generics; the Treasure, listed last, is never reached.
        let sources = [
            tower(9),
            sol_ring(5),
            land(3, ManaColor::Green),
            treasure(1),
        ];
        let found = plan(&cost("{2}{W}{G}"), &empty(), &sources).expect("payable");
        assert_eq!(order(&found), [9, 5, 3]);

        // The Forest is matched for a generic and then given back to the
        // Sol Ring's second mana; what is left keeps the slice's order.
        let sources = [tower(9), land(7, ManaColor::Green), sol_ring(5)];
        let found = plan(&cost("{2}{W}"), &empty(), &sources).expect("payable");
        assert_eq!(order(&found), [9, 5]);
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
            bundle: false,
            priced: false,
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

    /// A Forest under a Chromatic Lantern: five pips, and the green one is
    /// the land's **own** tap.
    ///
    /// The whole judgement in [`pours`], as arithmetic. Both taps make green;
    /// the grant would stop and ask which colour and the CR 305.6 shortcut
    /// would not, so green is poured by the shortcut and the other four by
    /// the grant. Backwards, it costs a round trip and a colour prompt for
    /// the one colour the land prints on itself.
    #[test]
    fn a_granted_any_colour_fills_in_round_the_lands_own_mana() {
        let id = ObjectId::new(4, 0);
        let offers = vec![
            Offer {
                tap: Tap::Ability(u32::MAX),
                colors: vec![
                    ManaColor::White,
                    ManaColor::Blue,
                    ManaColor::Black,
                    ManaColor::Red,
                    ManaColor::Green,
                ],
                fixed: true,
            },
            Offer {
                tap: Tap::Intrinsic,
                colors: vec![ManaColor::Green],
                fixed: true,
            },
        ];
        let poured = pours(id, &offers);
        assert_eq!(
            poured.iter().map(|p| p.color).collect::<Vec<_>>(),
            vec![
                ManaColor::White,
                ManaColor::Blue,
                ManaColor::Black,
                ManaColor::Red,
                ManaColor::Green
            ],
            "one pip per colour, in WUBRG order"
        );
        let green = poured.last().expect("five pips");
        assert_eq!(green.step.tap, Tap::Intrinsic, "green is the land's own");
        assert_eq!(
            green.step.color, None,
            "and a one-colour tap is never asked which colour"
        );
        let white = poured.first().expect("five pips");
        assert_eq!(white.step.tap, Tap::Ability(u32::MAX), "white is the grant");
        assert_eq!(
            white.step.color,
            Some(ManaColor::White),
            "which does ask, and the answer is already in hand"
        );
        assert!(poured.iter().all(|p| p.step.source == id));
    }

    /// Nothing to choose is not a bubble, and [`pours`] says so by answering
    /// with one pip: a Plains stays on the one-tap path it has always been on.
    #[test]
    fn a_source_of_one_colour_comes_to_one_pip() {
        let id = ObjectId::new(5, 0);
        let offers = vec![Offer {
            tap: Tap::Intrinsic,
            colors: vec![ManaColor::White],
            fixed: true,
        }];
        let poured = pours(id, &offers);
        assert_eq!(poured.len(), 1);
        assert_eq!(poured[0].color, ManaColor::White);
        assert_eq!(poured[0].step.color, None);
        assert!(pours(id, &[]).is_empty());
    }

    /// Every colour, and no number behind any of them.
    ///
    /// Harabaz Druid on its own: nothing else covers a single one of the five
    /// colours, so the unpredictable tap takes every pip and the creature is
    /// a bubble. The owner asked for that in as many words — *"Artefakte und
    /// Kreaturen, die nur Mana Ability haben"* — and it is the half of the
    /// rule that a plain "refuse what cannot be counted" would have lost.
    #[test]
    fn a_tap_no_one_can_count_still_pours_where_nothing_else_does() {
        let id = ObjectId::new(6, 0);
        let offers = vec![Offer {
            tap: Tap::Ability(0),
            colors: ManaColor::ALL
                .into_iter()
                .filter(|c| *c != ManaColor::Colorless)
                .collect(),
            fixed: false,
        }];
        let poured = pours(id, &offers);
        assert_eq!(poured.len(), 5, "one pip per colour it offers");
        assert!(poured.iter().all(|p| p.step.tap == Tap::Ability(0)));
    }

    /// The same tap beside one that pours a number, and it loses every
    /// colour.
    ///
    /// Harabaz Druid under a Great Divide Guide, which is what the owner
    /// reported: two taps, the same five colours, and one pip per colour to
    /// share between them. The pips go to the grant — one mana of whichever
    /// colour is pressed, which is what a pip claims — and the Druid's own
    /// ability wins nothing here precisely so that the sheet can give it its
    /// sentence back instead of folding it away.
    #[test]
    fn a_countable_tap_takes_the_pips_from_one_that_is_not() {
        let id = ObjectId::new(7, 0);
        let every: Vec<ManaColor> = ManaColor::ALL
            .into_iter()
            .filter(|c| *c != ManaColor::Colorless)
            .collect();
        let offers = vec![
            Offer {
                tap: Tap::Ability(0),
                colors: every.clone(),
                fixed: false,
            },
            Offer {
                tap: Tap::Ability(u32::MAX),
                colors: every,
                fixed: true,
            },
        ];
        let poured = pours(id, &offers);
        assert_eq!(poured.len(), 5);
        assert!(
            poured.iter().all(|p| p.step.tap == Tap::Ability(u32::MAX)),
            "every pip is the tap whose pour is a number: {poured:?}"
        );
        assert!(
            poured.iter().all(|p| p.step.color.is_some()),
            "a five-colour tap is asked which colour, and the answer is in hand"
        );
    }
}
