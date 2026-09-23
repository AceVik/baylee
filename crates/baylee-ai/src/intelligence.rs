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
                | Effect::DealDamageEach { .. }
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
    use super::{DeckIntel, sweeper};
    use baylee_cards_dsl::{Amount, Effect, Filter};
    use baylee_core::ids::{CardIndex, SubtypeId};

    /// A card by the name it prints, which is the only handle a test should
    /// use for one — an index would pin this test to a ledger row rather
    /// than to a card.
    fn card(name: &str) -> CardIndex {
        baylee_cards::decks::by_name(name).unwrap_or_else(|| panic!("the pool has {name}"))
    }

    fn intel(cards: &[&str]) -> DeckIntel {
        DeckIntel::new(cards.iter().copied().map(card).collect(), vec![])
    }

    /// **Every count is per card, read off the printing, and counts a card
    /// once.**
    ///
    /// Royal Assassin is the case that says what `interaction` means: it
    /// destroys a creature and is not interaction, because the sentence is an
    /// *activated ability* and the field counts what a card does when it is
    /// cast. An agent that read it as removal would hold it up as an answer
    /// to a threat that is already resolving.
    ///
    /// Serra Angel says what `cheap_creatures` means from the other side, and
    /// Sol Ring is a card that is counted by exactly one of these fields.
    #[test]
    fn a_deck_is_counted_by_what_its_cards_print() {
        static DECK: [&str; 6] = [
            "Lightning Bolt",
            "Counterspell",
            "Llanowar Elves",
            "Serra Angel",
            "Sol Ring",
            "Royal Assassin",
        ];
        let one = intel(&DECK);

        assert_eq!(
            one.creatures, 3,
            "Llanowar Elves, Serra Angel, Royal Assassin"
        );
        assert_eq!(
            one.cheap_creatures, 2,
            "and Serra Angel at five is not one of them"
        );
        assert_eq!(one.artifacts, 1, "Sol Ring, which is no creature");
        assert_eq!(
            one.interaction, 2,
            "Lightning Bolt and Counterspell — Royal Assassin destroys a \
             creature with an activated ability, which is not what a card \
             does when it is cast"
        );

        // Duplicates are kept, so a deck twice the size counts twice. The
        // submitted list is not deduplicated anywhere on the way in.
        let twice = DeckIntel::new(
            DECK.iter().chain(&DECK).copied().map(card).collect(),
            vec![],
        );
        assert_eq!(
            (
                twice.creatures,
                twice.cheap_creatures,
                twice.artifacts,
                twice.interaction
            ),
            (6, 4, 2, 4)
        );
    }

    /// **A commander is analysed beside the deck and not inside it.**
    ///
    /// It is never in the main-deck list — a commander starts in the command
    /// zone — so a reader that walked `cards` alone would say a Commander
    /// deck's general is not a creature it has.
    #[test]
    fn a_commander_is_counted_although_it_is_in_no_deck_list() {
        let general = DeckIntel::new(vec![], vec![card("Serra Angel")]);
        assert_eq!(general.creatures, 1);
        assert_eq!(general.cheap_creatures, 0, "five mana is not cheap");
        assert!(
            general.cards.is_empty() && general.commanders.len() == 1,
            "and the two lists stay apart"
        );
    }

    /// **The tribe is the commonest creature type, and a tie is broken the
    /// same way whatever order the deck was submitted in.**
    ///
    /// That second half is the whole reason the tie-break exists.
    /// `max_by_key` answers with the *last* maximum, so a rule of "the most
    /// frequent wins" alone would make the tribe a function of deck order —
    /// and a deck list that arrives shuffled differently would give the agent
    /// a different plan for the same sixty cards. `Reverse(id)` makes the
    /// lowest subtype id win a tie, which is an answer the submission order
    /// cannot move.
    #[test]
    fn the_tribe_is_the_commonest_type_and_a_tie_does_not_depend_on_deck_order() {
        use baylee_core::generated::subtypes::creature::{BIRD, DRUID, ELF};

        let outright = intel(&["Birds of Paradise", "Birds of Paradise", "Llanowar Elves"]);
        assert_eq!(outright.tribe, Some(BIRD), "two Birds against one of each");

        let reversed = intel(&["Llanowar Elves", "Birds of Paradise", "Birds of Paradise"]);
        assert_eq!(reversed.tribe, outright.tribe);

        // One of each: Bird, Elf and Druid all at one. The lowest id wins,
        // read here as the minimum rather than typed out, because which of
        // the three it is belongs to the generated subtype table and not to
        // this test.
        let lowest = [BIRD, ELF, DRUID].into_iter().min().expect("three of them");
        for deck in [
            ["Birds of Paradise", "Llanowar Elves"],
            ["Llanowar Elves", "Birds of Paradise"],
        ] {
            assert_eq!(
                intel(&deck).tribe,
                Some(lowest),
                "a tie is broken by the id and never by the order"
            );
        }

        assert_eq!(
            intel(&["Lightning Bolt", "Sol Ring"]).tribe,
            None::<SubtypeId>,
            "a deck with no creature in it has no tribe"
        );
    }

    /// A scouted sweeper is what stops the agent committing a third
    /// creature, and the descent is [`Effect::branches`]' rather than a
    /// list written here. Naming `Sequence` and `MayDo` and stopping there
    /// missed every sweeper printed inside a kicker clause or behind
    /// "unless you pay".
    #[test]
    fn a_sweeper_is_found_however_deeply_a_clause_nests_it() {
        const WRATH: Effect = Effect::destroy_all(&Filter::CREATURE);
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
        const ONE_AT_A_TIME: &[Effect] = &[Effect::destroy(baylee_cards_dsl::TargetSpec::Object(
            &Filter::CREATURE,
        ))];
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

    /// Damage to each creature is a sweep, bare or inside a sequence.
    ///
    /// A guard on the list and nothing more: `sweeper` reads a spell's
    /// effects, and the two finished cards that print the sentence carry it
    /// on a triggered and an activated ability, so no card in the pool
    /// reaches this arm yet.
    #[test]
    fn a_damage_sweep_is_named_by_the_sweeper_list() {
        const BLAST: &[Effect] = &[Effect::damage_each(3, &Filter::CREATURE)];
        const IN_A_SEQUENCE: &[Effect] = &[Effect::Sequence(BLAST)];
        assert!(sweeper(BLAST), "damage to each creature is a sweep");
        assert!(
            sweeper(IN_A_SEQUENCE),
            "and so is the same inside a sequence"
        );
    }
}
