//! Whether an activation's mana could be made at all, before it is made.
//!
//! An ability with `{X}` in its mana cost is offered before its mana floats:
//! its X is announced first and its mana made in its payment window (CR
//! 602.2b, by 601.2b and 601.2g), so the offer keeps it while the seat can
//! make mana (`compute_legal`). "Can make mana" alone kept Ballista Squad's
//! `{X}{W}, {T}` on offer beside nothing but an Island (Carbon b8, segment 3,
//! attempt 42, q126): no mana that seat could make pays `{W}`, and taking it
//! opened a window that could only be reversed (CR 734.1). This is the reach
//! the offer asks for as well: what the floating mana and the mana abilities
//! on offer could make, colour by colour.
use super::{CardLookup, Engine, PlayerId};
use crate::choice::LegalActions;
use baylee_core::ids::ObjectId;
use baylee_core::mana::{ManaColor, ManaCost, ManaSpending, ManaSymbol};

impl<L: CardLookup> Engine<L> {
    /// Whether the mana `player` has floating and the mana abilities `legal`
    /// offers could make `cost` at all, for ability `index` of `source`
    /// (CR 601.2g, 602.2b): [`Self::makes_activation_mana`], and then every
    /// coloured symbol a colour that mana or a source can make, and as much
    /// mana in all.
    ///
    /// A necessary condition, never a plan: each source counts toward every
    /// colour it may make and in full toward the total, a source this does
    /// not read (a mana ability with a price, an "if", a granted shortcut on
    /// no basic land type) makes anything, Phyrexian symbols are life, and a
    /// permission to spend mana as another type (CR 609.4b) answers yes. So
    /// it refuses only a cost no assignment of that mana could pay.
    pub(super) fn reaches_activation_mana(
        &self,
        player: PlayerId,
        legal: &LegalActions,
        source: ObjectId,
        index: u32,
        cost: &ManaCost,
    ) -> bool {
        if !self.makes_activation_mana(player, legal, source, index) {
            return false;
        }
        if crate::casting::ability_mana_spending(&self.state, player, source) != ManaSpending::EXACT
        {
            return true;
        }
        let excluded = self.reserved_by_cost(source, index);
        let mut reach = Reach::default();
        let pool = &self.state.players[usize::from(player.get())].mana_pool;
        for color in ManaColor::ALL {
            reach.add(&[color], Some(pool.available(color)));
        }
        for mana in pool.restricted() {
            reach.add(&[mana.color], Some(u32::from(mana.amount)));
        }
        reach.add(
            &[ManaColor::Colorless],
            Some(self.state.granted_colorless_capacity(player)),
        );
        for offer in &legal.granted_actions {
            if let crate::choice::GrantedActionKind::AddMana { color, amount } = offer.effect {
                reach.add(&[color], Some(u32::from(amount)));
            }
        }
        for &id in legal.mana_abilities.iter().filter(|&&id| id != excluded) {
            let colors = crate::casting::intrinsic_mana_colors(&self.state, id);
            if colors.is_empty() {
                reach.anything();
            } else {
                reach.add(&colors, Some(1));
            }
        }
        for &(id, at) in &legal.abilities {
            if id != excluded && self.is_payment_mana_offer(id, at) {
                self.reach_of(player, id, at, &mut reach);
            }
        }
        reach.pays(cost)
    }

    /// What mana ability `index` of `id` adds to `reach`: the colours its
    /// mana may be ([`crate::resolve::colors_of`], the reader the mana is
    /// made by) and how much, or anything where the ability is not one this
    /// reads.
    fn reach_of(&self, player: PlayerId, id: ObjectId, index: u32, reach: &mut Reach) {
        let Some(
            baylee_cards_dsl::AbilityDef::Activated { cost, effects, .. }
            | baylee_cards_dsl::AbilityDef::ActivatedConditional { cost, effects, .. },
        ) = self.activation_definition(id, index)
        else {
            reach.anything();
            return;
        };
        if let Some((made, amount, _)) = baylee_cards_dsl::mana_with_riders(&cost, effects) {
            let colors = crate::resolve::colors_of(&self.state, player, made, id);
            reach.add(&colors, amount.map(u32::from));
        } else if let Some(colors) = baylee_cards_dsl::mana_bundle(&cost, effects) {
            for color in colors {
                reach.add(&[color], Some(1));
            }
        } else {
            reach.anything();
        }
    }
}

