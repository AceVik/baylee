//! Mana prices, reductions, increases, and resource estimates.

#[allow(clippy::wildcard_imports)]
use super::*;

/// Whether any effect lets mana be spent as though it were mana of any
/// color (Mycosynth Lattice).
///
/// The payment step has always honoured this; the *affordability* checks
/// did not, so the Lattice's third line was unreachable — the spell it made
/// payable was never offered as castable in the first place.
#[must_use]
pub fn mana_is_wild(state: &GameState) -> bool {
    state.effects.may_change_mana_spending()
        && state
            .effects
            .iter()
            .any(|fx| matches!(fx.modifier, baylee_cards_dsl::Modifier::ManaIsAnyColor))
}

/// The live permissions for `player` to spend actual mana as another type.
/// Static-effect synchronization supplies the source's current controller
/// and removes effects whose source left, phased out or lost its ability.
#[must_use]
pub fn mana_spending(state: &GameState, player: PlayerId) -> ManaSpending {
    if !state.effects.may_change_mana_spending() {
        return ManaSpending::EXACT;
    }
    let mut spending = if mana_is_wild(state) {
        ManaSpending::ANY_COLOR
    } else {
        ManaSpending::EXACT
    };
    for fx in state.effects.iter().filter(|fx| fx.controller == player) {
        if let baylee_cards_dsl::Modifier::SpendManaAs { from, to } = fx.modifier {
            spending.allow(from, to);
        }
    }
    spending
}

/// Spending permissions for an activated ability of this exact source.
/// Creature-only permissions leave spells and colorless requirements unchanged.
#[must_use]
pub fn ability_mana_spending(
    state: &GameState,
    player: PlayerId,
    source: ObjectId,
) -> ManaSpending {
    if !state.effects.may_change_mana_spending() {
        return ManaSpending::EXACT;
    }
    let mut spending = mana_spending(state, player);
    let creature = state.object(source).is_some_and(|object| {
        object.zone == Zone::Battlefield
            && object.controller == player
            && object.characteristics().types.contains(TypeSet::CREATURE)
    });
    if creature
        && state.effects.iter().any(|effect| {
            effect.controller == player
                && effect.modifier == baylee_cards_dsl::Modifier::CreatureAbilityManaAnyColor
        })
    {
        for actual in ManaColor::ALL {
            for required in ManaColor::ALL
                .into_iter()
                .filter(|color| *color != ManaColor::Colorless)
            {
                spending.allow(actual, required);
            }
        }
    }
    spending
}

/// Spending permission follows the exact exiled card incarnation, not its owner.
pub(crate) fn spell_mana_spending(
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
) -> ManaSpending {
    if state.object(card).is_some_and(|object| {
        state.play_permissions.iter().any(|permission| {
            permission.player == player
                && permission.card == card
                && permission.version == object.version
                && permission.mana_any_type
        })
    }) {
        ManaSpending::ANY_TYPE
    } else {
        mana_spending(state, player)
    }
}

/// The permanents convoke may be paid with (CR 702.51a): untapped creatures
/// the caster controls, one `{1}` each.
///
/// One function because the offer and the payment must not disagree. The
/// wizard enumerated these to ask which to tap, and `can_cast` did not
/// count them at all — so a convoke spell was offered as castable exactly
/// when its printed cost was already payable, which is the one case convoke
/// is not for. Clever Concealment and Spirit Water Revival were
/// `Coverage::Implemented` and could never actually be convoked.
///
/// Creatures only: artifacts are what [`waterbend_sources`] adds, and the two
/// were one walk until #229, so a Darksteel Pendant convoked. Over
/// [`GameState::battlefield_seen`], because a phased-out creature is treated
/// as though it does not exist (CR 702.26b) and cannot be tapped for anything.
#[must_use]
pub fn convoke_sources(state: &GameState, player: PlayerId) -> Vec<ObjectId> {
    untapped_of(state, player, TypeSet::CREATURE)
}

/// Whether a spell `player` casts from `face` has improvise (CR 702.126a):
/// printed on the face, or given to the player's next spell this turn
/// (Archway of Innovation).
#[must_use]
pub fn has_improvise(
    state: &GameState,
    face: &baylee_cards_dsl::FaceDef,
    player: PlayerId,
) -> bool {
    face.improvise
        || state
            .per_turn
            .improvise_next_spell
            .get(usize::from(player.get()))
            .copied()
            .unwrap_or(false)
}

