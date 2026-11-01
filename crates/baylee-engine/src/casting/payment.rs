//! Paying mana: what a pool can spend on what, and the payment itself.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary
use super::*;

mod source_pool;

/// Admitted or consumed mana units with their provenance and spend rider.
pub(super) type ManaEntries = SmallVec<[(RestrictedMana, ObjectId, SpendRider); 4]>;

/// A requirement whose filter reads the announced X, as the same spec over
/// every object and the filter it widened away. `None` for every other
/// requirement, and for the kinds no X-reading filter is printed on.
pub(crate) fn x_bounded(
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
        TargetSpec::CardInGraveyard(f, who) => (TargetSpec::CardInGraveyard(&ANY, who), f),
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
#[cfg(test)]
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
#[cfg(test)]
pub(crate) fn phyrexian_affordable(
    state: &GameState,
    player: PlayerId,
    pool: &ManaPool,
    cost: &ManaCost,
    settled: &[bool],
) -> bool {
    phyrexian_affordable_with(
        state,
        player,
        pool,
        cost,
        settled,
        mana_spending(state, player),
    )
}

pub(crate) fn phyrexian_affordable_with(
    state: &GameState,
    player: PlayerId,
    pool: &ManaPool,
    cost: &ManaCost,
    settled: &[bool],
    spending: ManaSpending,
) -> bool {
    phyrexian_payable(state, player, cost, settled, |cost| {
        mana_pay::can_pay_with(pool, cost, spending)
    })
}

pub(crate) fn source_affordable(
    state: &GameState,
    player: PlayerId,
    what: SpendFor,
    pool: &ManaPool,
    cost: &ManaCost,
    settled: &[bool],
    allowance: Option<SourceAllowance>,
) -> bool {
    let spending = payment_spending(state, player, what);
    if !super::source_cost::has_source_credits(state, pool) {
        return phyrexian_affordable_with(state, player, pool, cost, settled, spending);
    }
    let allowance = allowance.unwrap_or_else(|| live_allowance(state, player, what));
    phyrexian_payable(state, player, cost, settled, |cost| {
        source_plan(state, what, pool, cost, spending, None, allowance).is_some()
    })
}

fn phyrexian_payable(
    state: &GameState,
    player: PlayerId,
    cost: &ManaCost,
    settled: &[bool],
    pays: impl Fn(&ManaCost) -> bool,
) -> bool {
    let symbols = cost.phyrexian_count();
    let fixed = u32::try_from(settled.len()).unwrap_or(u32::MAX);
    // Eight symbols is 256 ways; no card prints more than four. Past that
    // only the mana is tried, which under-offers and never over-offers.
    if symbols == 0 || symbols > 8 {
        return fixed == 0 && pays(cost);
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
        state.can_pay_life(player, life) && pays(&cost.with_phyrexian_settled(mask))
    })
}