/// What some mana could come to, read loosely: per colour (`ManaColor::index`)
/// the most of it there could be, and the most in all; a colour or the total
/// is open where a source's amount is not a number this reads.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Reach {
    by_color: [u64; 6],
    open: [bool; 6],
    total: u64,
    open_total: bool,
}

impl Reach {
    /// A source of `amount` mana, each of which may be any of `colors`;
    /// `None` for an amount this does not read. A source of no colour adds
    /// nothing (a lone Reflecting Pool, CR 106.7).
    pub(super) fn add(&mut self, colors: &[ManaColor], amount: Option<u32>) {
        if colors.is_empty() || amount == Some(0) {
            return;
        }
        let Some(amount) = amount else {
            for color in colors {
                self.open[color.index()] = true;
            }
            self.open_total = true;
            return;
        };
        for color in colors {
            let slot = &mut self.by_color[color.index()];
            *slot = slot.saturating_add(u64::from(amount));
        }
        self.total = self.total.saturating_add(u64::from(amount));
    }

    /// A source this does not read: any colour, any amount.
    pub(super) fn anything(&mut self) {
        self.open = [true; 6];
        self.open_total = true;
    }

    /// Whether `cost` (its X already a number) is within this reach: each
    /// colour's symbols within that colour's mana, each hybrid symbol with
    /// one of its colours there at all, and every mana symbol within the
    /// total. Generic, `{2/C}`, snow and the silver-bordered symbols ask the
    /// total only; Phyrexian symbols ask nothing, since life pays them
    /// (CR 107.4f).
    pub(super) fn pays(&self, cost: &ManaCost) -> bool {
        let mut need = [0_u64; 6];
        let mut total = 0_u64;
        let mut hybrids = Vec::new();
        for symbol in cost.symbols() {
            let color = match symbol {
                ManaSymbol::White => Some(ManaColor::White),
                ManaSymbol::Blue => Some(ManaColor::Blue),
                ManaSymbol::Black => Some(ManaColor::Black),
                ManaSymbol::Red => Some(ManaColor::Red),
                ManaSymbol::Green => Some(ManaColor::Green),
                ManaSymbol::Colorless => Some(ManaColor::Colorless),
                ManaSymbol::Generic(n) => {
                    total = total.saturating_add(u64::from(n));
                    None
                }
                ManaSymbol::Hybrid(pair) => {
                    hybrids.push(pair);
                    total = total.saturating_add(1);
                    None
                }
                ManaSymbol::TwoOrColor(_)
                | ManaSymbol::Snow
                | ManaSymbol::HalfGeneric
                | ManaSymbol::Infinite => {
                    total = total.saturating_add(1);
                    None
                }
                ManaSymbol::Phyrexian(_)
                | ManaSymbol::HybridPhyrexian(_)
                | ManaSymbol::Variable(_) => None,
            };
            if let Some(color) = color {
                need[color.index()] += 1;
                total = total.saturating_add(1);
            }
        }
        let has = |color: ManaColor, more: u64| {
            self.open[color.index()] || self.by_color[color.index()] >= more
        };
        ManaColor::ALL
            .into_iter()
            .all(|color| has(color, need[color.index()]))
            && hybrids.iter().all(|pair| {
                let (a, b) = (
                    ManaColor::from_color(pair.first()),
                    ManaColor::from_color(pair.second()),
                );
                has(a, need[a.index()] + 1) || has(b, need[b.index()] + 1)
            })
            && (self.open_total || self.total >= total)
    }
}