/// The permanents that may be tapped to pay for {1} each of a spell cast
/// from `face`: untapped creatures for convoke (CR 702.51a), untapped
/// artifacts for improvise (CR 702.126a), each permanent once.
#[must_use]
pub fn tap_to_pay_sources(
    state: &GameState,
    face: &baylee_cards_dsl::FaceDef,
    player: PlayerId,
) -> Vec<ObjectId> {
    let mut types = TypeSet::EMPTY;
    if face.convoke {
        types = types.union(TypeSet::CREATURE);
    }
    if has_improvise(state, face, player) {
        types = types.union(TypeSet::ARTIFACT);
    }
    if types.is_empty() {
        return Vec::new();
    }
    untapped_of(state, player, types)
}

/// The permanents a waterbend cost may be paid with (CR 701.67a): untapped
/// artifacts and creatures the caster controls, one `{1}` each — of the
/// waterbend cost alone (CR 701.67b), which is the wizard's bound to set.
#[must_use]
pub fn waterbend_sources(state: &GameState, player: PlayerId) -> Vec<ObjectId> {
    untapped_of(state, player, TypeSet::CREATURE.union(TypeSet::ARTIFACT))
}

fn untapped_of(state: &GameState, player: PlayerId, types: TypeSet) -> Vec<ObjectId> {
    state
        .battlefield_seen()
        .filter(|id| {
            state.object(*id).is_some_and(|o| {
                o.controller == player
                    && o.characteristics().types.intersects(types)
                    && !o.status.contains(crate::object::Status::TAPPED)
            })
        })
        .collect()
}

/// The generic mana this face's keywords can pay for right now: convoke's
/// untapped permanents (CR 702.51) and delve's graveyard (CR 702.66a), one
/// point each.
///
/// One function because two probes ask it and an offer is the difference
/// between their answers. [`can_cast`] decides whether the card appears in
/// `LegalActions` at all; `Engine::cast_options` decides which ways of casting
/// it appear once it has been pressed. Every time those two have counted
/// differently the result has been the same defect in one of its two
/// directions — a card offered and then refused as "no way to cast this
/// spell", or a card never offered that the wizard would have paid for
/// happily. Convoke was fixed by making the count one function and delve was
/// left beside it: Dig Through Time is the pool's only delve card, is
/// `Coverage::Implemented`, and was offered at eight mana or not at all, with
/// a full graveyard doing nothing.
///
/// The *printed* face, because neither keyword can be granted: a copy of a
/// delve spell is not a delve spell.
///
/// Waterbend counts for nothing here, and that is its rule rather than an
/// omission: its taps pay only the waterbend cost itself (CR 701.67b), which
/// no probe of the printed cost includes, and paying it adds at least as much
/// as they take off. Counting them was #229: Spirit Water Revival was offered
/// off `{U}{U}` and one creature.
///
/// `card` is the card being cast, which delve never counts
/// ([`delve_sources`]).
#[must_use]
pub fn keyword_reduction(
    state: &GameState,
    face: &baylee_cards_dsl::FaceDef,
    player: PlayerId,
    card: ObjectId,
) -> u32 {
    let convoke = u32::try_from(tap_to_pay_sources(state, face, player).len()).unwrap_or(u32::MAX);
    let delve = if face.delve {
        u32::try_from(delve_sources(state, player, card).len()).unwrap_or(u32::MAX)
    } else {
        0
    };
    convoke.saturating_add(delve)
}

/// The cards delve (CR 702.66a) may exile to pay for `card`: its caster's
/// graveyard, less the card itself.
///
/// CR 601.2a puts a spell on the stack before any of its costs is paid
/// (601.2h), so a spell cast from a graveyard (a flashback grant) is not
/// there to exile for itself. This engine moves the card at the end of the
/// payment instead, and the delve question offered the whole graveyard: Dig
/// Through Time flashed back by Snapcaster Mage paid {1} of its own cost with
/// itself. One list for the offer's count and the question's options.
#[must_use]
pub fn delve_sources(state: &GameState, player: PlayerId, card: ObjectId) -> Vec<ObjectId> {
    state
        .zones
        .list(ZoneLocation::Graveyard(player))
        .iter()
        .copied()
        .filter(|id| *id != card)
        .collect()
}

