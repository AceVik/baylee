//! Miracle tests (CR 702.94): first-of-turn draw offers the miracle
//! cast; accepting casts at the miracle cost.

use super::*;
use baylee_core::ids::{CardIndex, PrintRef};
use baylee_core::preset::{
    AIProfile, DeckEntry, Finish, FormatId, GamePreset, PrintInfo, SeatController, SeatSpec,
};

struct RegistryLookup;
impl CardLookup for RegistryLookup {
    fn card(&self, index: CardIndex) -> Option<&'static baylee_cards_dsl::CardDef> {
        baylee_cards::by_index(index)
    }
}

fn card_index(oracle_id: &str) -> CardIndex {
    baylee_cards::by_oracle_id(oracle_id)
        .expect("card exists")
        .index
}

fn island() -> CardIndex {
    card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
}
fn brainstorm() -> CardIndex {
    card_index("36cd2364-d113-47d1-b2c4-b088d9eb88dd")
}
fn temporal_mastery() -> CardIndex {
    card_index("5c58b8e6-c572-461e-893e-a8c05f20ba17")
}

fn entry(card: CardIndex) -> DeckEntry {
    DeckEntry {
        card,
        print: PrintRef::new(0),
    }
}

/// Deck of 60 Temporal Mastery: every draw is the miracle card.
fn preset(seed: u64, hand0: Vec<CardIndex>, bf0: Vec<CardIndex>) -> GamePreset {
    let deck: Vec<DeckEntry> = (0..60).map(|_| entry(temporal_mastery())).collect();
    let mk = |hand: Vec<CardIndex>, bf: Vec<CardIndex>| SeatSpec {
        controller: SeatController::Ai(AIProfile::default()),
        deck: deck.clone(),
        sideboard: vec![],
        starting_life: None,
        starting_hand: Some(hand.into_iter().map(entry).collect()),
        starting_battlefield: bf.into_iter().map(entry).collect(),
        emblems: vec![],
        team: None,
    };
    GamePreset {
        format: FormatId::Freeform,
        seed,
        dev_mode: false,
        house_rules: HouseRules::default(),
        modifiers: vec![],
        prints: vec![PrintInfo {
            scryfall_id: uuid::Uuid::nil(),
            lang: "EN".into(),
            finish: Finish::Normal,
        }],
        seats: vec![mk(hand0, bf0), mk(vec![], vec![])],
    }
}

fn keep_mulligans(engine: &mut Engine<RegistryLookup>) {
    for _ in 0..2 {
        match engine.pending().clone() {
            Pending::Mulligan { player, .. } => {
                engine.apply(player, PlayerAction::MulliganKeep).unwrap();
            }
            other => panic!("expected mulligan, got {other:?}"),
        }
    }
}

