//! Privileged scouting data. These types intentionally have no wire encoding.
//! Only a host may supply them; an agent never receives an engine reference.

use baylee_cards_dsl::{AbilityDef, Effect};
use baylee_core::ids::{CardIndex, PlayerId, SubtypeId};
use baylee_core::types::TypeSet;
use baylee_engine::choice::Pending;
use baylee_view::PlayerView;

use crate::HeuristicAgent;

/// How much library order a trusted host is asked to disclose.
#[derive(Clone, Copy, Debug, Default)]
pub enum LibraryAccess {
    /// No library information.
    #[default]
    None,
    /// At most this many cards, top first.
    Top(u16),
    /// Entire current library, top first.
    All,
}

/// A request from an in-process AI, never a client-supplied authorization.
#[derive(Clone, Copy, Debug, Default)]
pub struct ScoutingRequest {
    /// Include opposing seats as well as the requesting AI's seat.
    pub opponents: bool,
    /// Read current hands.
    pub hands: bool,
    /// Read current library order.
    pub library: LibraryAccess,
    /// Read cards currently outside the game.
    pub sideboards: bool,
}

/// Immutable submitted deck, analysed once at game setup.
pub struct DeckIntel {
    /// Main deck, preserving duplicate card counts but not shuffled order.
    pub cards: Vec<CardIndex>,
    /// Every commander, independently of main-deck cards.
    pub commanders: Vec<CardIndex>,
    pub(crate) creatures: u32,
    pub(crate) cheap_creatures: u32,
    pub(crate) interaction: u32,
    pub(crate) artifacts: u32,
    pub(crate) tribe: Option<SubtypeId>,
}

impl DeckIntel {
    /// Analyse printed properties and effect operations, never deck names.
    #[must_use]
    pub fn new(cards: Vec<CardIndex>, commanders: Vec<CardIndex>) -> Self {
        let mut result = Self {
            cards,
            commanders,
            creatures: 0,
            cheap_creatures: 0,
            interaction: 0,
            artifacts: 0,
            tribe: None,
        };
        let mut tribes: Vec<(SubtypeId, u32)> = Vec::new();
        for &index in result.cards.iter().chain(&result.commanders) {
            let Some(def) = baylee_cards::by_index(index) else {
                continue;
            };
            let Some(face) = def.faces.first() else {
                continue;
            };
            if face.types.contains(TypeSet::CREATURE) {
                result.creatures += 1;
                result.cheap_creatures += u32::from(face.mana_cost.cmc() <= 3);
                for &kind in face.subtypes {
                    if let Some((_, n)) = tribes.iter_mut().find(|(t, _)| *t == kind) {
                        *n += 1;
                    } else {
                        tribes.push((kind, 1));
                    }
                }
            }
            result.artifacts += u32::from(face.types.contains(TypeSet::ARTIFACT));
            result.interaction += u32::from(def.abilities_for_face(0).iter().any(|a| {
                if let AbilityDef::Spell { effects, .. } = a {
                    let m = crate::tactics::meaning(effects, 1);
                    m.removal || m.counter || m.damage > 0
                } else {
                    false
                }
            }));
        }
        result.tribe = tribes
            .into_iter()
            .max_by_key(|&(id, n)| (n, std::cmp::Reverse(id)))
            .map(|(id, _)| id);
        result
    }
}

/// A host's response for one seat. No object handles can be turned into
/// actions on a card the engine never offered.
pub struct ScoutedSeat<'a> {
    /// Whose data this is.
    pub player: PlayerId,
    /// Original deck list and all commanders.
    pub deck: &'a DeckIntel,
    /// Current hand, or no disclosure.
    pub hand: Option<Vec<CardIndex>>,
    /// Current library, top first, or no disclosure.
    pub library: Option<Vec<CardIndex>>,
    /// Current cards outside the game, or no disclosure.
    pub sideboard: Option<Vec<CardIndex>>,
}

/// Information delivered for this decision only. Never retained in an agent
/// that a human could take over, or written into player views or print tables.
pub struct ScoutingReport<'a> {
    /// Authorized seats, in seat order.
    pub seats: Vec<ScoutedSeat<'a>>,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Strategy {
    pub creature_bonus: i64,
    pub interaction_bonus: i64,
    pub draw_bonus: i64,
    pub artifact_bonus: i64,
    pub sweeper_risk: bool,
    pub known_threat: bool,
    pub scouted_opponents: bool,
    pub tribe: Option<SubtypeId>,
}

/// Whether a card's effects contain a board sweeper.
///
/// The descent is [`Effect::branches`]', not this function's. It used to
/// name `Sequence` and `MayDo` and stop there, so a sweeper printed inside a
/// kicker clause or behind "unless you pay" was a sweeper this scout did not
/// see — and a scouted sweeper is what stops the agent committing a third
/// creature.
fn sweeper(effects: &'static [Effect]) -> bool {
    effects.iter().any(|effect| {
        if matches!(
            effect,
            Effect::DestroyAll { .. }
                | Effect::ReturnAllToHand { .. }
                | Effect::PumpFilter {
                    toughness: baylee_cards_dsl::Amount::NegX,
                    ..
                }
        ) {
            return true;
        }
        let (then, otherwise) = effect.branches();
        sweeper(then) || sweeper(otherwise)
    })
}