/// The generic mana a cost reduction printed on the card itself takes off
/// right now — Surgical Metamorph's "costs {1} less if you weren't the
/// starting player".
///
/// The other half of the pair above, and it went the same way: the wizard
/// applied it when it built the normal-cost option and the offer did not, so
/// the seat the reduction is *for* was never shown the card at the price it
/// would have paid.
#[must_use]
pub fn printed_reduction(
    state: &GameState,
    face: &baylee_cards_dsl::FaceDef,
    player: PlayerId,
    source: ObjectId,
) -> u32 {
    reduction_amount(state, face.cost_reduction, player, source)
}

/// The generic mana `reduction` takes off a cost `player` is paying with
/// `source` — a card being cast or the object whose ability is activated —
/// read now, as the total cost is determined (CR 601.2f, and CR 602.2b for
/// an ability). The taking-off is `ManaCost::with_less_generic`, which
/// touches only the generic component and stops at {0} (CR 118.7a).
#[must_use]
pub fn reduction_amount(
    state: &GameState,
    reduction: Option<baylee_cards_dsl::CostReduction>,
    player: PlayerId,
    source: ObjectId,
) -> u32 {
    match reduction {
        Some(baylee_cards_dsl::CostReduction::If { condition, amount })
            if crate::eval::condition_holds(state, player, source, condition) =>
        {
            amount
        }
        Some(baylee_cards_dsl::CostReduction::NotStartingPlayer(n))
            if player != state.starting_player =>
        {
            n
        }
        Some(baylee_cards_dsl::CostReduction::PerCount { amount, each }) => {
            crate::eval::amount(&amount, state, player, source, None).saturating_mul(each)
        }
        _ => 0,
    }
}

/// CR 903.8's tax on casting `card` from the command zone, in generic mana:
/// `{2}` for each previous cast of *this* commander from there.
///
/// Zero unless the card is in the command zone right now — a commander cast
/// from a hand it was bounced to pays nothing. And per commander rather than
/// per seat: a partner deck taxes its two independently, which is why the
/// count sits on [`crate::state::Commander`] and not beside
/// `GameState::commander_casts`, whose job is Commander's Insight.
///
/// It lives beside [`controls_a_commander`] for the same reason that one
/// does, and it moved here from the wizard after making the same mistake in
/// the other direction: the wizard charged the tax and the legality probe
/// did not, so a commander that had been cast once was *offered* for its
/// printed cost and then refused mid-wizard. A tax the counter records and
/// nothing charges is not a tax, and one only half the engine charges is
/// worse than none.
#[must_use]
pub fn commander_tax(state: &GameState, player: PlayerId, card: ObjectId) -> u32 {
    if state.object(card).map(|o| o.zone) != Some(Zone::Command) {
        return 0;
    }
    state
        .commanders
        .get(player.get() as usize)
        .and_then(|cs| cs.iter().find(|c| c.object == card))
        .map_or(0, |c| c.casts.saturating_mul(2))
}

/// Generic adjustments combine before clamping: increases first (CR 601.2f).
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct SpellAdjustment {
    pub increase: u32,
    pub eligible_increase: u32,
    pub reduction: u32,
    pub mana_increase: ManaCost,
}
impl SpellAdjustment {
    pub(crate) fn allowance(
        self,
        state: &GameState,
        what: SpendFor,
        additional_reduction: u32,
    ) -> SourceAllowance {
        base_allowance(state, what).adjusted(
            self.eligible_increase,
            self.increase.saturating_sub(self.eligible_increase),
            self.reduction.saturating_add(additional_reduction),
        )
    }

    pub(crate) fn price(self, raw: ManaCost, printed_reduction: u32) -> ManaCost {
        raw.combine(&self.mana_increase)
            .with_more_generic(self.increase)
            .with_less_generic(self.reduction.saturating_add(printed_reduction))
    }
}

