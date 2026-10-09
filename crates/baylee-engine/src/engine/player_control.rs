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
    /// 601.2h) from what the player has, asked in its payment window between
    /// two activations ([`Self::commanded_cost_feasible`] is the reading).
    pub(super) fn commanded_payment_feasible(&self) -> Option<bool> {
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
        self.commanded_cost_feasible(player, wizard.card, cost)
    }

    /// Whether `cost`, the price of the commanded card `card`, can be paid
    /// (CR 601.2g, 601.2h) from what `player` has: the mana in their pool, of
    /// which the obligated units must all be spent on it, and one activation
    /// of each land mana ability still on offer, every unit of which must be
    /// spent on it or on another land's mana ability (Word of Command).
    ///
    /// Exact over what it reads, and it reads a land's mana ability with a
    /// mana cost (a filter land's `{1}, {T}`) and one granted to the land as
    /// well as a plain `{T}`: every order of activations and every way of
    /// paying an activation's generic mana is tried, as states of what has
    /// been spent and added. `None` where some land's mana is beyond it — a
    /// cost other than mana and `{T}`, an amount or colour only the board
    /// knows, mana with a restriction — or the search outgrows its bound, and
    /// the caller falls back to the optimistic proof.
    pub(super) fn commanded_cost_feasible(
        &self,
        player: PlayerId,
        card: ObjectId,
        cost: &baylee_core::mana::ManaCost,
    ) -> Option<bool> {
        use baylee_core::mana::ManaColor;
        let obligation = self
            .state
            .constrained_payment(player)
            .filter(|payment| payment.card.object == card)?;
        let required = crate::constrained_payment::amounts(&obligation.required)?;
        if cost.has_variable() || !obligation.required.restricted().is_empty() {
            return None;
        }
        let lands = self.commanded_land_sources(player)?;
        if lands.len() > 16 {
            return None;
        }
        let pool = &self.state.players[usize::from(player.get())].mana_pool;
        let spending = crate::casting::mana_spending(&self.state, player);
        let base = ManaColor::ALL.map(|color| pool.available(color));
        // Every unit a land adds must be spent on the card or on another
        // land's activation, so no state may owe more than all of those
        // can still take.
        let sink = |used: u32| -> u32 {
            cost.cmc().saturating_add(
                lands
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| used & (1 << i) == 0)
                    .map(|(_, land)| land.most_paid())
                    .sum::<u32>(),
            )
        };
        let pays = |state: &Search| -> bool {
            let mut future = pool.clone();
            for color in ManaColor::ALL {
                let i = color.index();
                // Added first: what an activation spent may be what an
                // earlier one made.
                if state.added[i] > 0 {
                    future.add(color, state.added[i]);
                }
                if state.spent[i] > 0 && !future.spend(color, state.spent[i]) {
                    return false;
                }
            }
            crate::mana_pay::payment_consuming(&future, cost, spending, [0; 6], None, state.owed)
                .is_some()
        };
        let start = Search {
            used: 0,
            added: [0; 6],
            spent: [0; 6],
            owed: required,
        };
        let mut seen: std::collections::BTreeSet<Search> = std::collections::BTreeSet::new();
        let mut frontier = vec![start];
        seen.insert(start);
        while let Some(state) = frontier.pop() {
            if pays(&state) {
                return Some(true);
            }
            for (i, land) in lands.iter().enumerate() {
                if state.used & (1 << i) != 0 {
                    continue;
                }
                let avail: [u32; 6] =
                    std::array::from_fn(|c| base[c] + state.added[c] - state.spent[c]);
                for &(price, color, amount) in &land.options {
                    for payment in price.payments(&avail) {
                        let mut next = state;
                        next.used |= 1 << i;
                        for (c, &paid) in payment.iter().enumerate() {
                            next.spent[c] += paid;
                            // Spending an obligated unit is never worse
                            // than spending a free one of the same colour.
                            next.owed[c] -= next.owed[c].min(paid);
                        }
                        next.added[color.index()] += amount;
                        next.owed[color.index()] += amount;
                        if next.owed.iter().sum::<u32>() > sink(next.used) {
                            continue;
                        }
                        if seen.insert(next) {
                            if seen.len() > 50_000 {
                                return None;
                            }
                            frontier.push(next);
                        }
                    }
                }
            }
        }
        Some(false)
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

    /// The lands the commanded player may still activate for mana, each
    /// with what activating it costs in mana and the ways it could add mana;
    /// `None` when any such land's mana is beyond a fixed reading.
    fn commanded_land_sources(&self, player: PlayerId) -> Option<Vec<LandSource>> {
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
        // The abilities on offer, and those whose mana cost the pool cannot
        // pay yet but whose other parts it can (`unpaid_abilities`): a
        // filter land's `{1}, {T}` is reached once another land has paid it.
        let mut activations: Vec<(ObjectId, u32)> = legal.abilities.clone();
        for &(source, index, _) in &legal.unpaid_abilities {
            if !activations.contains(&(source, index)) {
                activations.push((source, index));
            }
        }
        let mut sources: Vec<ObjectId> = legal.mana_abilities.clone();
        for &(source, _) in &activations {
            if !sources.contains(&source) {
                sources.push(source);
            }
        }
        let mut lands = Vec::with_capacity(sources.len());
        for source in sources {
            let object = self.state.object(source)?;
            let increase = crate::casting::activation_increase(&self.state, source);
            let mut land = LandSource::default();
            let push = |land: &mut LandSource, color, amount, price: Price| {
                if !land.options.contains(&(price, color, amount)) {
                    land.options.push((price, color, amount));
                }
            };
            if legal.mana_abilities.contains(&source) {
                let price = Price::of(&baylee_core::mana::ManaCost::ZERO, increase)?;
                for color in crate::casting::intrinsic_mana_choices(
                    &self.state,
                    &self.lookup,
                    player,
                    source,
                ) {
                    push(&mut land, color, 1, price);
                }
            }
            for &(_, index) in activations.iter().filter(|(s, _)| *s == source) {
                let (cost, effects) = if let Some(slot) = crate::choice::granted_slot(index) {
                    let granted = crate::effects::granted_activated(&self.state, source)
                        .nth(slot as usize)?;
                    if !granted.mana_ability {
                        return None;
                    }
                    (granted.cost, granted.effects)
                } else {
                    let ability = object.abilities(&self.lookup).get(index as usize)?;
                    if ability.is_intrinsic_mana_ability() {
                        let price = Price::of(&baylee_core::mana::ManaCost::ZERO, increase)?;
                        for color in crate::casting::intrinsic_mana_colors(&self.state, source) {
                            push(&mut land, color, 1, price);
                        }
                        continue;
                    }
                    let (AbilityDef::Activated { cost, effects, .. }
                    | AbilityDef::ActivatedConditional { cost, effects, .. }) = ability
                    else {
                        return None;
                    };
                    (*cost, *effects)
                };
                // `{T}` and mana, and nothing else: a life, a sacrifice or a
                // counter is a price this reading does not weigh.
                if !cost
                    .parts
                    .iter()
                    .all(|part| matches!(part, baylee_cards_dsl::CostPart::TapSelf))
                {
                    return None;
                }
                let price = Price::of(&cost.mana, increase)?;
                // What it makes, read off its effects alone: the mana in its
                // cost is the price above, not part of the reading.
                let (made, restricted) =
                    baylee_cards_dsl::mana_made(&baylee_cards_dsl::Cost::TAP, effects)?;
                if restricted || made.colors.is_empty() {
                    return None;
                }
                for &color in &made.colors {
                    push(&mut land, color, u32::from(made.amount), price);
                }
            }
            lands.push(land);
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

/// One state of the commanded payment's search: the lands used, and the
/// mana added and spent on activations so far, with the units still owed
/// to the card (every added unit is, until something spends it).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Search {
    used: u32,
    added: [u32; 6],
    spent: [u32; 6],
    owed: [u32; 6],
}

/// What a land mana ability costs in mana, by colour (colorless included)
/// and generic.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Price {
    colored: [u32; 6],
    generic: u32,
}