impl HeuristicAgent {
    /// The stronger levels scout current hands; expert additionally looks
    /// three draws ahead. Other levels request only their own submitted deck.
    #[must_use]
    pub fn scouting_request(&self, pending: &Pending) -> Option<ScoutingRequest> {
        matches!(
            pending,
            Pending::Priority { .. }
                | Pending::ChooseColor { .. }
                | Pending::Mulligan { .. }
                | Pending::ChooseSubtype { .. }
                | Pending::ChooseCards { .. }
        )
        .then_some(ScoutingRequest {
            opponents: self.profile.lookahead > 0,
            hands: self.profile.lookahead > 0,
            library: if self.profile.lookahead > 1 {
                LibraryAccess::Top(3)
            } else {
                LibraryAccess::None
            },
            sideboards: matches!(
                pending,
                Pending::ChooseCards {
                    prompt: baylee_engine::choice::ChoicePrompt::Wish,
                    ..
                }
            ),
        })
    }

    /// Answer with a host-authorized scouting report. Only numerical tactical
    /// summaries live in the temporary controller; the original stays clean.
    #[must_use]
    pub fn act_with_scouting(
        &self,
        view: &PlayerView,
        pending: &Pending,
        context: &baylee_engine::engine::DecisionContext<'_>,
        report: &ScoutingReport<'_>,
    ) -> baylee_engine::choice::PlayerAction {
        let mut informed = self.clone();
        informed.strategy = self.strategy(view, report);
        informed.act_with_context(view, pending, context)
    }

    fn strategy(&self, view: &PlayerView, report: &ScoutingReport<'_>) -> Strategy {
        let mut plan = Strategy::default();
        for seat in &report.seats {
            let deck = seat.deck;
            if seat.player == view.seat {
                plan.tribe = deck.tribe;
                if deck.cheap_creatures * 3 >= u32::try_from(deck.cards.len()).unwrap_or(u32::MAX) {
                    plan.creature_bonus += 200;
                }
                if deck.interaction > deck.creatures {
                    plan.draw_bonus += 200;
                }
                if deck.artifacts > deck.creatures {
                    plan.artifact_bonus += 120;
                }
            } else if self.hostile(seat.player, view.seat) {
                plan.scouted_opponents |= seat.hand.is_some();
                if deck.cheap_creatures > deck.interaction {
                    plan.interaction_bonus += 200;
                } else {
                    plan.draw_bonus += 120;
                }
                for index in seat
                    .hand
                    .iter()
                    .flatten()
                    .chain(seat.library.iter().flatten())
                {
                    if let Some(def) = baylee_cards::by_index(*index) {
                        plan.known_threat |= def
                            .faces
                            .first()
                            .is_some_and(|f| !f.types.contains(TypeSet::LAND));
                        plan.sweeper_risk |= def.abilities_for_face(0).iter().any(|a| match a {
                            AbilityDef::Spell { effects, .. } => sweeper(effects),
                            _ => false,
                        });
                    }
                }
            }
        }
        plan
    }
}

#[cfg(test)]
mod tests {
    use super::sweeper;
    use baylee_cards_dsl::{Amount, Effect, Filter};

    /// A scouted sweeper is what stops the agent committing a third
    /// creature, and the descent is [`Effect::branches`]' rather than a
    /// list written here. Naming `Sequence` and `MayDo` and stopping there
    /// missed every sweeper printed inside a kicker clause or behind
    /// "unless you pay".
    #[test]
    fn a_sweeper_is_found_however_deeply_a_clause_nests_it() {
        const WRATH: Effect = Effect::DestroyAll {
            filter: &Filter::CREATURE,
        };
        const BARE: &[Effect] = &[WRATH];
        const IN_A_SEQUENCE: &[Effect] = &[Effect::Sequence(&[WRATH])];
        const IN_A_MAY: &[Effect] = &[Effect::MayDo { effects: &[WRATH] }];
        const IN_A_KICKER: &[Effect] = &[Effect::IfKicked {
            then: &[WRATH],
            otherwise: &[],
        }];
        const IN_THE_OTHER_HALF: &[Effect] = &[Effect::IfKicked {
            then: &[],
            otherwise: &[Effect::MayDo { effects: &[WRATH] }],
        }];

        for (label, effects) in [
            ("bare", BARE),
            ("a sequence", IN_A_SEQUENCE),
            ("a may-do", IN_A_MAY),
            ("a kicker clause", IN_A_KICKER),
            ("the unkicked half, two deep", IN_THE_OTHER_HALF),
        ] {
            assert!(sweeper(effects), "a sweeper inside {label} was not seen");
        }
    }

    /// And the other direction, which is what makes the first worth
    /// anything: a card that removes one creature is not a board sweeper,
    /// and a pump is one only where the toughness it hands out is negative.
    #[test]
    fn a_card_that_is_not_a_sweeper_is_not_read_as_one() {
        const ONE_AT_A_TIME: &[Effect] = &[Effect::Destroy {
            target: baylee_cards_dsl::TargetSpec::Object(&Filter::CREATURE),
        }];
        const ANTHEM: &[Effect] = &[Effect::PumpFilter {
            filter: &Filter::CREATURE,
            controlled_by: None,
            power: Amount::Fixed(1),
            toughness: Amount::Fixed(1),
            keywords: baylee_cards_dsl::KeywordSet::EMPTY,
            duration: baylee_cards_dsl::Duration::UntilEndOfTurn,
        }];
        const A_PLAGUE: &[Effect] = &[Effect::Sequence(&[Effect::PumpFilter {
            filter: &Filter::CREATURE,
            controlled_by: None,
            power: Amount::NegX,
            toughness: Amount::NegX,
            keywords: baylee_cards_dsl::KeywordSet::EMPTY,
            duration: baylee_cards_dsl::Duration::UntilEndOfTurn,
        }])];

        assert!(!sweeper(&[]), "a card with no effects sweeps nothing");
        assert!(!sweeper(ONE_AT_A_TIME));
        assert!(!sweeper(ANTHEM), "a bonus is not a wrath");
        assert!(sweeper(A_PLAGUE), "and the same shape negated is");
    }
}
