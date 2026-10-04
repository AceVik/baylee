//! A decision's actor is distinct from the player whose resources it uses.

use super::{CardLookup, Engine, ObjectId, PlayerId};
use baylee_core::ids::DamageSourceRef;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Control {
    source: DamageSourceRef,
    controller: PlayerId,
    player: PlayerId,
}

/// Exact spells awaiting their later control segment, and currently resolving segments.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub(super) struct PlayerControl {
    waiting: Vec<Control>,
    active: Vec<Control>,
}

impl<L: CardLookup> Engine<L> {
    /// A necessary-condition rejection, never a complete mana-plan solver.
    /// With only cost-free mana producers remaining, generated mana has no
    /// consumer except the selected spell. Extra hypothetical resources make
    /// this an optimistic check: failure proves the activation cannot finish.
    pub(super) fn commanded_payment_dead_end(&self) -> bool {
        use baylee_core::mana::{ManaColor, ManaPool};
        // The exact answer where one can be given; the optimistic proof
        // below only where a land's mana is beyond the exact reader.
        if let Some(feasible) = self.commanded_payment_feasible() {
            return !feasible;
        }
        let Some(super::PaymentWindow {
            player,
            suspended: super::PaymentContinuation::Miracle { wizard, cost, .. },
        }) = &self.mana_window
        else {
            return false;
        };
        let Some(obligation) = self
            .state
            .constrained_payment(*player)
            .filter(|payment| payment.card.object == wizard.card && !payment.required.is_empty())
        else {
            return false;
        };
        let Some(required) = crate::constrained_payment::amounts(&obligation.required) else {
            return true;
        };
        if self.land_mana_might_consume(*player) {
            return false;
        }
        let mut future = ManaPool::new();
        for color in ManaColor::ALL {
            future.add_snow(color, u32::MAX);
        }
        crate::mana_pay::payment_consuming(
            &future,
            cost,
            crate::casting::mana_spending(&self.state, *player),
            [0; 6],
            None,
            required,
        )
        .is_none()
    }

    /// Whether the commanded card's payment can still be completed (CR 601.2g,
    /// 601.2h) from what the player has: the mana in their pool, of which the
    /// obligated units must all be spent on it, and one activation of each
    /// land mana ability still on offer, every unit of which must be spent on
    /// it too (Word of Command). `None` where some land's mana is beyond the
    /// exact reader — a mana cost, a non-tap cost, a restriction, a granted
    /// ability — and the caller falls back to the optimistic proof.
    ///
    /// Exact over what it reads: every combination of lands and colours is
    /// tried, as the vectors of mana they could add. Units that the cost
    /// cannot consume are a dead end, so no vector may hold more than the
    /// cost's symbols; that bounds the search by the cost, not the board.
    pub(super) fn commanded_payment_feasible(&self) -> Option<bool> {
        use baylee_core::mana::ManaColor;
        let super::PaymentWindow {
            player,
            suspended: super::PaymentContinuation::Miracle { wizard, cost, .. },
        } = self.mana_window.as_ref()?
        else {
            return None;
        };
        let player = *player;
        // Only between activations: while a mana ability is still asking
        // (its colour), its land is tapped and its mana not yet made, and
        // the board says nothing exact about the payment.
        if !matches!(&self.pending, super::Pending::Priority { player: asked, .. } if *asked == player)
        {
            return None;
        }
        let obligation = self
            .state
            .constrained_payment(player)
            .filter(|payment| payment.card.object == wizard.card)?;
        let required = crate::constrained_payment::amounts(&obligation.required)?;
        if cost.has_variable() || !obligation.required.restricted().is_empty() {
            return None;
        }
        let lands = self.commanded_land_outputs(player)?;
        let cap = cost.cmc().saturating_mul(2);
        let mut reachable: std::collections::BTreeSet<[u32; 6]> =
            std::collections::BTreeSet::from([[0; 6]]);
        for options in &lands {
            let mut next = reachable.clone();
            for added in &reachable {
                for &(color, amount) in options {
                    let mut grown = *added;
                    grown[color.index()] = grown[color.index()].saturating_add(amount);
                    if grown.iter().sum::<u32>() <= cap {
                        next.insert(grown);
                    }
                }
            }
            reachable = next;
        }
        let pool = &self.state.players[usize::from(player.get())].mana_pool;
        let spending = crate::casting::mana_spending(&self.state, player);
        Some(reachable.iter().any(|added| {
            let mut future = pool.clone();
            let mut owed = required;
            for color in ManaColor::ALL {
                let n = added[color.index()];
                if n > 0 {
                    future.add(color, n);
                    owed[color.index()] = owed[color.index()].saturating_add(n);
                }
            }
            crate::mana_pay::payment_consuming(&future, cost, spending, [0; 6], None, owed)
                .is_some()
        }))
    }