/// Pays a nonspell cost, admitting mana whose restriction allows such payments.
/// Spell and activation payments use [`pay_mana_for`].
#[cfg(test)]
pub(crate) fn pay_mana(state: &mut GameState, player: PlayerId, cost: &ManaCost) -> bool {
    if !admitted(state, player, SpendFor::Other).is_empty() {
        return pay_mana_for(state, player, SpendFor::Other, cost).is_some();
    }
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

/// Pay a resolving instruction's cost with its exact imposing source retained.
pub(crate) fn pay_mana_from(
    state: &mut GameState,
    player: PlayerId,
    source: baylee_core::ids::DamageSourceRef,
    cost: &ManaCost,
) -> bool {
    pay_mana_for(state, player, SpendFor::OtherFrom(source), cost).is_some()
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
                base.colors = if base.keywords.contains(baylee_cards_dsl::KeywordSet::DEVOID) {
                    baylee_core::color::ColorSet::EMPTY
                } else {
                    p.cost.colors()
                };
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
                base.defense = None;
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
    /// A different face announced before the card physically reaches the stack.
    SpellOnFace(ObjectId, usize),
    /// An announced prototype or face-down creature.
    SpellAs(ObjectId, SpellForm),
    /// A kicked spell's announced face/form, before its costs are paid.
    KickedSpell(ObjectId, usize, Option<SpellForm>),
    /// Activating an ability of this source (CR 602.2b, CR 113.7).
    Ability(ObjectId),
    /// Anything else that costs mana: suspend (a special action, CR 116.2f),
    /// an "unless you pay", echo, a pact. Powerstone mana admits these too.
    Other,
    /// A nonspell payment imposed by this exact card or ability source.
    OtherFrom(baylee_core::ids::DamageSourceRef),
    /// A cumulative upkeep payment (CR 702.24a), optionally naming its source.
    CumulativeUpkeep(Option<baylee_core::ids::DamageSourceRef>),
}

impl SpendFor {
    pub(crate) const fn is_other(self) -> bool {
        matches!(
            self,
            Self::Other | Self::OtherFrom(_) | Self::CumulativeUpkeep(_)
        )
    }
}

/// Borrow the ordinary spell; clone only an alternative projection needed by mana rules.
pub(super) fn spending_object(
    state: &GameState,
    what: SpendFor,
) -> Option<std::borrow::Cow<'_, crate::object::GameObject>> {
    use crate::state::CardLookup;
    use std::borrow::Cow;
    let (card, form, face) = match what {
        SpendFor::Spell(card) | SpendFor::Ability(card) => (card, None, None),
        SpendFor::SpellAs(card, form) => (card, Some(form), None),
        SpendFor::SpellOnFace(card, face) => (card, None, Some(face)),
        SpendFor::KickedSpell(card, face, form) => (card, form, Some(face)),
        SpendFor::Other | SpendFor::CumulativeUpkeep(None) => return None,
        SpendFor::OtherFrom(source) | SpendFor::CumulativeUpkeep(Some(source)) => {
            return state.source_object(source).map(Cow::Borrowed);
        }
    };
    let original = state.object(card)?;
    if let Some(form) = form {
        let mut projected = form.project(original);
        projected.kicked = matches!(what, SpendFor::KickedSpell(..));
        return Some(Cow::Owned(projected));
    }
    if let Some(face) = face {
        let def = state.bases.card(original.card?.index)?;
        def.faces.get(face)?;
        let mut projected = original.clone();
        projected.base = std::sync::Arc::new(crate::object::Characteristics::from_face(
            def,
            face,
            original.base.name,
        ));
        projected.cache.clear();
        projected.face_index = u8::try_from(face).ok()?;
        projected.kicked = matches!(what, SpendFor::KickedSpell(..));
        return Some(Cow::Owned(projected));
    }
    Some(Cow::Borrowed(original))
}

/// The restricted mana in `player`'s pool that may pay for `what`, in pool
/// order, each with the permanent that made it and the rider it carries.
///
/// An entry counts when its own filter matches the spell. The rider is not
/// consulted: it changes what the spell becomes once the mana is spent,
/// never whether the mana may be spent. Ability restrictions match the
/// actual source, including sources outside the battlefield. The walk is over the pool's list and never over the map, which
/// is keyed by a hash.
#[cfg(test)]
pub(super) fn admitted(state: &GameState, player: PlayerId, what: SpendFor) -> ManaEntries {
    admitted_from_pool(
        state,
        what,
        &state.players[usize::from(player.get())].mana_pool,
    )
}

/// Read the actual payment pool, which may exclude reserved mana.
pub(super) fn admitted_from_pool(
    state: &GameState,
    what: SpendFor,
    pool: &ManaPool,
) -> ManaEntries {
    if pool.restricted().is_empty() {
        return SmallVec::new();
    }
    let spell = spending_object(state, what);
    let ability = matches!(what, SpendFor::Ability(_));
    pool.restricted()
        .iter()
        .filter_map(|mana| {
            let provenance = state.restriction_info.get(&mana.restriction.0)?;
            // Source credits remain a distinct resource class, even when the
            // base is eligible. Never merge them through saturating counters.
            if super::source_cost::is_source_credit(state, *mana) {
                return None;
            }
            let allows = if what.is_other() {
                provenance.restriction.uses
                    == baylee_cards_dsl::ManaUse::NonSpellPaymentsOrMatchingSpells
                    || spell
                        .as_ref()
                        .is_some_and(|source| provenance.admits_source_cost(state, source))
                    || (matches!(what, SpendFor::CumulativeUpkeep(_))
                        && provenance.admits_cumulative_upkeep(state, spell.as_deref()))
            } else {
                spell
                    .as_ref()
                    .is_some_and(|spell| provenance.admits(state, spell, ability))
            };
            allows.then_some((
                *mana,
                provenance.context.source,
                provenance.restriction.rider,
            ))
        })
        .collect()
}

