//! One planner for a payment the engine names, restricted mana included
//! (CR 106.6): the client's reach and payment window, the language-model
//! seat and the trained AI's dev menu plan through it. The frozen house does not: it
//! keeps its own reading (`policy::spend_on`, `restricted`).
//!
//! What a restricted mana ability's mana, or restricted mana already
//! floating, may pay for is the engine's statement in the offer
//! (`LegalActions::restricted_taps`, `restricted_floating`), asked of the
//! payment as the engine will make it. Read off the view instead it was
//! three answers: the client tapped at most one restricted source and the
//! seat any number, both judged a face of a card by its front, and neither
//! could count a Cavern of Souls' green once it floated, so two Ancient
//! Ziggurats never lit Grizzly Bears and an Elf whose green was already
//! tapped went dark. With the statement, every restricted tap that may pay
//! is a source like any other, and what floats is mana like any other.
//!
//! What a source *makes* stays each caller's reading (the client reads the
//! registry and the view's projections, the seat and the menu this crate's
//! `policy::sources`); what its mana may *pay for* is the engine's alone.

use baylee_client_core::manaplan::{self, Dear, Plan, Source, Tap};
use baylee_core::ids::ObjectId;
use baylee_core::mana::{ManaColor, ManaCost};
use baylee_engine::choice::{LegalActions, Spend};
use baylee_view::{ManaPoolView, PlayerView};

/// The mana a seat could pay with, as its caller reads it: the pool, the
/// unrestricted sources, and the restricted ones (each with what it makes,
/// and nothing of what it may pay for).
#[derive(Clone, Copy, Debug)]
pub struct Mana<'a> {
    /// What floats.
    pub pool: &'a ManaPoolView,
    /// The unrestricted taps, one per permanent.
    pub plain: &'a [Source],
    /// The restricted taps on offer.
    pub restricted: &'a [Source],
}

/// `pool` with the restricted mana floating that the engine says `spend`
/// may use counted as ordinary mana of its colour.
#[must_use]
pub fn pool_for(pool: &ManaPoolView, legal: &LegalActions, spend: Spend) -> ManaPoolView {
    let floating = legal.restricted_floating_for(spend);
    let mut out = *pool;
    for color in ManaColor::ALL {
        let slot = match color {
            ManaColor::White => &mut out.white,
            ManaColor::Blue => &mut out.blue,
            ManaColor::Black => &mut out.black,
            ManaColor::Red => &mut out.red,
            ManaColor::Green => &mut out.green,
            ManaColor::Colorless => &mut out.colorless,
        };
        *slot = slot.saturating_add(floating[color.index()]);
    }
    out
}

/// The unrestricted sources with every restricted tap the engine names for
/// `spend` beside them, last: in place of its permanent's unrestricted tap,
/// which stays as its other mode when both make one mana (Cavern of Souls'
/// `{C}` beside its coloured mana for an Elf), so a permanent is still
/// tapped once. `except` is a permanent no restricted tap is taken from (an
/// activation's own source, kept for its cost).
#[must_use]
pub fn sources_for(
    legal: &LegalActions,
    spend: Spend,
    mana: Mana<'_>,
    except: Option<ObjectId>,
) -> Vec<Source> {
    let mut named = legal.restricted_taps_for(spend);
    named.retain(|(source, _)| Some(*source) != except);
    let mut restricted: Vec<Source> = mana
        .restricted
        .iter()
        .filter(|source| {
            matches!(source.tap, Tap::Ability(index) if named.contains(&(source.id, index)))
        })
        .cloned()
        .collect();
    // One per permanent: the one that makes the most, then the most colours.
    restricted.sort_by_key(|s| {
        (
            s.id,
            std::cmp::Reverse(s.amount),
            std::cmp::Reverse(s.colors.len()),
        )
    });
    restricted.dedup_by_key(|s| s.id);
    let mut out: Vec<Source> = Vec::with_capacity(mana.plain.len() + restricted.len());
    let mut last: Vec<Source> = Vec::with_capacity(restricted.len());
    for source in mana.plain {
        let Some(at) = restricted.iter().position(|r| r.id == source.id) else {
            out.push(source.clone());
            continue;
        };
        let mut taken = restricted.remove(at);
        // A permanent whose plain tap already has another mode, or makes
        // more, keeps it: it is tapped once, one way.
        if source.converter.is_some() || source.dear.is_some() || taken.amount < source.amount {
            out.push(source.clone());
            continue;
        }
        if taken.amount == 1
            && source.amount == 1
            && source.colors.iter().any(|c| !taken.colors.contains(c))
        {
            taken.dear = Some(Dear {
                tap: source.tap,
                colors: source.colors.clone(),
            });
        }
        last.push(taken);
    }
    out.extend(last);
    out.extend(restricted);
    out
}