    /// Whether the commanded player's pool alone pays the commanded card
    /// now, spending every obligated unit.
    pub(super) fn commanded_pool_completes(&self) -> bool {
        let Some(super::PaymentWindow {
            player,
            suspended: super::PaymentContinuation::Miracle { wizard, cost, .. },
        }) = self.mana_window.as_ref()
        else {
            return false;
        };
        let Some(required) = self
            .state
            .constrained_payment(*player)
            .filter(|payment| payment.card.object == wizard.card)
            .and_then(|payment| crate::constrained_payment::amounts(&payment.required))
        else {
            return false;
        };
        crate::mana_pay::payment_consuming(
            &self.state.players[usize::from(player.get())].mana_pool,
            cost,
            crate::casting::mana_spending(&self.state, *player),
            [0; 6],
            None,
            required,
        )
        .is_some()
    }

    /// What each land the commanded player may still activate could add,
    /// one entry per land and one option per colour it could make; `None`
    /// when any such land's mana is beyond a fixed, tap-only reading.
    fn commanded_land_outputs(
        &self,
        player: PlayerId,
    ) -> Option<Vec<Vec<(baylee_core::mana::ManaColor, u32)>>> {
        use baylee_cards_dsl::AbilityDef;
        let mut legal = self.compute_legal(player);
        self.narrow_to_mana(&mut legal);
        super::constrained_mana::ManaActivationScope::ControlledLands.narrow(
            &self.state,
            player,
            &mut legal,
        );
        // A special action with mana-ability timing (Channel's life for
        // {C}) is a source beyond the lands, with no obligation on its mana.
        if !legal.granted_actions.is_empty() {
            return None;
        }
        let mut sources: Vec<ObjectId> = legal.mana_abilities.clone();
        for &(source, _) in &legal.abilities {
            if !sources.contains(&source) {
                sources.push(source);
            }
        }
        let mut lands = Vec::with_capacity(sources.len());
        for source in sources {
            if crate::casting::activation_increase(&self.state, source) != 0 {
                return None;
            }
            let object = self.state.object(source)?;
            let mut options: Vec<(baylee_core::mana::ManaColor, u32)> = Vec::new();
            let mut push = |color, amount| {
                if !options.contains(&(color, amount)) {
                    options.push((color, amount));
                }
            };
            if legal.mana_abilities.contains(&source) {
                for color in crate::casting::intrinsic_mana_choices(
                    &self.state,
                    &self.lookup,
                    player,
                    source,
                ) {
                    push(color, 1);
                }
            }
            for &(_, index) in legal.abilities.iter().filter(|(s, _)| *s == source) {
                let ability = object.abilities(&self.lookup).get(index as usize)?;
                if ability.is_intrinsic_mana_ability() {
                    for color in crate::casting::intrinsic_mana_colors(&self.state, source) {
                        push(color, 1);
                    }
                    continue;
                }
                let (AbilityDef::Activated { cost, effects, .. }
                | AbilityDef::ActivatedConditional { cost, effects, .. }) = ability
                else {
                    return None;
                };
                if !baylee_cards_dsl::tap_only(cost) {
                    return None;
                }
                let (made, restricted) = baylee_cards_dsl::mana_made(cost, effects)?;
                if restricted || made.colors.is_empty() {
                    return None;
                }
                for &color in &made.colors {
                    push(color, u32::from(made.amount));
                }
            }
            lands.push(options);
        }
        Some(lands)
    }