impl Price {
    /// `cost` and an `increase` of generic mana, when every symbol is a
    /// colour, colorless or generic; `None` for anything else (hybrid,
    /// Phyrexian, snow, X).
    fn of(cost: &baylee_core::mana::ManaCost, increase: u32) -> Option<Self> {
        use baylee_core::mana::{ManaColor, ManaSymbol};
        let mut price = Self {
            colored: [0; 6],
            generic: increase,
        };
        for symbol in cost.symbols() {
            let color = match symbol {
                ManaSymbol::Generic(n) => {
                    price.generic += n;
                    continue;
                }
                ManaSymbol::Colorless => ManaColor::Colorless,
                ManaSymbol::White => ManaColor::White,
                ManaSymbol::Blue => ManaColor::Blue,
                ManaSymbol::Black => ManaColor::Black,
                ManaSymbol::Red => ManaColor::Red,
                ManaSymbol::Green => ManaColor::Green,
                _ => return None,
            };
            price.colored[color.index()] += 1;
        }
        Some(price)
    }

    fn total(&self) -> u32 {
        self.colored.iter().sum::<u32>() + self.generic
    }

    /// Every way to pay this out of `avail`, as units spent by colour.
    fn payments(&self, avail: &[u32; 6]) -> Vec<[u32; 6]> {
        fn spread(
            left: u32,
            c: usize,
            room: &[u32; 6],
            current: &mut [u32; 6],
            out: &mut Vec<[u32; 6]>,
        ) {
            if c == 6 {
                if left == 0 {
                    out.push(*current);
                }
                return;
            }
            for k in 0..=left.min(room[c]) {
                current[c] += k;
                spread(left - k, c + 1, room, current, out);
                current[c] -= k;
            }
        }
        let mut out = Vec::new();
        if (0..6).any(|c| self.colored[c] > avail[c]) {
            return out;
        }
        let room: [u32; 6] = std::array::from_fn(|c| avail[c] - self.colored[c]);
        let mut current = self.colored;
        spread(self.generic, 0, &room, &mut current, &mut out);
        out
    }
}

/// A land the commanded player may activate for mana once: each way it
/// could, as its price, the colour and how much.
#[derive(Clone, Debug, Default)]
struct LandSource {
    options: Vec<(Price, baylee_core::mana::ManaColor, u32)>,
}

impl LandSource {
    /// The most mana activating it can consume.
    fn most_paid(&self) -> u32 {
        self.options
            .iter()
            .map(|(price, ..)| price.total())
            .max()
            .unwrap_or(0)
    }
}
