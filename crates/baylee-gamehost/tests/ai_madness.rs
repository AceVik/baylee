//! Madness answered by the house across the real engine/view boundary.

// Looks cards up, so this program holds the pool (`baylee-cards-pool`).
use baylee_ai::{AIProfile, HeuristicAgent, pending_player};
use baylee_cards_pool as _;
use baylee_core::ids::{ObjectId, PlayerId};
use baylee_core::preset::DeckEntry;
use baylee_engine::choice::{Pending, PlayerAction, YesNoPrompt};
use baylee_engine::engine::Engine;
use baylee_engine::state::GameState;
use baylee_engine::zone::{Zone, ZoneLocation};
use baylee_gamehost::{RegistryLookup, SeatContext, owed_payment, player_view};

const ME: PlayerId = PlayerId::new(0);

fn entry(name: &str) -> DeckEntry {
    DeckEntry {
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
        card: baylee_cards::decks::by_name(name).expect("a pool card"),
        print: baylee_core::ids::PrintRef::new(0),
    }
}

fn named(state: &GameState, zone: ZoneLocation, name: &str) -> ObjectId {
    state
        .zones
        .list(zone)
        .iter()
        .copied()
        .find(|id| {
            state
                .object(*id)
                .and_then(|object| object.card)
                .is_some_and(|card| {
                    baylee_cards::by_index(card.index).is_some_and(|def| def.faces[0].name == name)
                })
        })
        .unwrap_or_else(|| panic!("{name} is in {zone:?}"))
}

/// Reach seat 0's first main phase without making a game decision for it.
fn settle(engine: &mut Engine<RegistryLookup>) {
    for seq in 0..100 {
        let pending = engine.pending().clone();
        let seat = pending_player(&pending).expect("a live game");
        let action = match pending {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::Priority { .. } if seat != ME => PlayerAction::PassPriority,
            Pending::Priority { .. } => {
                let view = player_view(
                    engine.state(),
                    ME,
                    seq,
                    Some(engine.pending()),
                    &SeatContext {
                        awaiting: Some(ME),
                        ..Default::default()
                    },
                    &[],
                );
                if view.stack.is_empty()
                    && view.active == ME
                    && view.phase == baylee_view::Phase::FirstMain
                {
                    return;
                }
                PlayerAction::PassPriority
            }
            other => panic!("unexpected setup question: {other:?}"),
        };
        engine.apply(seat, action).expect("a legal setup answer");
    }
    panic!("seat 0 never reached its main phase");
}

fn discarded_rootwalla_game() -> (Engine<RegistryLookup>, ObjectId) {
    let mut preset = baylee_cards::decks::probe_preset(41, entry("Forest").card).unwrap();
    preset.house_rules.starting_player = Some(0);
    for seat in &mut preset.seats {
        seat.starting_hand = Some(vec![]);
        seat.starting_battlefield.clear();
    }
    preset.seats[0].starting_hand = Some(vec![
        entry("Basking Rootwalla"),
        entry("Prying Eyes"),
        entry("Forest"),
    ]);
    preset.seats[0].starting_battlefield = vec![entry("Island"); 6];
    let mut engine = Engine::new(&preset, RegistryLookup).unwrap();
    settle(&mut engine);

    let rootwalla = named(engine.state(), ZoneLocation::Hand(ME), "Basking Rootwalla");
    let prying = named(engine.state(), ZoneLocation::Hand(ME), "Prying Eyes");
    let islands: Vec<_> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine.state().object(*id).is_some_and(|object| {
                object.controller == ME
                    && object.card.is_some_and(|card| {
                        baylee_cards::by_index(card.index)
                            .is_some_and(|def| def.faces[0].name == "Island")
                    })
            })
        })
        .collect();
    assert_eq!(islands.len(), 6);
    for source in islands {
        engine
            .apply(ME, PlayerAction::ActivateManaAbility { source })
            .expect("an Island taps for mana");
    }
    engine
        .apply(ME, PlayerAction::CastSpell { card: prying })
        .expect("Prying Eyes is payable from the floating mana");

    let mut discarded = false;
    for _ in 0..100 {
        let pending = engine.pending().clone();
        let seat = pending_player(&pending).expect("a live game");
        let action = match pending {
            Pending::Priority { .. } => PlayerAction::PassPriority,
            Pending::ChooseCards {
                options,
                min: 2,
                max: 2,
                ..
            } if seat == ME => {
                assert!(options.contains(&rootwalla));
                let other = options
                    .iter()
                    .copied()
                    .find(|id| *id != rootwalla)
                    .expect("another card to discard");
                discarded = true;
                PlayerAction::ChooseObjects {
                    objects: vec![rootwalla, other],
                }
            }
            other => panic!("unexpected discard setup question: {other:?}"),
        };
        engine.apply(seat, action).expect("a legal discard answer");
        if discarded {
            break;
        }
    }
    assert!(discarded, "Prying Eyes asked for its two discards");
    (engine, rootwalla)
}

#[test]
fn the_house_casts_a_discarded_rootwalla_for_its_free_madness_cost() {
    let (mut engine, rootwalla) = discarded_rootwalla_game();
    let agent = HeuristicAgent::new(AIProfile::EXPERT);
    let mut answered = false;
    for seq in 0..100 {
        if engine
            .state()
            .object(rootwalla)
            .is_some_and(|object| object.zone == Zone::Battlefield && object.controller == ME)
        {
            assert!(answered, "the house answered the actual madness prompt");
            assert_eq!(engine.state().players[0].mana_pool.total(), 0);
            assert!(
                !engine
                    .state()
                    .zones
                    .list(ZoneLocation::Graveyard(ME))
                    .contains(&rootwalla)
            );
            return;
        }
        let pending = engine.pending().clone();
        let seat = pending_player(&pending).expect("a live game");
        let action = match pending {
            Pending::Priority { .. } => PlayerAction::PassPriority,
            Pending::YesNo {
                prompt: YesNoPrompt::Madness { card },
                ..
            } if seat == ME && card == rootwalla => {
                let context = SeatContext {
                    awaiting: Some(seat),
                    owed: owed_payment(&engine),
                    ..Default::default()
                };
                let view = player_view(
                    engine.state(),
                    seat,
                    seq,
                    Some(engine.pending()),
                    &context,
                    &[],
                );
                assert_eq!(
                    view.exile
                        .iter()
                        .flatten()
                        .find(|object| object.id == rootwalla)
                        .map(|object| object.name.as_str()),
                    Some("Basking Rootwalla")
                );
                assert_eq!(view.owed, None, "the choice precedes the payment window");
                let action =
                    agent.act_with_context(&view, engine.pending(), &engine.decision_context());
                assert_eq!(action, PlayerAction::YesNo(true));
                answered = true;
                action
            }
            other => panic!("unexpected madness question: {other:?}"),
        };
        engine
            .apply(seat, action)
            .expect("the projected answer is legal");
    }
    panic!("the house never put the madness Rootwalla onto the battlefield");
}
