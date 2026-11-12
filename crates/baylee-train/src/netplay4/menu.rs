//! `eval4 --menu-payable`: a dev-only arm, off by default.
//!
//! The spells of `legal.payable` the planner can pay join the priority menu at
//! run time, as `features7::encode_candidates7` adds them and as the language
//! model's seat offers them (`baylee_seat::wake::reachable`). When the net
//! answers with one, the planner taps for it step by step and the spell is
//! then cast from the pool, as a seat's `Act::Taps` does. A card of
//! `legal.unpaid_suspend` whose suspend cost the planner pays joins it the same
//! way, and is suspended once its taps are made (CR 116.2f). The net's own menu
//! stays as it is, so the arm only ever adds answers.
//!
//! The net never saw these candidates: in training a spell is on the menu only
//! with its mana floating. [`MenuStats4`] says what a frozen net makes of them.
//! As a way to play, the arm is a measurement and nothing else: `eval4`
//! refuses it with collection, search and shadow labels, and no run played
//! with it may become policy data.
//!
//! The menu itself may be an input (coordinator, 12 November 2026, on the
//! owner's ruling of 11 November that the planner-paid menu is no rule
//! script): the search student reads this payable menu at the search's
//! roots, in a second pass beside the seat's own (`search_advantage_v1`, its
//! labels the search's fork advantages). It is built by this module's
//! [`PayableMenu4`] wherever it is read, in conversion, in the evaluator and
//! in the shipped seat alike, so the student meets the same menu in training
//! and in play.

use baylee_client_core::manaplan::Plan;
use baylee_core::ids::ObjectId;
use baylee_engine::choice::{CastModeKind, LegalActions, Pending, PlayerAction};
use baylee_view::{Phase, PlayerView};

use super::script::{Script4, ScriptAnswer4};
use super::tactics::{lookup_card_identity, lookup_spell_cost};
use crate::features4::{EncodedCandidates4, MAX_CANDIDATES};

/// What a run's net did with the options it never saw.
#[derive(Clone, Copy, Debug, Default, serde::Serialize)]
pub struct MenuStats4 {
    /// Priority menus that held at least one added option.
    pub menus: u64,
    /// Added options offered, over all menus.
    pub options: u64,
    /// Menus whose best logit was an added option.
    pub topped: u64,
    /// Menus whose best added option beat passing.
    pub beat_pass: u64,
    /// Menus by the added options' share of the softmax at `T = 1`, in tenths.
    pub mass: [u64; 10],
    /// Answers that chose an added option.
    pub chosen: u64,
    /// Of those, on the other seat's turn.
    pub chosen_their_turn: u64,
    /// Of those, in the seat's own main phases.
    pub chosen_own_main: u64,
    /// Taps the planner made.
    pub taps: u64,
    /// Spells cast, or cards suspended, once the taps were made.
    pub cast: u64,
    /// Plans ended early: a tap the table would not take, or the moment passed.
    pub broken: u64,
    /// Plans whose taps all went through and whose spell was not castable.
    pub not_castable: u64,
}

impl MenuStats4 {
    /// Adds `other` to these counts.
    pub fn add(&mut self, other: &Self) {
        self.menus += other.menus;
        self.options += other.options;
        self.topped += other.topped;
        self.beat_pass += other.beat_pass;
        for (mine, theirs) in self.mass.iter_mut().zip(&other.mass) {
            *mine += theirs;
        }
        self.chosen += other.chosen;
        self.chosen_their_turn += other.chosen_their_turn;
        self.chosen_own_main += other.chosen_own_main;
        self.taps += other.taps;
        self.cast += other.cast;
        self.broken += other.broken;
        self.not_castable += other.not_castable;
    }
}

/// The arm's state on one net.
#[derive(Clone, Debug, Default)]
pub struct PayableMenu4 {
    /// Added to each added option's logit, a net that never saw them being
    /// free to read them as anything.
    bias: f32,
    /// Each added option: the cast or the suspend its taps end in, and the taps.
    offered: Vec<(PlayerAction, Plan)>,
    first_added: usize,
    plan: Option<Script4>,
    stats: MenuStats4,
}