fn has_spell_adjustments(state: &GameState) -> bool {
    state.effects.may_change_spell_costs()
        && state.effects.iter().any(|fx| {
            matches!(
                fx.modifier,
                baylee_cards_dsl::Modifier::SpellsCostMore(_)
                    | baylee_cards_dsl::Modifier::SpellsCostMorePerControlled(_)
                    | baylee_cards_dsl::Modifier::SpellsCostMoreMana(_)
                    | baylee_cards_dsl::Modifier::SpellsCostLess(_)
            )
        })
}

/// Match the projected stack characteristics, including alternative faces.
pub(crate) fn spell_adjustment(
    state: &GameState,
    player: PlayerId,
    object: &crate::object::GameObject,
) -> SpellAdjustment {
    let mut adjustment = SpellAdjustment {
        increase: commander_tax(state, player, object.id),
        eligible_increase: 0,
        reduction: 0,
        mana_increase: ManaCost::ZERO,
    };
    if !has_spell_adjustments(state) {
        return adjustment;
    }
    let mut spell = object.clone();
    spell.zone = Zone::Stack;
    spell.controller = player;
    spell.base_controller = player;
    spell.cache.clear();
    let projected = crate::layers::recompute(state, &spell);
    spell.base = std::sync::Arc::new(projected.characteristics);
    for effect in state.effects.iter() {
        match effect.modifier {
            baylee_cards_dsl::Modifier::SpellsCostMore(amount) => {
                if crate::effects::applies_to(state, effect, &spell) {
                    adjustment.increase = adjustment.increase.saturating_add(amount);
                    if super::effect_is_un(state, effect.source) {
                        adjustment.eligible_increase =
                            adjustment.eligible_increase.saturating_add(amount);
                    }
                }
            }
            baylee_cards_dsl::Modifier::SpellsCostMorePerControlled(filter)
                if crate::effects::applies_to(state, effect, &spell) =>
            {
                let context = crate::eval::live_context(state, effect.source.unwrap_or(object.id));
                let count = state
                    .battlefield_seen()
                    .filter(|&id| {
                        state.object(id).is_some_and(|permanent| {
                            permanent.controller == player
                                && crate::eval::matches_with_context(
                                    filter, state, permanent, player, context,
                                )
                        })
                    })
                    .count();
                let count = u32::try_from(count).unwrap_or(u32::MAX);
                adjustment.increase = adjustment.increase.saturating_add(count);
            }
            baylee_cards_dsl::Modifier::SpellsCostMoreMana(extra)
                if crate::effects::applies_to(state, effect, &spell) =>
            {
                adjustment.mana_increase = adjustment.mana_increase.combine(&extra);
            }
            baylee_cards_dsl::Modifier::SpellsCostLess(amount)
                if crate::effects::applies_to(state, effect, &spell) =>
            {
                adjustment.reduction = adjustment.reduction.saturating_add(amount);
            }
            _ => {}
        }
    }
    adjustment
}

pub(crate) fn face_adjustment(
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
    def: &baylee_cards_dsl::CardDef,
    face: usize,
    form: Option<SpellForm>,
) -> SpellAdjustment {
    if !has_spell_adjustments(state) {
        return SpellAdjustment {
            increase: commander_tax(state, player, card),
            eligible_increase: 0,
            reduction: 0,
            mana_increase: ManaCost::ZERO,
        };
    }
    let Some(object) = state.object(card) else {
        return SpellAdjustment::default();
    };
    let mut spell = object.clone();
    spell.base = std::sync::Arc::new(crate::object::Characteristics::from_face(
        def,
        face,
        object.base.name,
    ));
    spell.cache.clear();
    spell.face_index = u8::try_from(face).unwrap_or(0);
    if let Some(form) = form {
        spell = form.project(&spell);
    }
    spell_adjustment(state, player, &spell)
}

/// The public view still reports positive taxes separately from printed costs.
#[cfg(test)]
pub(crate) fn spell_increase(
    state: &GameState,
    player: PlayerId,
    object: &crate::object::GameObject,
) -> u32 {
    let adj = spell_adjustment(state, player, object);
    adj.increase.saturating_add(adj.mana_increase.cmc())
}
pub(crate) fn face_increase(
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
    def: &baylee_cards_dsl::CardDef,
    face: usize,
    form: Option<SpellForm>,
) -> u32 {
    let adj = face_adjustment(state, player, card, def, face, form);
    adj.increase.saturating_add(adj.mana_increase.cmc())
}

