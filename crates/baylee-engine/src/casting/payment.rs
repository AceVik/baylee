//! Paying mana: what a pool can spend on what, and the payment itself.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary
use super::*;

/// A requirement whose filter reads the announced X, as the same spec over
/// every object and the filter it widened away. `None` for every other
/// requirement, and for the kinds no X-reading filter is printed on.
pub(super) fn x_bounded(
    spec: baylee_cards_dsl::TargetSpec,
) -> Option<(
    baylee_cards_dsl::TargetSpec,
    &'static baylee_cards_dsl::Filter,
)> {
    use baylee_cards_dsl::{Filter, TargetSpec};
    static ANY: Filter = Filter::Any;
    let (widened, filter) = match spec {
        TargetSpec::Spell(f) => (TargetSpec::Spell(&ANY), f),
        TargetSpec::Object(f) => (TargetSpec::Object(&ANY), f),
        TargetSpec::StackOrBattlefield(f) => (TargetSpec::StackOrBattlefield(&ANY), f),
        _ => return None,
    };
    crate::eval::reads_announced_x(filter).then_some((widened, filter))
}

/// Whether `player` can spend `pool` to cover `cost` under current permissions.
///
/// A Phyrexian symbol is payable with its mana or with 2 life (CR 107.4f),
/// so a cost holding any counts as covered when some way of settling them
/// leaves mana the pool covers and life the player can pay (CR 119.4).
/// That is [`phyrexian_affordable`] with nothing settled yet: the cast
/// wizard's Phyrexian question asks the same reader with its answers so
/// far, so the offer and the question cannot disagree.
pub(crate) fn affordable(
    state: &GameState,
    player: PlayerId,
    pool: &ManaPool,
    cost: &ManaCost,
) -> bool {
    phyrexian_affordable(state, player, pool, cost, &[])
}

/// [`affordable`] with the first `settled.len()` Phyrexian symbols already
/// announced (`true` = 2 life, CR 601.2b) and every way of announcing the
/// rest tried. A cost with no Phyrexian symbol is the pool's question alone.
pub(crate) fn phyrexian_affordable(
    state: &GameState,
    player: PlayerId,
    pool: &ManaPool,
    cost: &ManaCost,
    settled: &[bool],
) -> bool {
    let spending = mana_spending(state, player);
    let symbols = cost.phyrexian_count();
    let fixed = u32::try_from(settled.len()).unwrap_or(u32::MAX);
    // Eight symbols is 256 ways; no card prints more than four. Past that
    // only the mana is tried, which under-offers and never over-offers.
    if symbols == 0 || symbols > 8 {
        return fixed == 0 && mana_pay::can_pay_with(pool, cost, spending);
    }
    if fixed > symbols {
        return false;
    }
    let base = settled
        .iter()
        .enumerate()
        .fold(0u32, |mask, (i, life)| mask | (u32::from(*life) << i));
    (0..(1u32 << (symbols - fixed))).any(|rest| {
        let mask = base | (rest << fixed);
        let life = 2 * i32::try_from(mask.count_ones()).unwrap_or(i32::MAX);
        state.can_pay_life(player, life)
            && mana_pay::can_pay_with(pool, &cost.with_phyrexian_settled(mask), spending)
    })
}

/// Pays a cost that admits no restricted mana, under this player's current
/// spending permissions. Spell and activation payments use [`pay_mana_for`].
pub(crate) fn pay_mana(state: &mut GameState, player: PlayerId, cost: &ManaCost) -> bool {
    let spending = mana_spending(state, player);
    if let Some(obligation) = state.constrained_payment(player) {
        let real = &state.players[player.get() as usize].mana_pool;
        let Some(free) = real.payment_receipt(&obligation.required) else {
            return false;
        };
        let mut remainder = free.clone();
        if !mana_pay::pay_with(&mut remainder, cost, spending) {
            return false;
        }
        let Some(spent) = free.payment_receipt(&remainder) else {
            return false;
        };
        let Some(after) = real.payment_receipt(&spent) else {
            return false;
        };
        state.players[player.get() as usize].mana_pool = after;
        return true;
    }
    mana_pay::pay_with(
        &mut state.players[player.get() as usize].mana_pool,
        cost,
        spending,
    )
}