/// The spells of `legal.payable` the planner has taps for, each with its plan.
/// A plan with nothing to tap is no reach: the mana is floating, so what keeps
/// the spell out of `castable` is not mana.
pub(super) fn reachable(view: &PlayerView, legal: &LegalActions) -> Vec<(ObjectId, Plan)> {
    if view.owed.is_some() {
        return Vec::new();
    }
    let actor = view.decision_player.unwrap_or(view.seat);
    let Some(pool) = view.seat(actor).map(|seat| seat.mana_pool) else {
        return Vec::new();
    };
    legal
        .payable
        .iter()
        .copied()
        .filter(|id| !legal.castable.contains(id))
        .filter_map(|id| {
            let card = lookup_card_identity(view, id)?;
            if !baylee_ai::spell_can_be_cast_now(view, legal, id, card) {
                return None;
            }
            // Out of a graveyard or exile a card is cast at the price the
            // engine names for each way it may be cast there (a flashback,
            // a disturb face, a foretell cost), whole: the cheapest the
            // planner pays.
            if !baylee_client_core::decision::hand(view)
                .iter()
                .any(|c| c.id == id)
                && legal.cast_ways.iter().any(|(way, _)| *way == id)
            {
                return legal
                    .cast_ways
                    .iter()
                    .filter(|(way, desc)| *way == id && desc.cost.symbols().next().is_some())
                    .filter_map(|(_, desc)| {
                        let price = desc.cost.with_x(u32::from(desc.cost.has_variable()));
                        baylee_ai::plan_for_spell(&price, &pool, view, legal, id, desc.kind)
                    })
                    .filter(|plan| !plan.steps.is_empty())
                    .min_by_key(Plan::taps)
                    .map(|plan| (id, plan));
            }
            let (printed, _) = lookup_spell_cost(view, id)?;
            // An `{X}` spell at its least useful, the tactical layer's reading.
            let printed = printed.with_x(u32::from(printed.has_variable()));
            let price =
                baylee_ai::spell_price(view, id, Some(card), legal, CastModeKind::Normal, printed);
            let plan =
                baylee_ai::plan_for_spell(&price, &pool, view, legal, id, CastModeKind::Normal)?;
            (!plan.steps.is_empty()).then_some((id, plan))
        })
        .collect()
}

/// The cards of `legal.unpaid_suspend` the planner has taps for, each with its
/// plan (`baylee_ai::suspend_plans`, CR 116.2f): suspended once the taps are
/// made, as the language-model seat suspends them.
pub(super) fn suspends(view: &PlayerView, legal: &LegalActions) -> Vec<(ObjectId, Plan)> {
    baylee_ai::suspend_plans(view, legal)
        .into_iter()
        .map(|(card, _, plan)| (card, plan))
        .collect()
}

impl PayableMenu4 {
    /// The cards of the added options whose action `kind` holds.
    fn cards(&self, kind: impl Fn(&PlayerAction) -> bool) -> Vec<ObjectId> {
        self.offered
            .iter()
            .filter(|(action, _)| kind(action))
            .filter_map(|(action, _)| match action {
                PlayerAction::CastSpell { card } | PlayerAction::Suspend { card } => Some(*card),
                _ => None,
            })
            .collect()
    }

    /// The arm with `bias` on the added options' logits.
    #[must_use]
    pub fn new(bias: f32) -> Self {
        Self {
            bias,
            ..Self::default()
        }
    }

    /// Clears what a game left behind; the counts stay until taken.
    pub(super) fn reset(&mut self) {
        self.offered.clear();
        self.plan = None;
    }

    /// The counts since this was last asked.
    pub fn take_stats(&mut self) -> MenuStats4 {
        std::mem::take(&mut self.stats)
    }

    /// The first item the last [`Self::extend`] added: every item before it
    /// is the seat's own menu, every one from it a cast or a suspend the
    /// planner pays for (`search_labels::macro_items` reads them so).
    #[must_use]
    pub const fn first_added(&self) -> usize {
        self.first_added
    }

