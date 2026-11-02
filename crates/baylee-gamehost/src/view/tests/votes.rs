//! A vote hides nothing: each ballot is in every seat's view, in a
//! spectator's and in every seat's log as it is cast (CR 701.38a, 608.2d).
use super::*;
use crate::log::GameLog;
use baylee_view::{LogEvent, LogObject};

fn entry(card: CardIndex) -> DeckEntry {
    DeckEntry {
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
        card,
        print: PrintRef::new(0),
    }
}

fn card(oracle_id: &str) -> CardIndex {
    by_oracle_id(oracle_id).expect("in the registry").index
}

/// Council's Judgment at a two-seat table: the caster votes for one of the
/// opponent's Sol Rings, and while the opponent is asked, both seats' views
/// and a spectator's carry that ballot and both seats' logs say it. After
/// the second ballot the vote is over and nothing is left in the views.
#[test]
fn every_seat_and_a_spectator_are_told_each_ballot_as_it_is_cast() {
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let judgment = card("acf4d685-a718-42f7-9a7a-dee069ae8db2");
    let sol_ring = card("6ad8011d-3471-4369-9d68-b264cc027487");
    let mut preset = mixed_print_preset();
    preset.seats[0].starting_hand = Some(vec![entry(judgment)]);
    preset.seats[1].starting_battlefield = vec![entry(sol_ring), entry(sol_ring)];
    let mut engine = Engine::new(&preset, Registry).expect("game starts");
    settle(&mut engine, None);
    let mut log = GameLog::new(engine.state());

    let state = engine.dev_state_mut(me).expect("dev commands");
    state.players[0].mana_pool.add(ManaColor::White, 3);
    engine.refresh_offer();
    let spell = *engine
        .state()
        .zones
        .list(ZoneLocation::Hand(me))
        .iter()
        .find(|id| {
            engine
                .state()
                .object(**id)
                .and_then(|o| o.card)
                .is_some_and(|c| c.index == judgment)
        })
        .expect("Council's Judgment in hand");
    engine
        .apply(me, PlayerAction::CastSpell { card: spell })
        .expect("cast off floating mana");
    for _ in 0..10 {
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            break;
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }
    let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("the caster votes first, got {:?}", engine.pending())
    };
    assert_eq!(player, me);
    for seat in [me, them] {
        assert!(
            seen_by(&engine, seat).ballots.is_empty(),
            "nobody has voted"
        );
    }
    let from = log.len();
    engine
        .apply(
            me,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .expect("a ballot");
    log.consume(engine.state());

    assert!(
        matches!(engine.pending(), Pending::ChooseCards { player, .. } if *player == them),
        "the opponent votes next"
    );
    let cast = vec![baylee_view::Ballot {
        voter: me,
        choice: options[0],
    }];
    for seat in [me, them] {
        assert_eq!(seen_by(&engine, seat).ballots, cast, "seat {seat:?}'s view");
        let told: Vec<LogEvent> = log
            .told(seat, from, log.len())
            .into_iter()
            .map(|line| line.event)
            .collect();
        assert!(
            told.iter().any(|event| matches!(
                event,
                LogEvent::Voted { player, object: LogObject::Known { id, .. } }
                    if *player == me && *id == options[0]
            )),
            "seat {seat:?}'s log says the ballot: {told:?}"
        );
    }
    let watcher = spectator_view(
        engine.state(),
        1,
        None,
        SeatSet::new(),
        SeatSet::new(),
        &[],
        engine.ballots(),
    );
    assert_eq!(watcher.ballots, cast, "a spectator's view");

    engine
        .apply(
            them,
            PlayerAction::ChooseObjects {
                objects: vec![options[1]],
            },
        )
        .expect("a ballot");
    assert!(seen_by(&engine, me).ballots.is_empty(), "the vote is over");
}