/// The taps that pay `cost` for `spend` out of `mana`, with the restricted
/// mana the engine names for it: unrestricted mana alone first, so a plan
/// that needs no restricted tap takes none. With `life`, a Phyrexian symbol
/// is paid with 2 life only where no mana pays it
/// (`manaplan::plan_with_life`). `except` is as [`sources_for`] reads it.
///
/// The restricted mana is earmarked for `cost` (`manaplan::Earmarked`): it
/// pays the cost's symbols and never a paid mana ability's `{1}` on the
/// way, and restricted mana the engine does not name for `spend` is left
/// alone, so a filter land or Study Hall is still planned while some floats
/// (Carbon b9: Great Hall of the Biblioplex's mana floating for instants
/// kept Study Hall's black out of every plan).
#[must_use]
pub fn plan_for(
    spend: Spend,
    cost: &ManaCost,
    mana: Mana<'_>,
    legal: &LegalActions,
    life: Option<i32>,
    except: Option<ObjectId>,
) -> Option<Plan> {
    let mut named = legal.restricted_taps_for(spend);
    named.retain(|(source, _)| Some(*source) != except);
    let taps: Vec<(ObjectId, Tap)> = named
        .iter()
        .map(|&(source, index)| (source, Tap::Ability(index)))
        .collect();
    let earmarked = manaplan::Earmarked {
        floating: legal.restricted_floating_for(spend),
        taps: &taps,
    };
    let plan = |sources: &[Source], with_life: bool| match life {
        Some(life) if with_life => {
            manaplan::plan_with_life_earmarked(cost, mana.pool, earmarked, sources, life)
        }
        _ => manaplan::plan_earmarked(cost, mana.pool, earmarked, sources),
    };
    let all = (!named.is_empty()).then(|| sources_for(legal, spend, mana, except));
    plan(mana.plain, false)
        .or_else(|| all.as_deref().and_then(|all| plan(all, false)))
        .or_else(|| plan(mana.plain, true))
        .or_else(|| all.as_deref().and_then(|all| plan(all, true)))
}

/// The pool and the sources a payment for `spend` is planned from, for a
/// caller that plans with its own matcher (the client's payment window
/// stages taps over them): [`pool_for`] and [`sources_for`].
#[must_use]
pub fn inputs_for(
    legal: &LegalActions,
    spend: Spend,
    mana: Mana<'_>,
) -> (ManaPoolView, Vec<Source>) {
    (
        pool_for(mana.pool, legal, spend),
        sources_for(legal, spend, mana, None),
    )
}

/// The restricted taps on offer as this crate reads what they make (its
/// `policy::offers`), for a caller with no reader of its own.
#[must_use]
pub fn restricted_sources(view: &PlayerView, legal: &LegalActions) -> Vec<Source> {
    crate::policy::restricted_sources(view, legal)
}

/// Whether this seat has any mana to plan with beyond what floats: an
/// unrestricted source, a restricted one the engine names for anything, or
/// restricted mana floating that pays for something.
#[must_use]
pub fn has_mana_to_plan(view: &PlayerView, legal: &LegalActions) -> bool {
    !legal.restricted_taps.is_empty()
        || !legal.restricted_floating.is_empty()
        || !crate::policy::sources(view, legal).is_empty()
}

/// A permanent a payment leaves untapped where it can: an ability of its
/// own that taps it is on offer and waits only for mana
/// (`LegalActions::unpaid_abilities`), at `cost`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Keep {
    /// The permanent.
    pub source: ObjectId,
    /// What its cheapest such ability costs in mana, `{X}` at 0.
    pub cost: ManaCost,
}