    /// Appends the reachable spells to a priority menu, encoded by `encode`
    /// exactly as the same spells are with their mana floating. The search
    /// student's root menu is built here and nowhere else, in conversion
    /// and in play.
    pub fn extend(
        &mut self,
        cands: &mut EncodedCandidates4,
        view: &PlayerView,
        pending: &Pending,
        encode: impl FnOnce(&Pending) -> EncodedCandidates4,
    ) {
        self.offered.clear();
        self.first_added = cands.candidates.len();
        let Pending::Priority { player, legal } = pending else {
            return;
        };
        self.offered = reachable(view, legal)
            .into_iter()
            .map(|(card, plan)| (PlayerAction::CastSpell { card }, plan))
            .chain(
                suspends(view, legal)
                    .into_iter()
                    .map(|(card, plan)| (PlayerAction::Suspend { card }, plan)),
            )
            .collect();
        self.offered
            .truncate(MAX_CANDIDATES.saturating_sub(cands.candidates.len()));
        if self.offered.is_empty() {
            return;
        }
        let mut announced = legal.clone();
        announced.castable = self.cards(|action| matches!(action, PlayerAction::CastSpell { .. }));
        announced.suspendable = self.cards(|action| matches!(action, PlayerAction::Suspend { .. }));
        let extra = encode(&Pending::Priority {
            player: *player,
            legal: announced,
        });
        cands
            .candidates
            .extend(extra.candidates.into_iter().filter(|cand| {
                cand.action
                    .as_ref()
                    .is_some_and(|action| self.offered.iter().any(|(added, _)| added == action))
            }));
    }

    /// Applies the bias and counts how the net scored the added options.
    pub(super) fn weigh(&mut self, logits: &mut [f32], cands: &EncodedCandidates4) {
        let n = cands.candidates.len();
        if self.offered.is_empty() || self.first_added >= n || logits.len() < n {
            return;
        }
        for logit in &mut logits[self.first_added..n] {
            *logit += self.bias;
        }
        let logits = &logits[..n];
        let top = logits.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let added = logits[self.first_added..]
            .iter()
            .copied()
            .fold(f32::NEG_INFINITY, f32::max);
        let total: f32 = logits.iter().map(|logit| (logit - top).exp()).sum();
        let share: f32 = logits[self.first_added..]
            .iter()
            .map(|logit| (logit - top).exp())
            .sum::<f32>()
            / total;
        let pass = cands
            .candidates
            .iter()
            .position(|cand| matches!(cand.action, Some(PlayerAction::PassPriority)))
            .map_or(f32::NEG_INFINITY, |index| logits[index]);
        self.stats.menus += 1;
        self.stats.options += (n - self.first_added) as u64;
        self.stats.topped += u64::from(added >= top);
        self.stats.beat_pass += u64::from(added > pass);
        self.stats.mass[((share * 10.0) as usize).min(9)] += 1;
    }

    /// The first tap of the plan for `action`, when it casts an added option.
    pub(super) fn begin(
        &mut self,
        view: &PlayerView,
        action: &PlayerAction,
    ) -> Option<PlayerAction> {
        let (_, plan) = self.offered.iter().find(|(added, _)| added == action)?;
        let (first, script) = Script4::begin(view, plan, action.clone())?;
        let actor = view.decision_player.unwrap_or(view.seat);
        self.stats.chosen += 1;
        if view.active != actor {
            self.stats.chosen_their_turn += 1;
        } else if matches!(view.phase, Phase::FirstMain | Phase::SecondMain) {
            self.stats.chosen_own_main += 1;
        }
        self.stats.taps += 1;
        self.plan = Some(script);
        Some(first)
    }