/// All actual mana units by type, with restricted units counted once.
pub(crate) fn mana_units_by_type(pool: &ManaPool) -> [u64; 6] {
    ManaColor::ALL.map(|color| {
        u64::from(pool.available(color))
            + pool
                .restricted()
                .iter()
                .filter(|unit| unit.color == color)
                .map(|unit| u64::from(unit.amount))
                .sum::<u64>()
    })
}

/// Record the actual types consumed by one successful mana payment.
pub(crate) fn record_mana_payment(
    paid: &mut crate::object::PaidRecord,
    before: &ManaPool,
    after: &ManaPool,
) {
    // Captured immediately around the payer; no other operation can produce
    // mana or alter restrictions between these snapshots.
    paid.mana_paid = before
        .payment_receipt(after)
        .expect("a successful payment only consumes existing mana");
    // The payer checks the finite single-payment domain before committing.
    paid.mana_types_spent = mana_units_by_type(&paid.mana_paid)
        .map(|n| u32::try_from(n).expect("checked payment domain"));
    paid.mana_spent = u32::try_from(paid.mana_paid.total()).expect("checked payment domain");
    paid.colors_spent = baylee_core::color::Color::ALL
        .into_iter()
        .filter(|&color| paid.mana_types_spent[ManaColor::from_color(color).index()] > 0)
        .fold(baylee_core::color::ColorSet::EMPTY, |set, color| {
            set.union(baylee_core::color::ColorSet::of(color))
        });
}

/// What a payment is for, which is what restricted mana asks (CR 106.6).
///
/// Cavern of Souls' mana may pay for a creature spell of the named type and
/// for nothing else, so the same pool answers two payments two ways. The
/// offer and the payment have to put the same question to it, or a spell
/// is offered in `LegalActions` and refused the moment it is taken.
/// Characteristics used to announce a special casting form, before payment.
#[derive(Clone, Copy, Debug)]
pub(crate) enum SpellForm {
    Prototype(baylee_cards_dsl::Prototype),
    Disguise,
}

impl SpellForm {
    pub(crate) fn project(self, object: &crate::object::GameObject) -> crate::object::GameObject {
        let mut object = object.clone();
        object.base = std::sync::Arc::new(object.characteristics().clone());
        object.cache.clear();
        let base = object.base_mut();
        match self {
            Self::Prototype(p) => {
                base.mana_cost = p.cost;
                base.colors = p.cost.colors();
                base.power = Some(p.power);
                base.toughness = Some(p.toughness);
            }
            Self::Disguise => {
                base.name = crate::state::NAMELESS;
                base.mana_cost = ManaCost::ZERO;
                base.colors = baylee_core::color::ColorSet::EMPTY;
                base.types = TypeSet::CREATURE;
                base.supertypes = baylee_core::types::SupertypeSet::EMPTY;
                base.subtypes = baylee_core::types::SubtypeSet::EMPTY;
                base.keywords = baylee_cards_dsl::KeywordSet::EMPTY;
                base.power = Some(2);
                base.toughness = Some(2);
                base.loyalty = None;
                base.produced_colors = baylee_core::color::ColorSet::EMPTY;
                base.produced_colorless = false;
                base.produced_chosen = false;
            }
        }
        object
    }