/// The mana in `player`'s pool that carries a rider for `what` and restricts
/// nothing (#232), in pool order, each with the permanent that made it and
/// its rider: the units whose filter matches the spell.
///
/// These units are in the plain counters already, so they are nothing to
/// merge. An action the restriction does not match spends them as ordinary
/// mana without triggering its rider (CR 106.6).
pub(super) fn ridden_for(state: &GameState, player: PlayerId, what: SpendFor) -> ManaEntries {
    if what.is_other() {
        return SmallVec::new();
    }
    if state.players[usize::from(player.get())]
        .mana_pool
        .ridden()
        .is_empty()
    {
        return SmallVec::new();
    }
    let Some(spell) = spending_object(state, what) else {
        return SmallVec::new();
    };
    state.players[player.get() as usize]
        .mana_pool
        .ridden()
        .iter()
        .filter_map(|mana| {
            let provenance = state.restriction_info.get(&mana.restriction.0)?;
            provenance
                .admits(state, &spell, matches!(what, SpendFor::Ability(_)))
                .then_some((
                    *mana,
                    provenance.context.source,
                    provenance.restriction.rider,
                ))
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
    let original = &state.players[player.get() as usize].mana_pool;
    let free = state
        .constrained_payment(player)
        .filter(|_| what.is_other())
        .map(|payment| {
            original
                .payment_receipt(&payment.required)
                .unwrap_or_default()
        });
    let pool = free.as_ref().unwrap_or(original);
    if pool.restricted().is_empty() {
        return free;
    }
    Some(merged(pool, &admitted_from_pool(state, what, pool)))
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
    let credits: u64 = super::source_cost::source_entries(state, pool)
        .iter()
        .map(|(mana, ..)| u64::from(mana.amount))
        .sum();
    u32::try_from(total.saturating_add(credits)).unwrap_or(u32::MAX)
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
) -> Option<ManaEntries> {
    pay_mana_restricting_generic(state, player, what, cost, None)
}

/// The ordinary payment with an actual-mana restriction on a generic part.
pub(crate) fn pay_mana_restricting_generic(
    state: &mut GameState,
    player: PlayerId,
    what: SpendFor,
    cost: &ManaCost,
    restriction: Option<crate::mana_pay::GenericRestriction>,
) -> Option<ManaEntries> {
    let allowance = if super::source_cost::has_source_credits(
        state,
        &state.players[usize::from(player.get())].mana_pool,
    ) {
        live_allowance(state, player, what)
    } else {
        SourceAllowance::default()
    };
    pay_mana_scoped(state, player, what, cost, restriction, allowance)
}

pub(crate) fn pay_mana_scoped(
    state: &mut GameState,
    player: PlayerId,
    what: SpendFor,
    cost: &ManaCost,
    restriction: Option<crate::mana_pay::GenericRestriction>,
    allowance: SourceAllowance,
) -> Option<ManaEntries> {
    pay_mana_scoped_recording_x(state, player, what, cost, restriction, allowance)
        .map(|(spent, _)| spent)
}

/// The payment assignment records which actual units paid the restricted X.
pub(crate) fn pay_mana_scoped_recording_x(
    state: &mut GameState,
    player: PlayerId,
    what: SpendFor,
    cost: &ManaCost,
    restriction: Option<crate::mana_pay::GenericRestriction>,
    allowance: SourceAllowance,
) -> Option<(ManaEntries, [u32; 6])> {
    let (pool, spent, x_paid) =
        plan_mana_scoped_with_x(state, player, what, cost, restriction, allowance)?;
    let obligation = state.constrained_payment(player).cloned();
    commit_mana_payment(
        state,
        player,
        pool,
        selected_obligation(state, player, what),
        obligation.as_ref().filter(|_| !what.is_other()),
    )?;
    Some((spent, x_paid))
}

/// Pure preview retains exact restricted identities, including mandatory units.
pub(crate) fn plan_mana_scoped(
    state: &GameState,
    player: PlayerId,
    what: SpendFor,
    cost: &ManaCost,
    restriction: Option<crate::mana_pay::GenericRestriction>,
    allowance: SourceAllowance,
) -> Option<(ManaPool, ManaEntries)> {
    plan_mana_scoped_with_x(state, player, what, cost, restriction, allowance)
        .map(|(pool, spent, _)| (pool, spent))
}

fn plan_mana_scoped_with_x(
    state: &GameState,
    player: PlayerId,
    what: SpendFor,
    cost: &ManaCost,
    restriction: Option<crate::mana_pay::GenericRestriction>,
    allowance: SourceAllowance,
) -> Option<(ManaPool, ManaEntries, [u32; 6])> {
    let obligation = state.constrained_payment(player).cloned();
    let is_selected_spell = selected_obligation(state, player, what);
    let spending = payment_spending(state, player, what);
    // Only unrelated payments must leave obligated mana untouched. Eligible
    // land mana abilities may consume it while paying for the instructed card.
    let protect_reserved = what.is_other();
    let original = &state.players[player.get() as usize].mana_pool;
    let free = if let Some(payment) = obligation.as_ref().filter(|_| protect_reserved) {
        Some(original.payment_receipt(&payment.required)?)
    } else {
        None
    };
    let real = free.as_ref().unwrap_or(original);
    let mut entries = admitted_from_pool(state, what, real);
    entries.retain(|(mana, ..)| !super::source_cost::is_source_credit(state, *mana));
    entries.extend(super::source_cost::source_entries(state, real));
    let mut riding = ridden_for(state, player, what);
    // Ordinary producer tracking must not displace a beneficial mana rider.
    riding.sort_by_key(|(_, _, rider)| *rider == SpendRider::None);
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
    let (mut pool, spent, x_paid) = source_pool::Payment {
        state,
        real,
        entries,
        riding,
        cost,
        spending,
        restriction,
        allowance,
        obligation: obligation.as_ref().filter(|_| !protect_reserved),
        must_consume: is_selected_spell,
    }
    .pay()?;
    if free.is_some() {
        let spent = real.payment_receipt(&pool)?;
        pool = original.payment_receipt(&spent)?;
    }
    if is_selected_spell && let Some(payment) = obligation.as_ref() {
        let receipt = original.payment_receipt(&pool)?;
        if !required_receipt_satisfied(&receipt, &payment.required) {
            return None;
        }
    }
    Some((pool, spent, x_paid))
}

fn selected_obligation(state: &GameState, player: PlayerId, what: SpendFor) -> bool {
    state
        .constrained_payment(player)
        .is_some_and(|payment| match what {
            SpendFor::Spell(card)
            | SpendFor::SpellAs(card, _)
            | SpendFor::SpellOnFace(card, _)
            | SpendFor::KickedSpell(card, ..) => {
                payment.card.object == card && state.source_identity(card) == Some(payment.card)
            }
            _ => false,
        })
}

fn required_receipt_satisfied(receipt: &ManaPool, required: &ManaPool) -> bool {
    required.restricted().iter().all(|unit| {
        receipt
            .restricted()
            .iter()
            .filter(|spent| spent.restriction == unit.restriction)
            .map(|spent| u32::from(spent.amount))
            .sum::<u32>()
            >= u32::from(unit.amount)
    })
}

/// Debit provenance before ordinary units, preserving snow and attached riders.
fn debit_tracked_units(
    real: &ManaPool,
    entries: ManaEntries,
    riding: ManaEntries,
    mut used_plain: [u32; 6],
    mut used_snow: [u32; 6],
) -> Option<(ManaPool, ManaEntries)> {
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
    Some((pool, spent))
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
        if !required_receipt_satisfied(&receipt, &payment.required) {
            return None;
        }
    }
    if before.payment_receipt(&pool)?.total() > u64::from(u32::MAX) {
        state.numeric_failure = Some("one recorded mana payment exceeds u32 total units");
        return None;
    }
    state.players[player.get() as usize].mana_pool = pool;
    if obligation.is_some() {
        state.note_constrained_payment(player, &before);
    }
    Some(())
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

pub(crate) fn payment_spending(
    state: &GameState,
    player: PlayerId,
    what: SpendFor,
) -> ManaSpending {
    match what {
        SpendFor::Spell(card)
        | SpendFor::SpellAs(card, _)
        | SpendFor::SpellOnFace(card, _)
        | SpendFor::KickedSpell(card, ..) => spell_mana_spending(state, player, card),
        SpendFor::Ability(source) => ability_mana_spending(state, player, source),
        _ => mana_spending(state, player),
    }
}