/// Additional generic mana for activating an ability of this object. The
/// filter's normal zone boundary deliberately excludes white enchantment
/// *cards* cycling in hand; Gloom names enchantments, i.e. permanents.
pub(crate) fn activation_increase(state: &GameState, source: ObjectId) -> u32 {
    if !state.effects.may_change_or_forbid_abilities() {
        return 0;
    }
    let Some(object) = state.object(source) else {
        return 0;
    };
    state
        .effects
        .iter()
        .filter_map(|fx| match fx.modifier {
            baylee_cards_dsl::Modifier::AbilitiesCostMore(n)
                if crate::effects::applies_to(state, fx, object) =>
            {
                Some(n)
            }
            _ => None,
        })
        .fold(0, u32::saturating_add)
}

/// One activation may pay one ordinary mana cost; additional costs stay exact.
pub(crate) fn north_star_effect(
    state: &GameState,
    player: PlayerId,
) -> Option<baylee_core::ids::EffectId> {
    if !state.effects.may_change_mana_spending() {
        return None;
    }
    state
        .effects
        .iter()
        .find(|effect| {
            effect.controller == player
                && effect.modifier == baylee_cards_dsl::Modifier::OneSpellManaAnyType
        })
        .map(|effect| effect.id)
}

pub(crate) fn north_star_face_allowed(
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
    face: usize,
) -> bool {
    state.object(card).is_some_and(|object| {
        object.zone != Zone::Graveyard
            || !graveyard_face_permissions(state, player, card, face).is_empty()
    })
}

/// Price already includes increases, reductions and additional costs.
pub(crate) fn normal_spell_affordable(
    state: &GameState,
    player: PlayerId,
    what: SpendFor,
    pool: &ManaPool,
    cost: &ManaCost,
    component: &ManaCost,
    allowance: SourceAllowance,
) -> bool {
    if source_affordable(state, player, what, pool, cost, &[], Some(allowance)) {
        return true;
    }
    if north_star_effect(state, player).is_none() {
        return false;
    }
    component_affordable(state, player, what, pool, cost, component, &[], allowance)
}

#[allow(clippy::too_many_arguments)] // component conversion preserves the same explicit source scope
pub(crate) fn component_affordable(
    state: &GameState,
    player: PlayerId,
    what: SpendFor,
    pool: &ManaPool,
    cost: &ManaCost,
    component: &ManaCost,
    settled: &[bool],
    allowance: SourceAllowance,
) -> bool {
    let symbols = cost.phyrexian_count();
    let fixed = u32::try_from(settled.len()).unwrap_or(u32::MAX);
    if symbols > 8 || fixed > symbols {
        return false;
    }
    let mask = settled
        .iter()
        .enumerate()
        .fold(0_u32, |mask, (index, life)| {
            mask | (u32::from(*life) << index)
        });
    (0..(1_u32 << (symbols - fixed))).any(|rest| {
        let life = mask | (rest << fixed);
        let payment = cost
            .with_phyrexian_settled(life)
            .with_any_type_component(&cost.component_after_phyrexian(component, life));
        state.can_pay_life(
            player,
            2 * i32::try_from(life.count_ones()).unwrap_or(i32::MAX),
        ) && source_plan(
            state,
            what,
            pool,
            &payment,
            payment_spending(state, player, what),
            None,
            allowance,
        )
        .is_some()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn generic_adjustments_match_integer_reference_without_reducing_colored_symbols() {
        for base in 0..24u32 {
            for increase in 0..12u32 {
                for reduction in 0..48u32 {
                    let cost = ManaCost::parse("{U}{B}").with_more_generic(base);
                    let adjusted = SpellAdjustment {
                        increase,
                        eligible_increase: 0,
                        mana_increase: ManaCost::ZERO,
                        reduction,
                    }
                    .price(cost, 0);
                    assert_eq!(
                        adjusted.generic_total(),
                        (base + increase).saturating_sub(reduction)
                    );
                    assert_eq!(adjusted.colors(), cost.colors());
                    assert_eq!(adjusted.cmc(), adjusted.generic_total() + 2);
                }
            }
        }
    }
}