    /// The plan's next answer, when one stands and this question is its own.
    /// Any other question stays the net's.
    pub(super) fn next(&mut self, view: &PlayerView, pending: &Pending) -> Option<PlayerAction> {
        let answer = self.plan.as_mut()?.next(view, pending);
        if answer.ends() {
            self.plan = None;
        }
        match answer {
            ScriptAnswer4::Tap(action) => {
                self.stats.taps += 1;
                Some(action)
            }
            ScriptAnswer4::Color(action) => Some(action),
            ScriptAnswer4::Then(action) => {
                self.stats.cast += 1;
                Some(action)
            }
            ScriptAnswer4::Broken => {
                self.stats.broken += 1;
                None
            }
            ScriptAnswer4::NotOffered => {
                self.stats.not_castable += 1;
                None
            }
            ScriptAnswer4::Other => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::netplay4::script::ScriptAnswer4;
    use baylee_client_core::test_support::{ViewBuilder, token};
    use baylee_core::ids::{PlayerId, PrintRef};
    use baylee_core::mana::ManaCost;
    use baylee_view::{CardIdentity, HandObject};

    fn identity(name: &str) -> CardIdentity {
        CardIdentity {
            index: baylee_cards::decks::by_name(name).expect(name),
            print: PrintRef::new(0),
            face: 0,
        }
    }

    fn island(slot: u32) -> baylee_view::PublicObject {
        let card = identity("Island");
        let face = &baylee_cards::by_index(card.index).expect("Island").faces[0];
        let mut object = token(slot, 0, "Island", 0, 0);
        object.card = Some(card);
        object.rules = Some(card.into());
        object.types = face.types;
        object.supertypes = face.supertypes;
        object.subtypes = baylee_core::types::SubtypeSet::from_slice(face.subtypes);
        object.power = None;
        object.toughness = None;
        object
    }

    /// Ancestral Vision beside an untapped Island, the engine naming its
    /// suspend with the {U} still in the land (`LegalActions::unpaid_suspend`):
    /// the arm plans the Island's tap, and the script taps it and then, the
    /// {U} floating, suspends (CR 116.2f).
    #[test]
    fn a_suspend_the_lands_pay_is_planned_and_ends_in_the_suspend() {
        let mut view = ViewBuilder::new(2)
            .with_battlefield(0, vec![island(70)])
            .build();
        let vision = ObjectId::new(61, 0);
        view.hand = vec![HandObject {
            id: vision,
            card: identity("Ancestral Vision"),
            name: "Ancestral Vision".into(),
            mana_value: 0,
            colors: baylee_core::color::ColorSet::default(),
            types: baylee_core::types::TypeSet::SORCERY,
            commander: false,
        }];
        let island = ObjectId::new(70, 0);
        let legal = LegalActions {
            can_pass: true,
            mana_abilities: vec![island],
            unpaid_suspend: vec![(vision, ManaCost::parse("{U}"))],
            ..LegalActions::default()
        };
        let planned = suspends(&view, &legal);
        assert_eq!(planned.len(), 1, "{planned:?}");
        let (card, plan) = &planned[0];
        assert_eq!(*card, vision);
        let (first, mut script) =
            Script4::begin(&view, plan, PlayerAction::Suspend { card: vision }).expect("a tap");
        assert_eq!(first, PlayerAction::ActivateManaAbility { source: island });
        let floating = Pending::Priority {
            player: PlayerId::new(0),
            legal: Box::new(LegalActions {
                can_pass: true,
                suspendable: vec![vision],
                ..LegalActions::default()
            }),
        };
        assert_eq!(
            script.next(&view, &floating),
            ScriptAnswer4::Then(PlayerAction::Suspend { card: vision })
        );
        let not_yet = Pending::Priority {
            player: PlayerId::new(0),
            legal: Box::new(LegalActions {
                can_pass: true,
                ..LegalActions::default()
            }),
        };
        let (_, mut script) =
            Script4::begin(&view, plan, PlayerAction::Suspend { card: vision }).expect("a tap");
        assert_eq!(script.next(&view, &not_yet), ScriptAnswer4::NotOffered);
    }

    /// Without the engine's hint there is nothing to plan: the arm never
    /// reads a suspend cost off a card by itself.
    #[test]
    fn no_hint_no_suspend() {
        let view = ViewBuilder::new(2)
            .with_battlefield(0, vec![island(70)])
            .build();
        let legal = LegalActions {
            can_pass: true,
            mana_abilities: vec![ObjectId::new(70, 0)],
            ..LegalActions::default()
        };
        assert!(suspends(&view, &legal).is_empty());
    }
}
