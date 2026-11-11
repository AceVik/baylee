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
#[must_use]
pub fn plan_for(
    spend: Spend,
    cost: &ManaCost,
    mana: Mana<'_>,
    legal: &LegalActions,
    life: Option<i32>,
    except: Option<ObjectId>,
) -> Option<Plan> {
    let pool = pool_for(mana.pool, legal, spend);
    let plan = |sources: &[Source], with_life: bool| match life {
        Some(life) if with_life => manaplan::plan_with_life(cost, &pool, sources, life),
        _ => manaplan::plan(cost, &pool, sources),
    };
    let named = legal
        .restricted_taps_for(spend)
        .iter()
        .any(|(source, _)| Some(*source) != except);
    let all = named.then(|| sources_for(legal, spend, mana, except));
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