    /// Any potential mana cost or stateful mana effect defeats the simple
    /// no-consumer proof. In particular repeatable X=0 storage abilities are
    /// legal sinks, and filter lands may consume one color to produce another.
    fn land_mana_might_consume(&self, player: PlayerId) -> bool {
        use baylee_cards_dsl::{AbilityDef, Effect};
        let only_adds = |effects: &[Effect]| {
            effects
                .iter()
                .all(|effect| matches!(effect, Effect::AddMana { .. }))
        };
        for id in self.state.battlefield_seen() {
            let Some(object) = self.state.object(id) else {
                continue;
            };
            if object.controller != player
                || !object
                    .characteristics()
                    .types
                    .contains(baylee_core::types::TypeSet::LAND)
            {
                continue;
            }
            if crate::casting::activation_increase(&self.state, id) != 0 {
                return true;
            }
            for ability in object.abilities(&self.lookup) {
                match ability {
                    AbilityDef::Activated {
                        cost,
                        effects,
                        mana_ability: true,
                        ..
                    }
                    | AbilityDef::ActivatedConditional {
                        cost,
                        effects,
                        mana_ability: true,
                        ..
                    } if cost.mana != baylee_core::mana::ManaCost::ZERO || !only_adds(effects) => {
                        return true;
                    }
                    _ => {}
                }
            }
            if crate::effects::granted_activated(&self.state, id).any(|ability| {
                ability.mana_ability
                    && (ability.cost.mana != baylee_core::mana::ManaCost::ZERO
                        || !only_adds(ability.effects))
            }) {
                return true;
            }
        }
        false
    }

    pub(super) fn controlled_actor(&self, player: PlayerId) -> PlayerId {
        let mut actor = player;
        let mut visited = baylee_core::ids::SeatSet::new();
        while !visited.contains(actor) {
            visited.insert(actor);
            let control = self
                .effect_plays
                .iter()
                .rev()
                .filter_map(super::effect_play::EffectPlay::control)
                .find(|&(_, subject)| subject == actor)
                .or_else(|| {
                    self.player_control
                        .active
                        .iter()
                        .rev()
                        .find(|entry| entry.player == actor)
                        .map(|entry| (entry.controller, entry.player))
                });
            let Some((controller, _)) = control else {
                break;
            };
            if self
                .state
                .players
                .get(controller.get() as usize)
                .is_none_or(crate::state::Player::has_lost)
            {
                break;
            }
            actor = controller;
        }
        actor
    }

    /// Other players whose private information this controller may inspect now.
    #[must_use]
    pub fn controlled_players(&self, viewer: PlayerId) -> baylee_core::ids::SeatSet {
        self.state
            .players
            .iter()
            .filter(|p| p.id != viewer && !p.has_lost() && self.may_inspect_private(viewer, p.id))
            .map(|p| p.id)
            .collect()
    }

    /// The special mana restrictions apply only while playing the selected card.
    pub(super) fn commanded_player(&self) -> Option<PlayerId> {
        self.effect_plays
            .last()
            .and_then(super::effect_play::EffectPlay::control)
            .map(|(_, player)| player)
    }

    pub(super) fn commanded_card(&self, card: ObjectId) -> bool {
        self.effect_plays
            .last()
            .is_some_and(|frame| frame.commanded_card() == Some(card))
    }

    pub(super) fn remember_controlled_spell(
        &mut self,
        card: ObjectId,
        controller: PlayerId,
        player: PlayerId,
    ) {
        if let Some(source) = self.state.source_identity(card) {
            self.player_control.waiting.push(Control {
                source,
                controller,
                player,
            });
        }
    }

    pub(super) fn begin_controlled_resolution(&mut self, card: ObjectId) {
        let Some(source) = self.state.source_identity(card) else {
            return;
        };
        if let Some(index) = self
            .player_control
            .waiting
            .iter()
            .position(|entry| entry.source == source)
        {
            self.player_control
                .active
                .push(self.player_control.waiting.remove(index));
        }
    }

    pub(super) fn end_controlled_resolution(&mut self, card: ObjectId) {
        self.player_control
            .active
            .retain(|entry| entry.source.object != card);
    }

    pub(super) fn prune_player_control(&mut self) {
        self.player_control.waiting.retain(|entry| {
            self.state
                .object(entry.source.object)
                .is_some_and(|object| {
                    object.version == entry.source.version
                        && object.zone == crate::zone::Zone::Stack
                })
        });
        // Entry replacement choices still belong to the resolving permanent spell.
        if self.entry_questions.is_empty() {
            self.player_control.active.retain(|entry| {
                self.state
                    .object(entry.source.object)
                    .is_some_and(|object| {
                        object.version == entry.source.version
                            && object.zone == crate::zone::Zone::Stack
                    })
            });
        }
    }
}