    pub(super) fn cost(self) -> ManaCost {
        match self {
            Self::Prototype(p) => p.cost,
            Self::Disguise => const { ManaCost::parse("{3}") },
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum SpendFor {
    /// Casting this spell (CR 601.2h).
    Spell(ObjectId),
    /// An announced prototype or face-down creature.
    SpellAs(ObjectId, SpellForm),
    /// Activating an ability of this source (CR 602.2b, CR 113.7).
    Ability(
        #[expect(
            dead_code,
            reason = "no restriction the DSL can write names an ability yet"
        )]
        ObjectId,
    ),
    /// Anything else that costs mana: suspend (a special action, CR 116.2f),
    /// an "unless you pay", echo, a pact. No restriction names these.
    Other,
}

/// The restricted mana in `player`'s pool that may pay for `what`, in pool
/// order, each with the permanent that made it and the rider it carries.
///
/// An entry counts when its own filter matches the spell. The rider is not
/// consulted: it changes what the spell becomes once the mana is spent,
/// never whether the mana may be spent. A spell restriction admits no
/// ability. The walk is over the pool's list and never over the map, which
/// is keyed by a hash.
pub(super) fn admitted(
    state: &GameState,
    player: PlayerId,
    what: SpendFor,
) -> SmallVec<[(RestrictedMana, ObjectId, SpendRider); 4]> {
    let (card, form) = match what {
        SpendFor::Spell(card) => (card, None),
        SpendFor::SpellAs(card, form) => (card, Some(form)),
        _ => return SmallVec::new(),
    };
    let Some(original) = state.object(card) else {
        return SmallVec::new();
    };
    let projected = form.map(|f| f.project(original));
    let spell = projected.as_ref().unwrap_or(original);
    state.players[player.get() as usize]
        .mana_pool
        .restricted()
        .iter()
        .filter_map(|mana| {
            let &(source, filter, rider) = state.restriction_info.get(&mana.restriction.0)?;
            crate::eval::matches(filter, state, spell, player, source)
                .then_some((*mana, source, rider))
        })
        .collect()
}

/// The mana in `player`'s pool that carries a rider for `what` and restricts
/// nothing (#232), in pool order, each with the permanent that made it and
/// its rider: the units whose filter matches the spell.
///
/// These units are in the plain counters already, so they are nothing to
/// merge. A spell the filter does not match, and every ability, spends them
/// as the ordinary mana they are, and no rider goes off (CR 106.6).
pub(super) fn ridden_for(
    state: &GameState,
    player: PlayerId,
    what: SpendFor,
) -> SmallVec<[(RestrictedMana, ObjectId, SpendRider); 4]> {
    let (card, form) = match what {
        SpendFor::Spell(card) => (card, None),
        SpendFor::SpellAs(card, form) => (card, Some(form)),
        _ => return SmallVec::new(),
    };
    let Some(original) = state.object(card) else {
        return SmallVec::new();
    };
    let projected = form.map(|f| f.project(original));
    let spell = projected.as_ref().unwrap_or(original);
    state.players[player.get() as usize]
        .mana_pool
        .ridden()
        .iter()
        .filter_map(|mana| {
            let &(source, filter, rider) = state.restriction_info.get(&mana.restriction.0)?;
            crate::eval::matches(filter, state, spell, player, source)
                .then_some((*mana, source, rider))
        })
        .collect()
}

/// `pool` with every admitted restricted unit added to its plain counters.
pub(super) fn merged(
    pool: &ManaPool,
    entries: &[(RestrictedMana, ObjectId, SpendRider)],
) -> ManaPool {
    let mut merged = pool.clone();
    for (mana, ..) in entries {
        if mana.flags.contains(ManaFlags::SNOW) {
            merged.add_snow(mana.color, u32::from(mana.amount));
        } else {
            merged.add(mana.color, u32::from(mana.amount));
        }
    }
    merged
}

/// The pool a payment for `what` may draw on, or `None` when that is simply
/// the player's pool.
///
/// Restricted mana does not live in the pool's plain counters, and
/// [`mana_pay::can_pay`] reads nothing else. So a player whose only mana
/// came off a Cavern was offered **nothing to cast**, while the payment on
/// the far side of the wizard would have spent it without complaint. The
/// offer and the payment were answering different questions about the same
/// pool. They ask one question here, and [`pay_mana_for`] pays from this
/// same pool.
///
/// The card is still in a hand rather than on the stack, which is the one
/// difference from the payment site and does not reach these filters: they
/// read characteristics and a chosen subtype, neither of which the stack
/// confers.
///
/// It is `pub(crate)` because it has **two** callers and they must not
/// disagree: the wizard enumerates the ways to cast a spell with the same
/// probe `can_cast` used to offer it, or a spell is offered in
/// `LegalActions` and then refused as "no way to cast this spell" — which is
/// exactly what the convoke count above this one was written to stop
/// happening.
pub(crate) fn spendable_pool(
    state: &GameState,
    player: PlayerId,
    what: SpendFor,
) -> Option<ManaPool> {
    let pool = &state.players[player.get() as usize].mana_pool;
    if pool.restricted().is_empty() {
        return None;
    }
    Some(merged(pool, &admitted(state, player, what)))
}

/// Planning a payment may count optional life-to-mana actions. Their cost
/// remains a separate explicit player action; actual payment never uses this pool.
pub(crate) fn planning_pool(
    state: &GameState,
    player: PlayerId,
    what: SpendFor,
    reserved_life: u32,
) -> Option<ManaPool> {
    let mut merged = spendable_pool(state, player, what);
    let amount = state.granted_colorless_after_life(player, reserved_life);
    if amount > 0 {
        let pool = merged
            .get_or_insert_with(|| state.players[usize::from(player.get())].mana_pool.clone());
        pool.add(ManaColor::Colorless, amount);
    }
    merged
}

/// Mana units this payment can spend, without counting the restricted
/// entries a second time after merging them into the plain counters.
pub(crate) fn spendable_units(state: &GameState, player: PlayerId, what: SpendFor) -> u32 {
    let merged = spendable_pool(state, player, what);
    let pool = merged
        .as_ref()
        .unwrap_or(&state.players[usize::from(player.get())].mana_pool);
    let total: u64 = ManaColor::ALL
        .iter()
        .map(|color| u64::from(pool.available(*color)))
        .sum();
    u32::try_from(total).unwrap_or(u32::MAX)
}

/// Greatest affordable value within a known finite bound. Increasing X
/// cannot make a fixed cost cheaper, so this takes at most 32 probes even
/// when a player has a very large mana pool or life total.
pub(crate) fn greatest_affordable(mut upper: u32, mut affordable: impl FnMut(u32) -> bool) -> u32 {
    let mut lower = 0;
    while lower < upper {
        let middle = lower + (upper - lower).div_ceil(2);
        if affordable(middle) {
            lower = middle;
        } else {
            upper = middle - 1;
        }
    }
    lower
}

/// Pays `cost` for `what` out of `player`'s pool, and returns the restricted
/// mana it spent, and the rider-carrying mana it spent on a spell that rider
/// names ([`ridden_for`]), each part with its source and rider. `None`
/// leaves the pool untouched, because partial payments are not allowed
/// (CR 601.2h).
///
/// The payment is solved once, on the same pool [`spendable_pool`] offers
/// from, so it pays whatever was offered. The solver prefers the admitted
/// restricted units wherever it has a choice. That is what spends the
/// Cavern's unit rather than a land's beside it, so the rider lands. It
/// then charges what was consumed to the restricted entries first, colour
/// by colour and snow apart from the rest, and takes only what is left from
/// the plain counters. Surplus restricted mana stays in the pool as it was.
///
/// The payer this replaces took each admitted entry whole and subtracted it
/// from the cost one unit at a time, generic before the coloured pip,
/// because `ManaCost` sorts the generic part first. So a Cavern's {U} paid
/// the {1} of a {1}{U} spell and left the {U} to a Forest, which cannot pay
/// it. Three {C} from a Workshop were all taken for a {1}. A restricted {G}
/// that matched no pip was taken and lost. Under Mycosynth Lattice it
/// matched colours literally, so it refused what the Lattice made payable.
pub(crate) fn pay_mana_for(
    state: &mut GameState,
    player: PlayerId,
    what: SpendFor,
    cost: &ManaCost,
) -> Option<SmallVec<[(RestrictedMana, ObjectId, SpendRider); 4]>> {
    pay_mana_restricting_generic(state, player, what, cost, None)
}

/// The ordinary payment with an actual-mana restriction on a generic part.
pub(crate) fn pay_mana_restricting_generic(
    state: &mut GameState,
    player: PlayerId,
    what: SpendFor,
    cost: &ManaCost,
    restriction: Option<(ManaColor, u32)>,
) -> Option<SmallVec<[(RestrictedMana, ObjectId, SpendRider); 4]>> {
    let obligation = state.constrained_payment(player).cloned();
    let is_selected_spell = obligation.as_ref().is_some_and(|payment| match what {
        SpendFor::Spell(card) | SpendFor::SpellAs(card, _) => {
            payment.card.object == card && state.source_identity(card) == Some(payment.card)
        }
        _ => false,
    });
    let spending = mana_spending(state, player);
    let mut entries = admitted(state, player, what);
    let mut riding = ridden_for(state, player, what);
    if let Some(payment) = &obligation {
        entries.sort_by_key(|(mana, ..)| {
            !payment
                .required
                .restricted()
                .iter()
                .any(|unit| unit.restriction == mana.restriction)
        });
        riding.sort_by_key(|(mana, ..)| {
            !payment
                .required
                .ridden()
                .iter()
                .any(|unit| unit.restriction == mana.restriction)
        });
    }
    let real = &state.players[player.get() as usize].mana_pool;
    let merged = merged(real, &entries);
    let mut prefer = [0_u32; 6];
    for (mana, ..) in entries.iter().chain(&riding) {
        let slot = &mut prefer[mana.color.index()];
        *slot = slot.saturating_add(u32::from(mana.amount));
    }
    let Some(required) = obligation.as_ref().map_or(Some([0; 6]), |payment| {
        crate::constrained_payment::amounts(&payment.required)
    }) else {
        state.numeric_failure = Some("generated-mana obligation exceeds u32 per color");
        return None;
    };
    for i in 0..6 {
        prefer[i] = prefer[i].max(required[i]);
    }
    let paid = mana_pay::payment_consuming(
        &merged,
        cost,
        spending,
        prefer,
        restriction,
        if is_selected_spell { required } else { [0; 6] },
    )?;

    let (mut used_plain, mut used_snow) = consumed_mana(&merged, &paid, obligation.as_ref());

    // Restricted entries first, in pool order, each class charged apart.
    let mut pool = real.clone();
    let mut spent = SmallVec::new();
    for (mana, source, rider) in entries {
        let budget = if mana.flags.contains(ManaFlags::SNOW) {
            &mut used_snow[mana.color.index()]
        } else {
            &mut used_plain[mana.color.index()]
        };
        let k =
            u16::try_from(u32::from(mana.amount).min(*budget)).expect("bounded by entry amount");
        if k == 0 {
            continue;
        }
        let taken = pool.take_restricted_units(mana.restriction.0, k)?;
        *budget -= u32::from(taken.amount);
        spent.push((taken, source, rider));
    }
    // Then the rider units the spell sets off, out of the plain counters
    // they are counted in, before an ordinary spend keeps them back.
    for (mana, source, rider) in riding {
        let budget = if mana.flags.contains(ManaFlags::SNOW) {
            &mut used_snow[mana.color.index()]
        } else {
            &mut used_plain[mana.color.index()]
        };
        let k =
            u16::try_from(u32::from(mana.amount).min(*budget)).expect("bounded by entry amount");
        if k == 0 {
            continue;
        }
        let taken = pool.take_ridden_units(mana.restriction.0, k)?;
        *budget -= u32::from(taken.amount);
        spent.push((taken, source, rider));
    }
    // The rest off the plain counters. The solver paid from the real pool
    // plus the admitted units, and every class was charged to those units
    // first, so what is left fits the real pool. A remainder that did not
    // would be a solver fault, and it refuses rather than half-paying.
    for color in ManaColor::ALL {
        if !pool.spend(color, used_plain[color.index()]) {
            return None;
        }
        if !pool.spend_snow_units(color, used_snow[color.index()]) {
            return None;
        }
    }
    commit_mana_payment(state, player, pool, is_selected_spell, obligation.as_ref())?;
    Some(spent)
}

/// Validate the exact constrained receipt before publishing any pool debit.
pub(super) fn commit_mana_payment(
    state: &mut GameState,
    player: PlayerId,
    pool: ManaPool,
    is_selected_spell: bool,
    obligation: Option<&crate::constrained_payment::ConstrainedPayment>,
) -> Option<()> {
    let before = state.players[player.get() as usize].mana_pool.clone();
    if is_selected_spell && let Some(payment) = obligation {
        let receipt = before.payment_receipt(&pool)?;
        if payment.required.restricted().iter().any(|unit| {
            receipt
                .restricted()
                .iter()
                .filter(|spent| spent.restriction == unit.restriction)
                .map(|spent| u32::from(spent.amount))
                .sum::<u32>()
                < u32::from(unit.amount)
        }) {
            return None;
        }
    }
    if before.payment_receipt(&pool)?.total() > u64::from(u32::MAX) {
        state.numeric_failure = Some("one recorded mana payment exceeds u32 total units");
        return None;
    }
    state.players[player.get() as usize].mana_pool = pool;
    state.note_constrained_payment(player, &before);
    Some(())
}

pub(super) fn consumed_mana(
    merged: &ManaPool,
    paid: &ManaPool,
    obligation: Option<&crate::constrained_payment::ConstrainedPayment>,
) -> ([u32; 6], [u32; 6]) {
    // What the payment consumed, per colour, and how much of it was snow.
    // Exact, because `spend` takes ordinary units before snow ones and
    // `spend_snow` takes only snow ones.
    let mut used_snow = [0_u32; 6];
    let mut used_plain = [0_u32; 6];
    for color in ManaColor::ALL {
        let used = merged.available(color) - paid.available(color);
        let mut snow = merged.snow_available(color) - paid.snow_available(color);
        if let Some(payment) = obligation {
            let wanted = u64::from(payment.required.snow_available(color))
                + crate::constrained_payment::restricted_snow(&payment.required, color);
            snow =
                snow.max(u32::try_from(wanted.min(u64::from(used))).expect("bounded by used mana"));
        }
        used_snow[color.index()] = snow;
        used_plain[color.index()] = used - snow;
    }

    (used_plain, used_snow)
}

/// Whether a face prints a mana cost at all (CR 202.1b).
///
/// A card with **no** mana cost cannot be cast unless something else gives it
/// a cost or lets it be cast without paying one — and that is a different
/// thing from a cost of `{0}`, which is paid by paying nothing and is a
/// perfectly ordinary spell. `ManaCost` keeps the two apart and always has:
/// Ornithopter's `{0}` is one `Generic(0)` symbol, Ancestral Vision's blank
/// is no symbols at all, and `to_string` writes them as `"{0}"` and `""`.
///
/// Nothing read that difference. Every probe here asks only whether the pool
/// covers the cost, and a pool covers a blank cost trivially, so Ancestral
/// Vision — a card whose entire text is a suspend ability and three drawn
/// cards — sat in `legal.castable` from the hand of anybody who reached their
/// main phase, castable for nothing.
///
/// Ask it of a **printed** cost and never of one that has been through the
/// cost arithmetic: `with_less_generic` rebuilds a cost symbol by symbol and
/// does not write back a `Generic(0)`, so `{0}` reduced by nothing comes out
/// blank. Both callers ask before any reduction, which is also what the rule
/// means — a discount that takes `{1}` down to nothing leaves a spell that is
/// cast for free and was never in question here.
pub(crate) fn has_a_printed_cost(cost: &ManaCost) -> bool {
    cost.symbols().next().is_some()
}