/// Every permanent but `except` with an ability on offer that taps it and
/// waits only for mana, cheapest first: the second Rishadan Port while the
/// first is paid for. A mana ability is no such claim (it is a source).
#[must_use]
pub fn keepers(view: &PlayerView, legal: &LegalActions, except: Option<ObjectId>) -> Vec<Keep> {
    let mut out: Vec<Keep> = Vec::new();
    for &(source, index, cost) in &legal.unpaid_abilities {
        if Some(source) == except
            || crate::is_mana_ability(view, source, index)
            || !crate::ability_taps_its_source(view, source, index)
        {
            continue;
        }
        let cost = cost.with_x(0);
        match out.iter_mut().find(|kept| kept.source == source) {
            Some(kept) if cost.cmc() < kept.cost.cmc() => kept.cost = cost,
            Some(_) => {}
            None => out.push(Keep { source, cost }),
        }
    }
    out.sort_by_key(|kept| (kept.cost.cmc(), kept.source));
    out
}

/// [`plan_for`], leaving each of `keep` untapped where the rest of the mana
/// pays `cost` and, after it, every kept permanent's own ability too.
///
/// Two Rishadan Ports beside five lands paid each other's `{1}`: a Port is
/// worth least to a hand that wants coloured mana (`manaplan::prioritize`
/// values an ability that costs mana only as a tie-break, report 5.11.2026),
/// so the seat offered each Port's activation tapping the other, and both
/// could never be used (Carbon b4, 11.11.2026). A permanent is kept only
/// while what is left pays its ability, from untapped sources alone; the
/// dearest is given up first, and with none kept the plan is [`plan_for`]'s,
/// so nothing that was payable stops being so.
#[must_use]
pub fn plan_keeping(
    spend: Spend,
    cost: &ManaCost,
    mana: Mana<'_>,
    legal: &LegalActions,
    life: Option<i32>,
    except: Option<ObjectId>,
    keep: &[Keep],
) -> Option<Plan> {
    let mut kept: Vec<Keep> = keep
        .iter()
        .filter(|kept| mana.plain.iter().any(|source| source.id == kept.source))
        .copied()
        .collect();
    while !kept.is_empty() {
        let held = |source: &&Source| !kept.iter().any(|k| k.source == source.id);
        let plain: Vec<Source> = mana.plain.iter().filter(held).cloned().collect();
        let restricted: Vec<Source> = mana.restricted.iter().filter(held).cloned().collect();
        let fewer = Mana {
            pool: mana.pool,
            plain: &plain,
            restricted: &restricted,
        };
        if let Some(found) = plan_for(spend, cost, fewer, legal, life, except) {
            let left: Vec<Source> = plain
                .iter()
                .filter(|source| !found.steps.iter().any(|step| step.source == source.id))
                .cloned()
                .collect();
            let owed = kept
                .iter()
                .fold(ManaCost::ZERO, |sum, kept| sum.combine(&kept.cost));
            if manaplan::plan(&owed, &ManaPoolView::default(), &left).is_some() {
                return Some(found);
            }
        }
        kept.pop();
    }
    plan_for(spend, cost, mana, legal, life, except)
}

/// The most activations of one counter-paid mana ability a plan reads: past
/// it a plan under-counts (`manaplan`'s third rule), and the matching stays
/// small.
const MOST_REPEATS: u16 = 8;

