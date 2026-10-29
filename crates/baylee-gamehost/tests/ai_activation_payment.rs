//! Deferred X activations must be payable before the house announces them.
use baylee_ai::{AIProfile, HeuristicAgent};
use baylee_core::ids::PlayerId;
use baylee_core::preset::{DeckEntry, GamePreset, SeatController};
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_gamehost::Session;
use baylee_view::Phase;

fn entry(name: &str) -> DeckEntry {
    DeckEntry {
        card: baylee_cards::decks::by_name(name).expect("registered fixture"),
        print: baylee_core::ids::PrintRef::new(0),
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
    }
}

fn position(red: bool) -> Session {
    let mut preset: GamePreset =
        baylee_cards::decks::probe_preset(41, entry("Forest").card).unwrap();
    // The position is seat 0's first main phase; a seeded start could hand
    // the first turn to the Bears instead.
    preset.house_rules.starting_player = Some(0);
    for seat in &mut preset.seats {
        seat.controller = SeatController::Human { user_id: 0 };
        seat.starting_hand = Some(Vec::new());
        seat.starting_battlefield.clear();
    }
    preset.seats[0].starting_battlefield = [
        "Kessig Wolf Run",
        "Forest",
        if red { "Mountain" } else { "Swamp" },
        "Ulvenwald Observer",
    ]
    .map(entry)
    .to_vec();
    preset.seats[1].starting_battlefield = vec![entry("Grizzly Bears")];
    preset.seats[1].starting_life = Some(4);
    let mut session = Session::new(&preset).unwrap();
    let me = PlayerId::new(0);
    for _ in 0..100 {
        let pending = session.pending().clone();
        let player = pending.asked().expect("setup remains playable");
        if let Pending::Priority { player, ref legal } = pending
            && player == me
            && session.observe_view(me).phase == Phase::FirstMain
        {
            if let Some(&card) = legal.lands.first() {
                session
                    .act_slice(me, PlayerAction::PlayLand { card }, 0)
                    .unwrap();
                continue;
            }
            return session;
        }
        let action = match pending {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::Priority { .. } => PlayerAction::PassPriority,
            other => panic!("unexpected setup choice: {other:?}"),
        };
        session.act_slice(player, action, 0).unwrap();
    }
    panic!("first main phase was not reached");
}

#[test]
fn kessig_without_red_is_not_announced_and_reversed_forever() {
    // Actual Sharp cap: round 8, six seats, seed 4840001. The house repeated
    // Wolf Run -> X=0 -> target -> pass payment, returning to the same offer.
    // This smaller real Session earns that offer without a fabricated view.
    let mut session = position(false);
    let me = PlayerId::new(0);
    let (pending, view) = session.view_for(me).unwrap();
    let source = view
        .battlefield
        .iter()
        .find(|o| o.name == "Kessig Wolf Run")
        .unwrap()
        .id;
    let Pending::Priority { ref legal, .. } = pending else {
        panic!("expected priority");
    };
    assert!(
        legal.abilities.contains(&(source, 1)),
        "X is announced before making mana"
    );
    let agent = HeuristicAgent::new(AIProfile::SHARP);
    assert_eq!(agent.act(&view, &pending), PlayerAction::PassPriority);
    // Passing a short real payment window restores the exact original offer.
    let before = session.state().snapshot_hash();
    let target = view
        .battlefield
        .iter()
        .find(|o| o.name == "Ulvenwald Observer")
        .unwrap()
        .id;
    for action in [
        PlayerAction::ActivateAbility {
            source,
            ability_index: 1,
        },
        PlayerAction::ChooseNumber(0),
        PlayerAction::ChooseObjects {
            objects: vec![target],
        },
    ] {
        session.act_slice(me, action, 0).unwrap();
    }
    let (pending, view) = session.view_for(me).unwrap();
    assert_eq!(
        view.owed,
        Some(baylee_core::mana::ManaPayment::Fixed(
            baylee_core::mana::ManaCost::parse("{R}{G}")
        ))
    );
    let action = agent.act(&view, &pending);
    assert_eq!(action, PlayerAction::PassPriority);
    session.act_slice(me, action, 0).unwrap();
    assert_eq!(session.state().snapshot_hash(), before);
    let (pending, view) = session.view_for(me).unwrap();
    assert_eq!(agent.act(&view, &pending), PlayerAction::PassPriority);
}

#[test]
fn kessig_with_red_and_green_still_pays_its_deferred_window() {
    let mut session = position(true);
    let me = PlayerId::new(0);
    let agent = HeuristicAgent::new(AIProfile::SHARP);
    let source = session
        .observe_view(me)
        .battlefield
        .iter()
        .find(|o| o.name == "Kessig Wolf Run")
        .unwrap()
        .id;
    let mut began = false;
    for _ in 0..16 {
        let (pending, view) = session.view_for(me).expect("activator keeps its choices");
        if began && !view.stack.is_empty() {
            assert!(
                view.battlefield
                    .iter()
                    .find(|o| o.id == source)
                    .unwrap()
                    .status
                    .is_tapped()
            );
            return;
        }
        let action = agent.act(&view, &pending);
        if !began {
            assert_eq!(
                action,
                PlayerAction::ActivateAbility {
                    source,
                    ability_index: 1
                }
            );
            began = true;
        }
        session.act_slice(me, action, 0).unwrap();
    }
    panic!("payable activation did not reach the stack");
}