#[test]
#[allow(clippy::too_many_lines)] // scenario script — step-by-step readability
fn first_draw_of_turn_offers_miracle_and_casts_at_miracle_cost() {
    // p0: island on the battlefield, Brainstorm in hand; the deck is all
    // Temporal Mastery, so the first Brainstorm draw is the miracle card.
    let mut engine = Engine::new(
        &preset(5, vec![brainstorm()], vec![island(), island(), island()]),
        RegistryLookup,
    )
    .unwrap();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    let mut guard = 0;
    // Drive to p0 priority, tap the island, cast Brainstorm.
    loop {
        guard += 1;
        assert!(guard < 400, "no brainstorm window");
        match engine.pending().clone() {
            Pending::Priority { player, legal } if player == p0 => {
                if let Some(&source) = legal.mana_abilities.first() {
                    engine
                        .apply(player, PlayerAction::ActivateManaAbility { source })
                        .unwrap();
                    continue;
                }
                if let Some(&card) = legal.castable.iter().find(|id| {
                    engine.state().object(**id).unwrap().card.unwrap().index == brainstorm()
                }) {
                    engine
                        .apply(player, PlayerAction::CastSpell { card })
                        .unwrap();
                    continue;
                }
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player } => {
                engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .unwrap();
            }
            Pending::ChooseBlockers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .unwrap();
            }
            Pending::DiscardChoice { player, count } => {
                let hand: Vec<_> = engine
                    .state()
                    .zones
                    .list(crate::zone::ZoneLocation::Hand(player))
                    .clone();
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: hand[..count as usize].to_vec(),
                        },
                    )
                    .unwrap();
            }
            Pending::ChooseCards {
                player, options, ..
            } => {
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: options[..2.min(options.len())].to_vec(),
                        },
                    )
                    .unwrap();
            }
            Pending::ChooseCastMode { player, .. } => {
                engine.apply(player, PlayerAction::ChooseMode(0)).unwrap();
            }
            Pending::YesNo {
                player,
                prompt: crate::choice::YesNoPrompt::Miracle { .. },
            } if player == p0 => {
                // Accept the miracle: the wizard starts and the spell
                // resolves at the miracle cost {1}{U}.
                engine.apply(player, PlayerAction::YesNo(true)).unwrap();
                break;
            }
            Pending::YesNo { player, .. } => {
                engine.apply(player, PlayerAction::YesNo(false)).unwrap();
            }
            other => panic!("unexpected pending: {other:?}"),
        }
    }
    // The miracle cast resolved: an extra turn is queued and the card is
    // exiled after resolution.
    let mut guard = 0;
    loop {
        guard += 1;
        assert!(guard < 400, "miracle cast never resolved");
        if !engine.state().extra_turns.is_empty() {
            break;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseCards {
                player, options, ..
            } => {
                // Brainstorm put-back: keep the first two.
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: options[..2.min(options.len())].to_vec(),
                        },
                    )
                    .unwrap();
            }
            Pending::ChooseCastMode { player, .. } => {
                engine.apply(player, PlayerAction::ChooseMode(0)).unwrap();
            }
            Pending::YesNo { player, .. } => {
                engine.apply(player, PlayerAction::YesNo(false)).unwrap();
            }
            other => panic!("unexpected pending: {other:?}"),
        }
    }
}

#[test]
fn declining_miracle_keeps_the_card_in_hand() {
    let mut engine = Engine::new(
        &preset(9, vec![brainstorm()], vec![island()]),
        RegistryLookup,
    )
    .unwrap();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    let mut guard = 0;
    loop {
        guard += 1;
        assert!(guard < 400, "no miracle offer");
        match engine.pending().clone() {
            Pending::Priority { player, legal } if player == p0 => {
                if let Some(&source) = legal.mana_abilities.first() {
                    engine
                        .apply(player, PlayerAction::ActivateManaAbility { source })
                        .unwrap();
                    continue;
                }
                if let Some(&card) = legal.castable.iter().find(|id| {
                    engine.state().object(**id).unwrap().card.unwrap().index == brainstorm()
                }) {
                    engine
                        .apply(player, PlayerAction::CastSpell { card })
                        .unwrap();
                    continue;
                }
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player } => {
                engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .unwrap();
            }
            Pending::ChooseBlockers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .unwrap();
            }
            Pending::DiscardChoice { player, count } => {
                let hand: Vec<_> = engine
                    .state()
                    .zones
                    .list(crate::zone::ZoneLocation::Hand(player))
                    .clone();
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: hand[..count as usize].to_vec(),
                        },
                    )
                    .unwrap();
            }
            Pending::ChooseCards {
                player, options, ..
            } => {
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: options[..2.min(options.len())].to_vec(),
                        },
                    )
                    .unwrap();
            }
            Pending::ChooseCastMode { player, .. } => {
                engine.apply(player, PlayerAction::ChooseMode(0)).unwrap();
            }
            Pending::YesNo {
                player,
                prompt: crate::choice::YesNoPrompt::Miracle { card },
            } if player == p0 => {
                engine.apply(player, PlayerAction::YesNo(false)).unwrap();
                // The declined card stays in hand.
                assert!(
                    engine
                        .state()
                        .zones
                        .list(crate::zone::ZoneLocation::Hand(p0))
                        .contains(&card),
                    "declined miracle card stays in hand"
                );
                return;
            }
            Pending::YesNo { player, .. } => {
                engine.apply(player, PlayerAction::YesNo(false)).unwrap();
            }
            other => panic!("unexpected pending: {other:?}"),
        }
    }
}