/// `sources` with a counter-paid mana ability that does not tap its
/// permanent read once for every activation its counters pay: Druids'
/// Repository with two charge counters ("Remove a charge counter from this
/// enchantment: Add one mana of any color") is two mana, each its own
/// activation and colour, where `manaplan` counts one source a permanent
/// (Carbon c1, 11.11.2026). Each further activation is a source of its own
/// beside the first, so a plan's steps name the permanent once per
/// activation. Only for the planners of [`plan_for`]; the frozen house keeps
/// its own reading.
pub fn repeat_counter_sources(view: &PlayerView, sources: &mut Vec<Source>) {
    use baylee_cards_dsl::{AbilityDef, CostPart};
    let mut more: Vec<Source> = Vec::new();
    for source in sources.iter() {
        let Tap::Ability(index) = source.tap else {
            continue;
        };
        if source.colors.is_empty() || source.converter.is_some() {
            continue;
        }
        let Some(
            AbilityDef::Activated {
                cost,
                mana_ability: true,
                ..
            }
            | AbilityDef::ActivatedConditional {
                cost,
                mana_ability: true,
                ..
            },
        ) = crate::activate::printed(view, source.id, index)
        else {
            continue;
        };
        let [CostPart::RemoveCounterSelf { kind, n }] = cost.parts else {
            continue;
        };
        if !cost.mana.is_empty() || *n == 0 {
            continue;
        }
        let Some(object) = view.object(source.id) else {
            continue;
        };
        let times = object.counter_count(crate::worth::counter_view(*kind)) / n;
        for _ in 1..times.min(MOST_REPEATS) {
            more.push(Source {
                dear: None,
                ..source.clone()
            });
        }
    }
    sources.extend(more);
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_engine::choice::{CastModeKind, RestrictedTaps};

    fn o(n: u32) -> ObjectId {
        ObjectId::new(n, 0)
    }

    fn source(id: u32, index: u32, colors: &[ManaColor], amount: u8) -> Source {
        Source {
            id: o(id),
            tap: Tap::Ability(index),
            colors: colors.to_vec(),
            amount,
            bundle: false,
            priced: false,
            preserve: 0,
            snow: false,
            dear: None,
            converter: None,
        }
    }

    const ANY: [ManaColor; 5] = [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ];

    /// Two Ziggurats the engine names for the Bears pay its `{1}{G}`; named
    /// for nothing, they pay nothing.
    #[test]
    fn every_named_restricted_tap_is_a_source() {
        let bears = Spend::Cast(o(50), CastModeKind::Normal);
        let restricted = [source(1, 0, &ANY, 1), source(2, 0, &ANY, 1)];
        let pool = ManaPoolView::default();
        let mana = Mana {
            pool: &pool,
            plain: &[],
            restricted: &restricted,
        };
        let cost = ManaCost::parse("{1}{G}");
        let mut legal = LegalActions::default();
        assert!(plan_for(bears, &cost, mana, &legal, None, None).is_none());
        legal.restricted_taps = vec![RestrictedTaps {
            taps: vec![(o(1), 0), (o(2), 0)],
            spends: vec![bears],
        }];
        let plan = plan_for(bears, &cost, mana, &legal, None, None).expect("both Ziggurats");
        assert_eq!(plan.taps(), 2);
        let bolt = Spend::Cast(o(51), CastModeKind::Normal);
        assert!(plan_for(bolt, &ManaCost::parse("{R}"), mana, &legal, None, None).is_none());
    }

    /// A Cavern's green floating counts for the Elf the engine names it for,
    /// and for nothing else.
    #[test]
    fn named_floating_mana_is_mana() {
        let elf = Spend::Cast(o(50), CastModeKind::Normal);
        let bear = Spend::Cast(o(51), CastModeKind::Normal);
        let mut pool = ManaPoolView::default();
        pool.restricted[ManaColor::Green.index()] = 1;
        let forest = [source(3, 0, &[ManaColor::Green], 1)];
        let mana = Mana {
            pool: &pool,
            plain: &forest,
            restricted: &[],
        };
        let mut green = [0; 6];
        green[ManaColor::Green.index()] = 1;
        let legal = LegalActions {
            restricted_floating: vec![(elf, green)],
            ..LegalActions::default()
        };
        let cost = ManaCost::parse("{G}{G}");
        assert_eq!(
            plan_for(elf, &cost, mana, &legal, None, None).map(|p| p.taps()),
            Some(1)
        );
        assert!(plan_for(bear, &cost, mana, &legal, None, None).is_none());
    }

    /// A Cavern is tapped once: its restricted colour for the Elf, or its
    /// `{C}` where only that pays, never both.
    #[test]
    fn a_permanent_with_both_taps_is_tapped_once() {
        let elf = Spend::Cast(o(50), CastModeKind::Normal);
        let plain = [source(1, 0, &[ManaColor::Colorless], 1)];
        let restricted = [source(1, 1, &ANY, 1)];
        let pool = ManaPoolView::default();
        let mana = Mana {
            pool: &pool,
            plain: &plain,
            restricted: &restricted,
        };
        let legal = LegalActions {
            restricted_taps: vec![RestrictedTaps {
                taps: vec![(o(1), 1)],
                spends: vec![elf],
            }],
            ..LegalActions::default()
        };
        let sources = sources_for(&legal, elf, mana, None);
        assert_eq!(sources.len(), 1, "{sources:?}");
        assert!(plan_for(elf, &ManaCost::parse("{G}"), mana, &legal, None, None).is_some());
        assert!(plan_for(elf, &ManaCost::parse("{C}"), mana, &legal, None, None).is_some());
        assert!(plan_for(elf, &ManaCost::parse("{1}{G}"), mana, &legal, None, None).is_none());
        assert!(
            plan_for(elf, &ManaCost::parse("{G}"), mana, &legal, None, Some(o(1))).is_none(),
            "an activation's own source pays nothing restricted toward it"
        );
    }
}
