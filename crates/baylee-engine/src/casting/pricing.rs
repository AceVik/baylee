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
#[doc(hidden)]
pub fn spell_mana_spending(state: &GameState, player: PlayerId, card: ObjectId) -> ManaSpending {
    let mut any_type = false;
    let mut any_color = false;
    if let Some(object) = state.object(card) {
        for permission in &state.play_permissions {
            if permission.grants_to(state, player, card, object.version) {
                any_type |= permission.mana_any_type;
                any_color |= permission.window.mana_any_color();
            }
        }
    }
    if any_type {
        ManaSpending::ANY_TYPE
    } else if any_color {
        let mut spending = mana_spending(state, player);
        for actual in ManaColor::ALL {
            for required in ManaColor::ALL
                .into_iter()
                .filter(|color| *color != ManaColor::Colorless)
            {
                spending.allow(actual, required);
            }
        }
        spending
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

/// The colours each of `taps` pays with for a spell cast from `face`: a
/// creature's colours when the face has convoke, each tap one coloured mana
/// of one of them (CR 702.51a); nothing coloured for an artifact tapped
/// for improvise, which pays generic mana alone (CR 702.126a).
#[must_use]
pub fn tap_colors(
    state: &GameState,
    face: &baylee_cards_dsl::FaceDef,
    taps: &[ObjectId],
) -> Vec<baylee_core::color::ColorSet> {
    taps.iter()
        .map(|id| {
            state
                .object(*id)
                .map(crate::object::GameObject::characteristics)
                .filter(|c| face.convoke && c.types.contains(TypeSet::CREATURE))
                .map_or(baylee_core::color::ColorSet::EMPTY, |c| c.colors)
        })
        .collect()
}

/// `cost` as the probes that ask whether a cast can be paid price it: the
/// permanents convoke may tap pay the coloured symbols their colours can
/// (`ManaCost::convoked_colored`, CR 702.51a), and `generic` — the
/// [`keyword_reduction`] count with whatever other generic reduction the
/// caller folds into it — comes off the generic part, less the taps that
/// paid a colour, since the count had them paying generic.
///
/// Until #230 the taps were counted as generic mana only, so Chord of
/// Calling (`{X}{G}{G}{G}`) was never castable by its green creatures and
/// was offered only when the lands alone paid its colour.
#[must_use]
pub fn keyword_priced(
    state: &GameState,
    face: &baylee_cards_dsl::FaceDef,
    player: PlayerId,
    cost: &ManaCost,
    generic: u32,
) -> ManaCost {
    let taps = tap_colors(state, face, &tap_to_pay_sources(state, face, player));
    let (cost, colored) = cost.convoked_colored(&taps);
    cost.with_less_generic(generic.saturating_sub(colored))
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

/// Full mana-cost reduction (including colored and colorless symbols per
/// CR 118.7a–g) contributed by a printed [`baylee_cards_dsl::CostReduction`].
///
/// Used by Power-up (`CostReduction::OwnManaCostIfEnteredThisTurn`), which
/// reduces the activation's mana cost by the permanent's own printed/copiable
/// mana cost if it entered the battlefield this turn.
#[must_use]
pub fn mana_cost_reduction(
    state: &GameState,
    reduction: Option<baylee_cards_dsl::CostReduction>,
    source: ObjectId,
) -> Option<ManaCost> {
    match reduction {
        Some(baylee_cards_dsl::CostReduction::OwnManaCostIfEnteredThisTurn) => {
            let object = state.object(source)?;
            if object.zone != Zone::Battlefield
                || !state.per_turn.entered_battlefield.contains(&source)
            {
                return None;
            }
            let own = object.characteristics().mana_cost.with_x(0);
            (own != ManaCost::ZERO).then_some(own)
        }
        _ => None,
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

/// Generic and colored adjustments combine before clamping: increases first (CR 601.2f).
#[derive(Clone, Copy, Debug, Default)]
#[doc(hidden)]
pub struct SpellAdjustment {
    pub increase: u32,
    pub eligible_increase: u32,
    pub reduction: u32,
    pub mana_increase: ManaCost,
    pub mana_reduction: ManaCost,
    pub colored_only_reduction: ManaCost,
    /// What comes off this spell's buyback cost alone (`Modifier::BuybackCostsLess`).
    pub buyback_reduction: u32,
    /// Net generic delta per distinct target from active
    /// `Modifier::AdjustSpellCostByCast { metric: DistinctTargets, .. }`
    /// effects (negative reduces generic mana, positive taxes generic mana).
    pub per_distinct_target_delta: i32,
    /// Net generic delta per announced `X` from active
    /// `Modifier::AdjustSpellCostByCast { metric: XPaid, .. }` effects.
    pub per_x_paid_delta: i32,
    /// Whether at least one active `Modifier::ReduceCostIfTargets { abilities: false, .. }`
    /// applies to this spell (signaling `preview_cast_metric_bounds` to inspect
    /// legal target clauses for achievable/guaranteed target-conditional reductions).
    pub has_target_conditional_reduction: bool,
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
            .with_less_mana(&self.colored_only_reduction, true)
            .with_less_mana(&self.mana_reduction, false)
            .with_less_generic(self.reduction.saturating_add(printed_reduction))
    }

    /// Returns a copy adjusted for castability/affordability previews using
    /// `preview` (`min_increase` for guaranteed target taxes and `max_reduction`
    /// / `max_mana_reduction` for achievable target/`XPaid` reductions). Never
    /// use for `CastModeDesc::cost` or `FrozenReduction`.
    #[must_use]
    pub(crate) fn for_preview(mut self, preview: PreviewDynamicDelta) -> Self {
        self.increase = self.increase.saturating_add(preview.min_increase);
        self.reduction = self.reduction.saturating_add(preview.max_reduction);
        self.mana_reduction = self.mana_reduction.combine(&preview.max_mana_reduction);
        self
    }
}

/// Castability preview bounds for cast-time `AdjustSpellCostByCast` metrics
/// (`DistinctTargets` and `XPaid`) and `ReduceCostIfTargets` modifiers before
/// targets and `X` are announced.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct PreviewDynamicDelta {
    /// Guaranteed minimum generic increase from mandatory targets.
    pub min_increase: u32,
    /// Guaranteed minimum generic reduction from mandatory targets.
    pub min_reduction: u32,
    /// Maximum achievable generic reduction from legal target / `XPaid` choices.
    pub max_reduction: u32,
    /// Guaranteed minimum mana reduction when every legal target selection
    /// satisfies an active `Modifier::ReduceCostIfTargets`.
    pub min_mana_reduction: ManaCost,
    /// Optimistic maximum mana reduction from active
    /// `Modifier::ReduceCostIfTargets` modifiers that have at least one legal
    /// target matching `target_filter`.
    pub max_mana_reduction: ManaCost,
}

/// Which target clauses a pre-target cast preview examines.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum PreviewCastKind {
    #[default]
    Face,
    Kicked,
    Bestow,
    Mutate,
    Fused,
}

const MAX_SPELL_COST_DEPTH: u8 = 8;

thread_local! {
    static SPELL_COST_DEPTH: std::cell::Cell<u8> = const { std::cell::Cell::new(0) };
}

struct SpellCostDepthGuard;

impl SpellCostDepthGuard {
    fn enter() -> Option<Self> {
        SPELL_COST_DEPTH.with(|depth| {
            let current = depth.get();
            if current >= MAX_SPELL_COST_DEPTH {
                return None;
            }
            depth.set(current + 1);
            Some(Self)
        })
    }
}

impl Drop for SpellCostDepthGuard {
    fn drop(&mut self) {
        SPELL_COST_DEPTH.with(|depth| {
            depth.set(depth.get().saturating_sub(1));
        });
    }
}

const fn is_spell_cost_modifier(modifier: baylee_cards_dsl::Modifier) -> bool {
    matches!(
        modifier,
        baylee_cards_dsl::Modifier::SpellsCostMore(_)
            | baylee_cards_dsl::Modifier::SpellsCostMorePerControlled(_)
            | baylee_cards_dsl::Modifier::SpellsCostMoreMana(_)
            | baylee_cards_dsl::Modifier::SpellsCostLess(_)
            | baylee_cards_dsl::Modifier::SpellsCostLessMana { .. }
            | baylee_cards_dsl::Modifier::SpellsCostLessByAmount(_)
            | baylee_cards_dsl::Modifier::SpellsCostLessPerCounter(_)
            | baylee_cards_dsl::Modifier::BuybackCostsLess(_)
            | baylee_cards_dsl::Modifier::AdjustSpellCostByCast { .. }
            | baylee_cards_dsl::Modifier::ReduceCostIfTargets {
                abilities: false,
                ..
            }
            | baylee_cards_dsl::Modifier::AdjustSpellCostByAmount { .. }
            | baylee_cards_dsl::Modifier::FirstSpellEachTurnCostAdjust { .. }
    )
}

#[must_use]
pub(crate) fn card_has_self_spell_cost_statics(def: &baylee_cards_dsl::CardDef) -> bool {
    (0..def.faces.len()).any(|face| {
        def.abilities_for_face(face).iter().any(|ability| {
            matches!(
                ability,
                baylee_cards_dsl::AbilityDef::Static(sa)
                    if Zone::Stack.matches_ref(sa.source_zone) && is_spell_cost_modifier(sa.modifier)
            )
        })
    })
}

fn has_spell_adjustments(state: &GameState) -> bool {
    state.effects.may_change_spell_costs()
        && state
            .effects
            .iter()
            .any(|fx| is_spell_cost_modifier(fx.modifier))
}

/// Match the projected stack characteristics, including alternative faces.
#[doc(hidden)]
pub fn projected_spell(
    state: &GameState,
    player: PlayerId,
    object: &crate::object::GameObject,
) -> crate::object::GameObject {
    let mut spell = object.clone();
    if object.zone != Zone::Stack {
        spell.cast_history.set_from_hand(object.zone == Zone::Hand);
        spell
            .cast_history
            .set_from_graveyard(object.zone == Zone::Graveyard);
    }
    spell.zone = Zone::Stack;
    spell.kind = crate::object::ObjectKind::Spell;
    spell.controller = player;
    spell.base_controller = player;
    spell.cache.clear();
    let projected = crate::layers::recompute(state, &spell);
    spell.base = std::sync::Arc::new(projected.characteristics);
    spell
}

pub(crate) fn distinct_target_count(spell: &crate::object::GameObject) -> u32 {
    let mut objects: smallvec::SmallVec<[ObjectId; 4]> = smallvec::SmallVec::new();
    for &id in spell
        .targets
        .iter()
        .chain(spell.second_targets().iter())
        .chain(
            spell
                .extra_target_instances()
                .iter()
                .flat_map(|inst| inst.targets.iter()),
        )
    {
        if !objects.contains(&id) {
            objects.push(id);
        }
    }
    let mut players = spell.target_req.map_or(spell.target_players, |req| {
        crate::eval::targeted_players(spell, &req.spec)
    });
    for p in spell.second_target_players().iter() {
        players.insert(p);
    }
    for inst in spell.extra_target_instances() {
        for p in inst.players.iter() {
            players.insert(p);
        }
    }
    u32::try_from(objects.len() + players.len()).unwrap_or(u32::MAX)
}

fn spell_cast_metric_units(
    state: &GameState,
    player: PlayerId,
    spell: &crate::object::GameObject,
    source: Option<ObjectId>,
    metric: baylee_cards_dsl::SpellCastMetric,
) -> u32 {
    use baylee_cards_dsl::SpellCastMetric;
    match metric {
        SpellCastMetric::DistinctTargets => distinct_target_count(spell),
        SpellCastMetric::XPaid => spell.x_value,
        SpellCastMetric::CardsDiscardedThisTurn => state.per_turn.cards_discarded_by(player),
        SpellCastMetric::PermanentsSacrificedThisTurn(sac_filter) => {
            let context = crate::eval::live_context(state, source.unwrap_or(spell.id));
            let count = state
                .per_turn
                .sacrificed_permanents
                .items
                .iter()
                .filter(|rec| {
                    rec.controller == player
                        && crate::eval::matches_with_context(
                            sac_filter,
                            state,
                            &rec.snapshot,
                            player,
                            context,
                        )
                })
                .count();
            u32::try_from(count).unwrap_or(u32::MAX)
        }
        SpellCastMetric::LifeGainedThisTurn => state
            .per_turn
            .life_gained_amount
            .get(usize::from(player.get()))
            .copied()
            .unwrap_or(0),
        SpellCastMetric::LifeLostByOpponentsThisTurn => state
            .players
            .iter()
            .filter(|p| !state.has_left(p.id) && state.is_opponent(p.id, player))
            .fold(0_u32, |total, p| {
                total.saturating_add(
                    state
                        .per_turn
                        .life_lost_amount
                        .get(usize::from(p.id.get()))
                        .copied()
                        .unwrap_or(0),
                )
            }),
        SpellCastMetric::SpellsCastThisTurn => state.per_turn.spells_cast_by(player),
        SpellCastMetric::CardsDrawnThisTurn => state
            .per_turn
            .draws
            .get(usize::from(player.get()))
            .copied()
            .unwrap_or(0),
    }
}

fn spell_targets_matching(
    state: &GameState,
    spell: &crate::object::GameObject,
    target_filter: &baylee_cards_dsl::Filter,
    controller: PlayerId,
    context: crate::text_changes::RuleContext,
) -> bool {
    spell
        .targets
        .iter()
        .chain(spell.second_targets().iter())
        .chain(
            spell
                .extra_target_instances()
                .iter()
                .flat_map(|inst| inst.targets.iter()),
        )
        .any(|&target_id| {
            state.object(target_id).is_some_and(|obj| {
                crate::eval::matches_with_context(target_filter, state, obj, controller, context)
            })
        })
}

fn spell_cost_effect_applies(
    state: &GameState,
    effect: &crate::effects::ContinuousEffect,
    who: baylee_cards_dsl::PlayerRel,
    player: PlayerId,
    spell: &crate::object::GameObject,
) -> bool {
    let stack_self_only = matches!(
        effect.filter,
        crate::effects::EffectFilter::Dsl(baylee_cards_dsl::Filter::This)
    ) && effect
        .source
        .and_then(|s| state.object(s))
        .is_some_and(|o| o.zone == Zone::Stack);
    super::player_locks::affects_player(state, effect, who, player)
        && if stack_self_only {
            effect.source == Some(spell.id)
        } else {
            matches!(
                effect.filter,
                crate::effects::EffectFilter::Dsl(
                    baylee_cards_dsl::Filter::Any | baylee_cards_dsl::Filter::This
                ) | crate::effects::EffectFilter::DslControlledBy(_, _)
            ) || crate::effects::applies_to(state, effect, spell)
        }
}

#[allow(clippy::too_many_arguments)]
fn apply_first_spell_cost_adjust(
    state: &GameState,
    player: PlayerId,
    spell: &crate::object::GameObject,
    controller: PlayerId,
    source: Option<ObjectId>,
    context: crate::text_changes::RuleContext,
    who: baylee_cards_dsl::PlayerRel,
    filter: &'static baylee_cards_dsl::Filter,
    cost: ManaCost,
    increase: bool,
    colored_only: bool,
    adjustment: &mut SpellAdjustment,
) {
    if cost == ManaCost::ZERO
        || crate::eval::players_for_source(who, state, controller, source)
            .is_some_and(|players| !players.contains(&player))
        || !crate::eval::matches_with_context(filter, state, spell, controller, context)
    {
        return;
    }
    let already_cast = if state.per_turn.cast_spells_this_turn.tracked {
        state
            .per_turn
            .cast_spells_this_turn
            .items
            .iter()
            .any(|rec| {
                rec.controller == player
                    && !(rec.source.object == spell.id && rec.source.version == spell.version)
                    && crate::eval::matches_with_context(
                        filter,
                        state,
                        &rec.snapshot,
                        controller,
                        context,
                    )
            })
    } else {
        *filter == baylee_cards_dsl::Filter::Any && state.per_turn.spells_cast_by(player) > 0
    };
    if already_cast {
        return;
    }
    if increase {
        if cost.generic_total() == cost.cmc() {
            adjustment.increase = adjustment.increase.saturating_add(cost.generic_total());
            if super::effect_is_un(state, source) {
                adjustment.eligible_increase = adjustment
                    .eligible_increase
                    .saturating_add(cost.generic_total());
            }
        } else {
            adjustment.mana_increase = adjustment.mana_increase.combine(&cost);
        }
    } else if colored_only {
        adjustment.colored_only_reduction = adjustment.colored_only_reduction.combine(&cost);
    } else if cost.generic_total() == cost.cmc() {
        adjustment.reduction = adjustment.reduction.saturating_add(cost.generic_total());
    } else {
        adjustment.mana_reduction = adjustment.mana_reduction.combine(&cost);
    }
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn apply_dynamic_spell_cost_modifier(
    state: &GameState,
    player: PlayerId,
    spell: &crate::object::GameObject,
    controller: PlayerId,
    source: Option<ObjectId>,
    context: crate::text_changes::RuleContext,
    modifier: baylee_cards_dsl::Modifier,
    adjustment: &mut SpellAdjustment,
) {
    match modifier {
        baylee_cards_dsl::Modifier::AdjustSpellCostByCast {
            who,
            filter,
            delta_per_unit,
            metric,
        } => {
            if delta_per_unit == 0
                || !crate::eval::players_for_source(who, state, controller, source)
                    .is_some_and(|players| players.contains(&player))
                || !crate::eval::matches_with_context(filter, state, spell, controller, context)
            {
                return;
            }
            match metric {
                baylee_cards_dsl::SpellCastMetric::DistinctTargets => {
                    adjustment.per_distinct_target_delta = adjustment
                        .per_distinct_target_delta
                        .saturating_add(i32::from(delta_per_unit));
                }
                baylee_cards_dsl::SpellCastMetric::XPaid => {
                    adjustment.per_x_paid_delta = adjustment
                        .per_x_paid_delta
                        .saturating_add(i32::from(delta_per_unit));
                }
                _ => {}
            }
            let units = spell_cast_metric_units(state, player, spell, source, metric);
            let magnitude = u32::from(delta_per_unit.unsigned_abs()).saturating_mul(units);
            if delta_per_unit > 0 {
                adjustment.increase = adjustment.increase.saturating_add(magnitude);
            } else {
                adjustment.reduction = adjustment.reduction.saturating_add(magnitude);
            }
        }
        baylee_cards_dsl::Modifier::ReduceCostIfTargets {
            who,
            spell_filter,
            target_filter,
            reduction,
            abilities,
        } => {
            if abilities
                || reduction == ManaCost::ZERO
                || !crate::eval::players_for_source(who, state, controller, source)
                    .is_some_and(|players| players.contains(&player))
                || !crate::eval::matches_with_context(
                    spell_filter,
                    state,
                    spell,
                    controller,
                    context,
                )
            {
                return;
            }
            adjustment.has_target_conditional_reduction = true;
            if spell_targets_matching(state, spell, target_filter, controller, context) {
                adjustment.mana_reduction = adjustment.mana_reduction.combine(&reduction);
            }
        }
        baylee_cards_dsl::Modifier::AdjustSpellCostByAmount {
            who,
            filter,
            increase,
            amount,
        } => {
            if !crate::eval::players_for_source(who, state, controller, source)
                .is_some_and(|players| players.contains(&player))
                || !crate::eval::matches_with_context(filter, state, spell, controller, context)
            {
                return;
            }
            let Some(_guard) = SpellCostDepthGuard::enter() else {
                return;
            };
            let value = crate::eval::amount_with_context(
                amount,
                state,
                controller,
                context,
                Some(spell.x_value),
            );
            if increase {
                adjustment.increase = adjustment.increase.saturating_add(value);
                if super::effect_is_un(state, source) {
                    adjustment.eligible_increase =
                        adjustment.eligible_increase.saturating_add(value);
                }
            } else {
                adjustment.reduction = adjustment.reduction.saturating_add(value);
            }
        }
        baylee_cards_dsl::Modifier::FirstSpellEachTurnCostAdjust {
            who,
            filter,
            cost,
            increase,
            colored_only,
        } => apply_first_spell_cost_adjust(
            state,
            player,
            spell,
            controller,
            source,
            context,
            who,
            filter,
            cost,
            increase,
            colored_only,
            adjustment,
        ),
        _ => {}
    }
}

#[allow(clippy::too_many_arguments)]
fn apply_spell_cost_modifier(
    state: &GameState,
    player: PlayerId,
    spell: &crate::object::GameObject,
    controller: PlayerId,
    source: Option<ObjectId>,
    text: crate::text_changes::TextChangeMap,
    chosen_subtype: Option<baylee_core::ids::SubtypeId>,
    modifier: baylee_cards_dsl::Modifier,
    adjustment: &mut SpellAdjustment,
) {
    match modifier {
        baylee_cards_dsl::Modifier::SpellsCostMore(amount) => {
            adjustment.increase = adjustment.increase.saturating_add(amount);
            if super::effect_is_un(state, source) {
                adjustment.eligible_increase = adjustment.eligible_increase.saturating_add(amount);
            }
        }
        baylee_cards_dsl::Modifier::SpellsCostMorePerControlled(filter) => {
            let context = crate::eval::live_context(state, source.unwrap_or(spell.id));
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
        baylee_cards_dsl::Modifier::SpellsCostMoreMana(extra) => {
            adjustment.mana_increase = adjustment.mana_increase.combine(&extra);
        }
        baylee_cards_dsl::Modifier::SpellsCostLess(amount) => {
            adjustment.reduction = adjustment.reduction.saturating_add(amount);
        }
        baylee_cards_dsl::Modifier::SpellsCostLessMana { cost, colored_only } => {
            if colored_only {
                adjustment.colored_only_reduction =
                    adjustment.colored_only_reduction.combine(&cost);
            } else {
                adjustment.mana_reduction = adjustment.mana_reduction.combine(&cost);
            }
        }
        baylee_cards_dsl::Modifier::SpellsCostLessByAmount(amount) => {
            let value =
                crate::eval::amount(&amount, state, controller, source.unwrap_or(spell.id), None);
            adjustment.reduction = adjustment.reduction.saturating_add(value);
        }
        baylee_cards_dsl::Modifier::BuybackCostsLess(amount) => {
            adjustment.buyback_reduction = adjustment.buyback_reduction.saturating_add(amount);
        }
        baylee_cards_dsl::Modifier::SpellsCostLessPerCounter(kind) => {
            let counters = source
                .and_then(|source| state.object(source))
                .map_or(0, |source| u32::from(source.counters.get(kind)));
            adjustment.reduction = adjustment.reduction.saturating_add(counters);
        }
        baylee_cards_dsl::Modifier::AdjustSpellCostByCast { .. }
        | baylee_cards_dsl::Modifier::ReduceCostIfTargets { .. }
        | baylee_cards_dsl::Modifier::AdjustSpellCostByAmount { .. }
        | baylee_cards_dsl::Modifier::FirstSpellEachTurnCostAdjust { .. } => {
            let context = crate::text_changes::RuleContext {
                source: source.unwrap_or(spell.id),
                text,
                resolved_name: None,
                resolved_subtype: chosen_subtype,
                reference: None,
            };
            apply_dynamic_spell_cost_modifier(
                state, player, spell, controller, source, context, modifier, adjustment,
            );
        }
        _ => {}
    }
}

#[doc(hidden)]
pub fn spell_adjustment(
    state: &GameState,
    player: PlayerId,
    object: &crate::object::GameObject,
) -> SpellAdjustment {
    spell_adjustment_with_def(state, player, object, None)
}

#[doc(hidden)]
pub fn spell_adjustment_with_def(
    state: &GameState,
    player: PlayerId,
    object: &crate::object::GameObject,
    def: Option<&baylee_cards_dsl::CardDef>,
) -> SpellAdjustment {
    let mut adjustment = SpellAdjustment {
        increase: commander_tax(state, player, object.id),
        eligible_increase: 0,
        reduction: state
            .per_turn
            .next_spell_cost_reduction
            .get(usize::from(player.get()))
            .copied()
            .unwrap_or(0),
        mana_increase: ManaCost::ZERO,
        mana_reduction: ManaCost::ZERO,
        colored_only_reduction: ManaCost::ZERO,
        buyback_reduction: 0,
        per_distinct_target_delta: 0,
        per_x_paid_delta: 0,
        has_target_conditional_reduction: false,
    };
    let Some(_guard) = SpellCostDepthGuard::enter() else {
        return adjustment;
    };
    let self_statics = def.is_some_and(card_has_self_spell_cost_statics)
        && !object.status.contains(crate::object::Status::FACE_DOWN);
    if !has_spell_adjustments(state) && !self_statics {
        return adjustment;
    }
    let spell = projected_spell(state, player, object);
    for effect in state.effects.iter() {
        if !is_spell_cost_modifier(effect.modifier) {
            continue;
        }
        let applies = match effect.modifier {
            baylee_cards_dsl::Modifier::AdjustSpellCostByCast { who, .. }
            | baylee_cards_dsl::Modifier::ReduceCostIfTargets {
                who,
                abilities: false,
                ..
            }
            | baylee_cards_dsl::Modifier::AdjustSpellCostByAmount { who, .. }
            | baylee_cards_dsl::Modifier::FirstSpellEachTurnCostAdjust { who, .. } => {
                spell_cost_effect_applies(state, effect, who, player, &spell)
            }
            _ => crate::effects::applies_to(state, effect, &spell),
        };
        if applies {
            apply_spell_cost_modifier(
                state,
                player,
                &spell,
                effect.controller,
                effect.source,
                state.effect_text(effect),
                effect.filter.chosen_subtype(),
                effect.modifier,
                &mut adjustment,
            );
        }
    }
    if self_statics
        && spell.characteristics().abilities_lost.is_none()
        && !spell.characteristics().rules_text_lost
        && let Some(def) = def
    {
        let face = usize::from(spell.face_index).min(def.faces.len().saturating_sub(1));
        for ability in def.abilities_for_face(face) {
            let baylee_cards_dsl::AbilityDef::Static(sa) = ability else {
                continue;
            };
            if !Zone::Stack.matches_ref(sa.source_zone) || !is_spell_cost_modifier(sa.modifier) {
                continue;
            }
            // Avoid double-counting if this object is already in a zone whose
            // static registration put the same modifier into `state.effects`.
            let already_registered = state.effects.iter().any(|fx| {
                fx.source == Some(spell.id)
                    && fx.origin == crate::effects::EffectOrigin::Static
                    && fx.modifier == sa.modifier
            });
            if already_registered {
                continue;
            }
            let context = crate::eval::live_context(state, spell.id);
            if sa.condition.is_some_and(|cond| {
                !crate::eval::condition_holds_with_context(state, player, context, cond)
            }) {
                continue;
            }
            if crate::eval::matches_with_context(&sa.filter, state, &spell, player, context) {
                apply_spell_cost_modifier(
                    state,
                    player,
                    &spell,
                    player,
                    Some(spell.id),
                    context.text,
                    None,
                    sa.modifier,
                    &mut adjustment,
                );
            }
        }
    }
    adjustment
}

#[doc(hidden)]
pub fn face_adjustment(
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
    def: &baylee_cards_dsl::CardDef,
    face: usize,
    form: Option<SpellForm>,
) -> SpellAdjustment {
    let self_statics =
        !matches!(form, Some(SpellForm::Disguise)) && card_has_self_spell_cost_statics(def);
    if !has_spell_adjustments(state) && !self_statics {
        return SpellAdjustment {
            increase: commander_tax(state, player, card),
            eligible_increase: 0,
            reduction: state
                .per_turn
                .next_spell_cost_reduction
                .get(usize::from(player.get()))
                .copied()
                .unwrap_or(0),
            mana_increase: ManaCost::ZERO,
            mana_reduction: ManaCost::ZERO,
            colored_only_reduction: ManaCost::ZERO,
            buyback_reduction: 0,
            per_distinct_target_delta: 0,
            per_x_paid_delta: 0,
            has_target_conditional_reduction: false,
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
    spell_adjustment_with_def(state, player, &spell, Some(def))
}

struct ClauseTargets {
    min: u32,
    max: u32,
    objects: Vec<ObjectId>,
    players: Vec<PlayerId>,
}

fn clause_preview_targets(
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
    def: &'static baylee_cards_dsl::CardDef,
    face: usize,
    req: baylee_cards_dsl::TargetReq,
    wall_only: bool,
) -> ClauseTargets {
    let objs = crate::eval::spell_target_options_for(
        &req.spec,
        state,
        player,
        card,
        Some((def, face)),
        None,
        wall_only,
    );
    let pls = crate::eval::target_player_options_from(
        state,
        &req.spec,
        player,
        crate::eval::live_context(state, card),
    );
    let (min_c, max_c) = match req.constraint() {
        Some(baylee_cards_dsl::TargetConstraint::OnePerPlayer) => {
            let seats = crate::eval::one_per_player_seats(state, &req.spec, player);
            let eligible = u32::try_from(
                seats
                    .iter()
                    .filter(|&&p| {
                        objs.iter()
                            .any(|&id| crate::eval::same_controller_group(state, id) == Some(p))
                    })
                    .count(),
            )
            .unwrap_or(u32::MAX);
            (
                if req.min > 0 { eligible } else { 0 },
                if req.max > 0 { eligible } else { 0 },
            )
        }
        Some(baylee_cards_dsl::TargetConstraint::SameController) => {
            let mut counts: smallvec::SmallVec<[(PlayerId, u32); 4]> = smallvec::SmallVec::new();
            for &id in &objs {
                let Some(p) = crate::eval::same_controller_group(state, id) else {
                    continue;
                };
                if let Some((_, c)) = counts.iter_mut().find(|(q, _)| *q == p) {
                    *c = c.saturating_add(1);
                } else {
                    counts.push((p, 1));
                }
            }
            let best = counts.into_iter().map(|(_, c)| c).max().unwrap_or(0);
            let min_c = if req.count_is_x {
                0
            } else {
                u32::from(req.min)
            };
            let max_c = if req.count_is_x {
                best
            } else {
                req.bounds(0).1.min(best)
            };
            (min_c, max_c)
        }
        None | Some(baylee_cards_dsl::TargetConstraint::DistinctFromPriorTargetGroups) => {
            let avail = u32::try_from(objs.len() + pls.len()).unwrap_or(u32::MAX);
            let min_c = if req.count_is_x {
                0
            } else {
                u32::from(req.min)
            };
            let max_c = if req.count_is_x {
                avail
            } else {
                req.bounds(0).1.min(avail)
            };
            (min_c, max_c)
        }
    };
    ClauseTargets {
        min: min_c,
        max: max_c,
        objects: objs,
        players: pls,
    }
}

// Each distinct target requirement scans the board once per wall restriction.
// Repeated and entwined modes can reuse the same pool across many combinations.
type PreviewPools = SmallVec<[(baylee_cards_dsl::TargetReq, bool, ClauseTargets); 4]>;

fn clauses_preview_targets(
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
    def: &'static baylee_cards_dsl::CardDef,
    face: usize,
    clauses: &[Option<baylee_cards_dsl::TargetReq>],
    pools: &mut PreviewPools,
) -> (u32, u32, bool) {
    let wall_only =
        crate::eval::target_domain::wall_requirements(clauses.iter().copied().flatten());
    let mut min_d = 0_u32;
    let mut max_sum = 0_u32;
    let mut any_x = false;
    let mut union_objs: Vec<ObjectId> = Vec::new();
    let mut union_pls: Vec<PlayerId> = Vec::new();
    let mut seen_clause = false;
    for req in clauses.iter().copied().flatten() {
        let index = pools
            .iter()
            .position(|(saved, wall, _)| *saved == req && *wall == wall_only)
            .unwrap_or_else(|| {
                pools.push((
                    req,
                    wall_only,
                    clause_preview_targets(state, player, card, def, face, req, wall_only),
                ));
                pools.len() - 1
            });
        let pool = &pools[index].2;
        any_x |= req.count_is_x;
        max_sum = max_sum.saturating_add(pool.max);
        if !seen_clause {
            min_d = pool.min;
            union_objs.clone_from(&pool.objects);
            union_pls.clone_from(&pool.players);
            seen_clause = true;
            continue;
        }
        let disjoint = req.constraint()
            == Some(baylee_cards_dsl::TargetConstraint::DistinctFromPriorTargetGroups)
            || (!union_objs.iter().any(|o| pool.objects.contains(o))
                && !union_pls.iter().any(|p| pool.players.contains(p)));
        min_d = if disjoint {
            min_d.saturating_add(pool.min)
        } else {
            min_d.max(pool.min)
        };
        for &id in &pool.objects {
            if !union_objs.contains(&id) {
                union_objs.push(id);
            }
        }
        for &p in &pool.players {
            if !union_pls.contains(&p) {
                union_pls.push(p);
            }
        }
    }
    if !seen_clause {
        return (0, 0, false);
    }
    let union_cap = u32::try_from(union_objs.len() + union_pls.len()).unwrap_or(u32::MAX);
    (min_d, union_cap.min(max_sum), any_x)
}

pub(crate) fn preview_max_mandatory_x(
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
    face: &'static baylee_cards_dsl::FaceDef,
) -> u32 {
    if !crate::engine::cast_wizard::mandatory_scales_with_x(face) {
        return 0;
    }
    let choice = crate::engine::cast_wizard::CostChoice::default();
    let mut max = u32::MAX;
    let life_x_count = face
        .mandatory_additional_costs
        .iter()
        .filter(|p| matches!(p, baylee_cards_dsl::CostPart::PayLifeX))
        .count();
    if life_x_count > 0 {
        let fixed_life = crate::engine::cast_wizard::fixed_life_cost(face, choice);
        let life = state.life_payable(player).saturating_sub(fixed_life);
        let avail = u32::try_from(life).unwrap_or(0) / u32::try_from(life_x_count).unwrap_or(1);
        max = max.min(avail);
    }
    let energy_x_count = face
        .mandatory_additional_costs
        .iter()
        .filter(|p| matches!(p, baylee_cards_dsl::CostPart::PayEnergyX))
        .count();
    if energy_x_count > 0 {
        let fixed_energy = crate::engine::cast_wizard::fixed_energy_cost(face, choice);
        let avail = u32::from(state.players[usize::from(player.get())].energy)
            .saturating_sub(fixed_energy)
            / u32::try_from(energy_x_count).unwrap_or(1);
        max = max.min(avail);
    }
    if face.mandatory_additional_costs.iter().any(|p| {
        matches!(
            p,
            baylee_cards_dsl::CostPart::SacrificeX(_)
                | baylee_cards_dsl::CostPart::TapX(_)
                | baylee_cards_dsl::CostPart::DiscardX(_)
                | baylee_cards_dsl::CostPart::ExileFromGraveyardX(_)
                | baylee_cards_dsl::CostPart::RemoveAnyCounterX { .. }
        )
    }) {
        let counter_cap =
            crate::engine::cost_wizard::counter_remove::total_battlefield_counters(state, player);
        let cap = u32::try_from(
            state
                .battlefield_view()
                .len()
                .max(state.zones.list(ZoneLocation::Hand(player)).len())
                .max(state.zones.list(ZoneLocation::Graveyard(player)).len()),
        )
        .unwrap_or(u32::MAX)
        .max(counter_cap);
        max = greatest_affordable(max.min(cap), |x| {
            additional_payment_capacity_for_x(state, player, card, face, choice, x, &[]).is_some()
        });
    }
    if max == u32::MAX { 0 } else { max }
}

struct TargetConditionalRule {
    controller: PlayerId,
    context: crate::text_changes::RuleContext,
    target_filter: &'static baylee_cards_dsl::Filter,
    reduction: ManaCost,
}

fn collect_board_target_reductions(
    state: &GameState,
    player: PlayerId,
    spell: &crate::object::GameObject,
    rules: &mut Vec<TargetConditionalRule>,
) {
    for effect in state.effects.iter() {
        let baylee_cards_dsl::Modifier::ReduceCostIfTargets {
            who,
            spell_filter,
            target_filter,
            reduction,
            abilities: false,
        } = effect.modifier
        else {
            continue;
        };
        if reduction == ManaCost::ZERO
            || !spell_cost_effect_applies(state, effect, who, player, spell)
        {
            continue;
        }
        let player_matches =
            crate::eval::players_for_source(who, state, effect.controller, effect.source)
                .is_some_and(|players| players.contains(&player));
        if !player_matches {
            continue;
        }
        let context = crate::text_changes::RuleContext {
            source: effect.source.unwrap_or(spell.id),
            text: state.effect_text(effect),
            resolved_name: None,
            resolved_subtype: effect.filter.chosen_subtype(),
            reference: None,
        };
        if crate::eval::matches_with_context(spell_filter, state, spell, effect.controller, context)
        {
            rules.push(TargetConditionalRule {
                controller: effect.controller,
                context,
                target_filter,
                reduction,
            });
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn collect_self_target_reductions(
    state: &GameState,
    player: PlayerId,
    object: &crate::object::GameObject,
    spell: &crate::object::GameObject,
    def: &'static baylee_cards_dsl::CardDef,
    face_idx: usize,
    kind: PreviewCastKind,
    rules: &mut Vec<TargetConditionalRule>,
) {
    if object.status.contains(crate::object::Status::FACE_DOWN)
        || spell.characteristics().abilities_lost.is_some()
        || spell.characteristics().rules_text_lost
    {
        return;
    }
    let faces: smallvec::SmallVec<[usize; 2]> = if kind == PreviewCastKind::Fused {
        let mut f = smallvec::SmallVec::new();
        f.push(0);
        if def.faces.len() > 1 {
            f.push(1);
        }
        f
    } else {
        smallvec::smallvec![face_idx.min(def.faces.len().saturating_sub(1))]
    };
    for f_idx in faces {
        for ability in def.abilities_for_face(f_idx) {
            let baylee_cards_dsl::AbilityDef::Static(sa) = ability else {
                continue;
            };
            if !Zone::Stack.matches_ref(sa.source_zone) {
                continue;
            }
            let baylee_cards_dsl::Modifier::ReduceCostIfTargets {
                who,
                spell_filter,
                target_filter,
                reduction,
                abilities: false,
            } = sa.modifier
            else {
                continue;
            };
            if reduction == ManaCost::ZERO {
                continue;
            }
            let already_registered = state.effects.iter().any(|fx| {
                fx.source == Some(spell.id)
                    && fx.origin == crate::effects::EffectOrigin::Static
                    && fx.modifier == sa.modifier
            });
            if already_registered {
                continue;
            }
            let context = crate::eval::live_context(state, spell.id);
            if sa.condition.is_some_and(|cond| {
                !crate::eval::condition_holds_with_context(state, player, context, cond)
            }) || !crate::eval::matches_with_context(&sa.filter, state, spell, player, context)
            {
                continue;
            }
            let player_matches =
                crate::eval::players_for_source(who, state, player, Some(spell.id))
                    .is_some_and(|players| players.contains(&player));
            if player_matches
                && crate::eval::matches_with_context(spell_filter, state, spell, player, context)
            {
                rules.push(TargetConditionalRule {
                    controller: player,
                    context,
                    target_filter,
                    reduction,
                });
            }
        }
    }
}

fn collect_target_conditional_spell_reductions(
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
    def: &'static baylee_cards_dsl::CardDef,
    face_idx: usize,
    form: Option<SpellForm>,
    kind: PreviewCastKind,
) -> Vec<TargetConditionalRule> {
    let mut rules = Vec::new();
    let Some(object) = state.object(card) else {
        return rules;
    };
    if matches!(form, Some(SpellForm::Disguise)) {
        return rules;
    }
    let mut raw = object.clone();
    if kind != PreviewCastKind::Fused {
        raw.base = std::sync::Arc::new(crate::object::Characteristics::from_face(
            def,
            face_idx,
            object.base.name,
        ));
        raw.cache.clear();
        raw.face_index = u8::try_from(face_idx).unwrap_or(0);
    }
    if let Some(form) = form {
        raw = form.project(&raw);
    }
    let spell = projected_spell(state, player, &raw);
    collect_board_target_reductions(state, player, &spell, &mut rules);
    collect_self_target_reductions(
        state, player, object, &spell, def, face_idx, kind, &mut rules,
    );
    rules
}

#[allow(clippy::too_many_arguments)]
fn clause_matches_target_filter(
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
    def: &'static baylee_cards_dsl::CardDef,
    face: usize,
    req: baylee_cards_dsl::TargetReq,
    wall_only: bool,
    rule: &TargetConditionalRule,
) -> (bool, bool) {
    let ClauseTargets {
        min: min_c,
        max: max_c,
        objects: objs,
        players: pls,
    } = clause_preview_targets(state, player, card, def, face, req, wall_only);
    let total_opts = objs.len() + pls.len();
    if total_opts < usize::from(req.min) || min_c > max_c || max_c == 0 {
        return (false, false);
    }
    let matching_objs = objs
        .iter()
        .filter(|&&id| {
            state.object(id).is_some_and(|o| {
                crate::eval::matches_with_context(
                    rule.target_filter,
                    state,
                    o,
                    rule.controller,
                    rule.context,
                )
            })
        })
        .count();
    let achievable = matching_objs > 0;
    let non_matching = total_opts - matching_objs;
    let guaranteed = if req.constraint().is_some() {
        min_c > 0 && non_matching == 0 && matching_objs > 0
    } else {
        min_c > 0 && non_matching < usize::try_from(min_c).unwrap_or(usize::MAX)
    };
    (achievable, guaranteed)
}

#[allow(clippy::too_many_arguments)]
fn pair_matches_target_filter(
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
    def: &'static baylee_cards_dsl::CardDef,
    face: usize,
    first: Option<baylee_cards_dsl::TargetReq>,
    second: Option<baylee_cards_dsl::TargetReq>,
    third: Option<baylee_cards_dsl::TargetReq>,
    rule: &TargetConditionalRule,
) -> (bool, bool) {
    let wall_only = crate::eval::target_domain::walls3(first, second, third);
    let (mut achievable, mut guaranteed) = first.map_or((false, false), |req| {
        clause_matches_target_filter(state, player, card, def, face, req, wall_only, rule)
    });
    for extra in [second, third].into_iter().flatten() {
        let (a, g) = match first {
            Some(r1) if super::spec_reads_reference(extra.spec) => {
                let first_objs = crate::eval::spell_target_options_for(
                    &r1.spec,
                    state,
                    player,
                    card,
                    Some((def, face)),
                    None,
                    wall_only,
                );
                let any_extra = first_objs.into_iter().any(|reference| {
                    let sec_objs = crate::eval::spell_target_options_relative(
                        &extra.spec,
                        state,
                        player,
                        card,
                        Some((def, face)),
                        None,
                        wall_only,
                        Some(reference),
                    );
                    sec_objs.len() >= usize::from(extra.min)
                        && (extra.count_is_x || extra.max > 0)
                        && sec_objs.into_iter().any(|id| {
                            state.object(id).is_some_and(|o| {
                                crate::eval::matches_with_context(
                                    rule.target_filter,
                                    state,
                                    o,
                                    rule.controller,
                                    rule.context,
                                )
                            })
                        })
                });
                (any_extra, false)
            }
            _ => {
                clause_matches_target_filter(state, player, card, def, face, extra, wall_only, rule)
            }
        };
        achievable |= a;
        guaranteed |= g;
    }
    (achievable, guaranteed)
}

fn modal_matches_target_filter(
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
    def: &'static baylee_cards_dsl::CardDef,
    face_idx: usize,
    modes: &'static [baylee_cards_dsl::SpellMode],
    rule: &TargetConditionalRule,
) -> (bool, bool) {
    let mut any_achievable = false;
    let mut any_mode = false;
    let mut all_guaranteed = true;
    for mode in modes {
        let wall_only = crate::eval::target_domain::walls(mode.targets, mode.second_targets);
        let reachable = super::requirement_is_reachable_for(
            mode.targets,
            state,
            player,
            card,
            Some((def, face_idx)),
            wall_only,
        ) && super::requirement_is_reachable_for(
            mode.second_targets,
            state,
            player,
            card,
            Some((def, face_idx)),
            wall_only,
        );
        if !reachable {
            continue;
        }
        any_mode = true;
        let (a, g) = pair_matches_target_filter(
            state,
            player,
            card,
            def,
            face_idx,
            mode.targets,
            mode.second_targets,
            None,
            rule,
        );
        any_achievable |= a;
        all_guaranteed &= g;
    }
    (any_achievable, any_mode && all_guaranteed)
}

fn face_matches_target_filter(
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
    def: &'static baylee_cards_dsl::CardDef,
    face_idx: usize,
    kind: PreviewCastKind,
    rule: &TargetConditionalRule,
) -> (bool, bool) {
    match kind {
        PreviewCastKind::Bestow => clause_matches_target_filter(
            state,
            player,
            card,
            def,
            face_idx,
            super::BESTOW_TARGET_REQ,
            false,
            rule,
        ),
        PreviewCastKind::Mutate => clause_matches_target_filter(
            state,
            player,
            card,
            def,
            face_idx,
            super::MUTATE_TARGET_REQ,
            false,
            rule,
        ),
        PreviewCastKind::Fused => {
            let (a0, g0) = face_matches_target_filter(
                state,
                player,
                card,
                def,
                0,
                PreviewCastKind::Face,
                rule,
            );
            let (a1, g1) = if def.faces.len() > 1 {
                face_matches_target_filter(state, player, card, def, 1, PreviewCastKind::Face, rule)
            } else {
                (false, false)
            };
            (a0 || a1, g0 || g1)
        }
        PreviewCastKind::Face | PreviewCastKind::Kicked => {
            let Some(face) = def.faces.get(face_idx) else {
                return (false, false);
            };
            if let Some((first, second, third)) =
                def.abilities_for_face(face_idx)
                    .iter()
                    .find_map(|a| match a {
                        baylee_cards_dsl::AbilityDef::Spell {
                            targets,
                            second_targets,
                            third_targets,
                            ..
                        } => Some((*targets, *second_targets, *third_targets)),
                        _ => None,
                    })
            {
                let eff_first = if kind == PreviewCastKind::Kicked {
                    face.kicked_targets.or(first)
                } else {
                    first
                };
                return pair_matches_target_filter(
                    state, player, card, def, face_idx, eff_first, second, third, rule,
                );
            }
            if let Some((modes, ..)) = def
                .abilities_for_face(face_idx)
                .iter()
                .find_map(baylee_cards_dsl::AbilityDef::modal_spell)
            {
                return modal_matches_target_filter(
                    state, player, card, def, face_idx, modes, rule,
                );
            }
            if kind == PreviewCastKind::Kicked
                && let Some(req) = face.kicked_targets
            {
                return clause_matches_target_filter(
                    state, player, card, def, face_idx, req, false, rule,
                );
            }
            (false, false)
        }
    }
}

/// Computes preview dynamic cost bounds (`DistinctTargets`, `XPaid`, and
/// `ReduceCostIfTargets`) for `card`'s `face` before targets or `X` are
/// selected.
#[must_use]
pub(crate) fn preview_cast_metric_bounds(
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
    def: &'static baylee_cards_dsl::CardDef,
    face_idx: usize,
    adjustment: &SpellAdjustment,
) -> PreviewDynamicDelta {
    preview_cast_metric_bounds_for(
        state,
        player,
        card,
        def,
        face_idx,
        None,
        PreviewCastKind::Face,
        adjustment,
    )
}

fn preview_distinct_target_bounds(
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
    def: &'static baylee_cards_dsl::CardDef,
    face_idx: usize,
    face: &baylee_cards_dsl::FaceDef,
) -> (u32, u32, bool) {
    let mut min_d = 0_u32;
    let mut max_d = 0_u32;
    let mut count_is_x = false;
    let mut pools = PreviewPools::new();
    if let Some((first, second, third)) =
        def.abilities_for_face(face_idx)
            .iter()
            .find_map(|a| match a {
                baylee_cards_dsl::AbilityDef::Spell {
                    targets,
                    second_targets,
                    third_targets,
                    ..
                } => Some((*targets, *second_targets, *third_targets)),
                _ => None,
            })
    {
        (min_d, max_d, count_is_x) = clauses_preview_targets(
            state,
            player,
            card,
            def,
            face_idx,
            &[first, second, third],
            &mut pools,
        );
    } else if let Some((modes, choose, repeat)) = def
        .abilities_for_face(face_idx)
        .iter()
        .find_map(baylee_cards_dsl::AbilityDef::modal_spell)
    {
        let abilities = def.abilities_for_face(face_idx);
        min_d = if choose.min == 0 { 0 } else { u32::MAX };
        // CR 601.2b–f: price the targets of the complete set of modes.
        // Two modes naming the same object still contribute only one target.
        let mut record = |chosen: &[(usize, bool, baylee_cards_dsl::TargetReq)]| {
            let clauses: SmallVec<[_; 6]> = chosen.iter().map(|(_, _, req)| Some(*req)).collect();
            let (min, max, x) =
                clauses_preview_targets(state, player, card, def, face_idx, &clauses, &mut pools);
            min_d = min_d.min(min);
            max_d = max_d.max(max);
            count_is_x |= x;
        };
        if repeat {
            for set in repeated_mode_sets(modes, choose) {
                record(&repeated_modal_target_clauses(abilities, set));
            }
        } else {
            for set in mode_sets(modes, choose) {
                record(&modal_target_clauses(abilities, set));
            }
            if face.entwine.is_some() && choose.is_one() {
                record(&modal_target_clauses(abilities, entwine_set(modes)));
            }
        }
        if min_d == u32::MAX {
            min_d = 0;
        }
    }
    if face.bestow.is_some() || face.mutate.is_some() {
        max_d = max_d.max(1);
    }
    (min_d, max_d, count_is_x)
}

/// Computes preview dynamic cost bounds for a specific cast form and target
/// mode (`Face`, `Kicked`, `Bestow`, `Mutate`, `Fused`).
#[must_use]
#[allow(clippy::too_many_arguments)]
pub(crate) fn preview_cast_metric_bounds_for(
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
    def: &'static baylee_cards_dsl::CardDef,
    face_idx: usize,
    form: Option<SpellForm>,
    kind: PreviewCastKind,
    adjustment: &SpellAdjustment,
) -> PreviewDynamicDelta {
    if adjustment.per_distinct_target_delta == 0
        && adjustment.per_x_paid_delta == 0
        && !adjustment.has_target_conditional_reduction
    {
        return PreviewDynamicDelta::default();
    }
    let Some(face) = def.faces.get(face_idx) else {
        return PreviewDynamicDelta::default();
    };
    let mut out = PreviewDynamicDelta::default();
    let x_pips = face
        .mana_cost
        .with_x(1)
        .generic_total()
        .saturating_sub(face.mana_cost.with_x(0).generic_total());
    if adjustment.per_distinct_target_delta != 0 {
        let (min_d, max_d, count_is_x) =
            preview_distinct_target_bounds(state, player, card, def, face_idx, face);
        if adjustment.per_distinct_target_delta > 0 {
            let unit = adjustment.per_distinct_target_delta.unsigned_abs();
            out.min_increase = out.min_increase.saturating_add(unit.saturating_mul(min_d));
        } else {
            let unit = adjustment.per_distinct_target_delta.unsigned_abs();
            let effective = if count_is_x {
                unit.saturating_sub(x_pips)
            } else {
                unit
            };
            let min_red = unit.saturating_mul(min_d);
            let max_red = effective.saturating_mul(max_d).max(min_red);
            out.min_reduction = out.min_reduction.saturating_add(min_red);
            out.max_reduction = out.max_reduction.saturating_add(max_red);
        }
    }
    if adjustment.per_x_paid_delta < 0 {
        let max_x = preview_max_mandatory_x(state, player, card, face);
        let net_per_x = adjustment
            .per_x_paid_delta
            .unsigned_abs()
            .saturating_sub(x_pips);
        out.max_reduction = out
            .max_reduction
            .saturating_add(net_per_x.saturating_mul(max_x));
    }
    if adjustment.has_target_conditional_reduction {
        for rule in collect_target_conditional_spell_reductions(
            state, player, card, def, face_idx, form, kind,
        ) {
            let (achievable, guaranteed) =
                face_matches_target_filter(state, player, card, def, face_idx, kind, &rule);
            if achievable {
                out.max_mana_reduction = out.max_mana_reduction.combine(&rule.reduction);
            }
            if guaranteed {
                out.min_mana_reduction = out.min_mana_reduction.combine(&rule.reduction);
            }
        }
    }
    out
}

/// The public view still reports positive taxes separately from printed costs.
#[cfg(any(test, feature = "test-support"))]
#[doc(hidden)]
pub fn spell_increase(
    state: &GameState,
    player: PlayerId,
    object: &crate::object::GameObject,
) -> u32 {
    let adj = spell_adjustment(state, player, object);
    adj.increase.saturating_add(adj.mana_increase.cmc())
}

/// Additional generic mana for activating an ability of this object. The
/// filter's normal zone boundary deliberately excludes white enchantment
/// *cards* cycling in hand; Gloom names enchantments, i.e. permanents.
#[doc(hidden)]
pub fn activation_increase(state: &GameState, source: ObjectId) -> u32 {
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
#[doc(hidden)]
pub fn north_star_effect(
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

#[doc(hidden)]
pub fn north_star_face_allowed(
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
#[doc(hidden)]
pub fn normal_spell_affordable(
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
#[doc(hidden)]
pub fn component_affordable(
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
                        reduction,
                        ..SpellAdjustment::default()
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
