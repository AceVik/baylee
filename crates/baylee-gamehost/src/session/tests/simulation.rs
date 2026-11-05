//! `Session::fork_for`: a simulation that plays as the table plays and
//! leaves the table as it was.
use super::*;
use baylee_engine::engine::{DeckKnowledge, ForkRefusal};
use baylee_view::Phase;

const SEARCHER: PlayerId = PlayerId::new(0);
const POLICY_SEED: u64 = 0x5eed_0ff0_4c4c;

/// Allytifact (seat 0, taken over) against Victory (the house), recorded.
fn table(seed: u64) -> Session {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/acceptance-decks.txt"
    ))
    .expect("acceptance deck file");
    let allytifact =
        baylee_cards::decks::load_acceptance(&text, "Allytifact").expect("Allytifact loads");
    let victory = baylee_cards::decks::load_acceptance(&text, "Victory").expect("Victory loads");
    let preset = baylee_cards::decks::preset_for(seed, &allytifact, &victory);
    let mut session = Session::new_recorded(&preset, "test").expect("the table builds");
    session.describe("sim".into(), vec!["A".into(), "B".into()]);
    assert!(session.take_over(SEARCHER));
    session
}

/// Plays the driven seat with the default house answer until it holds
/// priority in a main phase of turn `turn` or later that a fork accepts.
fn to_forkable_main(session: &mut Session, turn: u32) {
    for _ in 0..4000 {
        assert!(
            !matches!(session.pending(), Pending::GameOver(_)),
            "the game ended before a forkable main phase"
        );
        if session.needs_pump() {
            session.pump_at_most(1);
            continue;
        }
        let (pending, view) = session.view_for(SEARCHER).expect("the searcher is asked");
        if matches!(pending, Pending::Priority { .. })
            && view.turn >= turn
            && view.active == SEARCHER
            && matches!(view.phase, Phase::FirstMain)
            && session
                .fork_for(SEARCHER, DeckKnowledge::Known, POLICY_SEED)
                .is_ok()
        {
            return;
        }
        let action = session.house_action(SEARCHER).expect("an answer");
        session
            .act_slice(SEARCHER, action, 0)
            .expect("the answer stands");
    }
    panic!("no forkable main phase within the step budget");
}

/// Plays a fork for `steps` actions, the driven seat by the house's
/// answer, and returns the hash after each.
fn play_fork(fork: &mut SimulationSession, steps: usize) -> Vec<u64> {
    let mut hashes = Vec::new();
    for _ in 0..steps {
        if matches!(fork.pending(), Pending::GameOver(_)) {
            break;
        }
        if fork.needs_pump() {
            assert_eq!(fork.pump_at_most(1), 1);
        } else {
            let action = fork.house_action(SEARCHER).expect("the searcher is asked");
            fork.act_slice(SEARCHER, action, 0)
                .expect("the house's answer stands");
        }
        hashes.push(fork.snapshot_hash());
    }
    hashes
}

/// Plays the rest of the game the same way and returns its record.
fn finish(session: &mut Session) -> Vec<u8> {
    for _ in 0..20_000 {
        if matches!(session.pending(), Pending::GameOver(_)) {
            break;
        }
        if session.needs_pump() {
            session.pump_at_most(1);
            continue;
        }
        let action = session.house_action(SEARCHER).expect("an answer");
        session
            .act_slice(SEARCHER, action, 0)
            .expect("the answer stands");
    }
    session.take_record()
}

#[test]
fn forking_and_playing_a_fork_leave_the_table_and_its_record_as_they_were() {
    let mut searched = table(31);
    let mut plain = table(31);
    to_forkable_main(&mut searched, 3);
    to_forkable_main(&mut plain, 3);
    let hash = searched.snapshot_hash();
    let recorded = searched.record_pending();
    let seq = (searched.seq(), searched.decision_seq());

    for particle in 0..3 {
        let mut fork = searched
            .fork_for(SEARCHER, DeckKnowledge::Known, POLICY_SEED + particle)
            .expect("the root forks");
        assert_eq!(fork.decision_seq(), searched.decision_seq());
        assert!(!play_fork(&mut fork, 300).is_empty());
    }

    assert_eq!(searched.snapshot_hash(), hash);
    assert_eq!(searched.record_pending(), recorded);
    assert_eq!((searched.seq(), searched.decision_seq()), seq);
    assert_eq!(
        searched.snapshot_hash(),
        plain.snapshot_hash(),
        "twin tables agree before the rest is played"
    );
    let searched_record = finish(&mut searched);
    let plain_record = finish(&mut plain);
    assert!(!plain_record.is_empty());
    assert_eq!(
        searched_record, plain_record,
        "the forked table records exactly what its twin records"
    );
}

#[test]
fn a_fork_is_a_function_of_the_table_and_its_policy_seed() {
    let mut session = table(32);
    to_forkable_main(&mut session, 3);
    let fork = |seed| {
        let mut fork = session
            .fork_for(SEARCHER, DeckKnowledge::Known, seed)
            .expect("the root forks");
        play_fork(&mut fork, 200)
    };
    assert_eq!(fork(POLICY_SEED), fork(POLICY_SEED), "same seed, same play");
    assert_ne!(
        fork(POLICY_SEED),
        fork(POLICY_SEED ^ 1),
        "another seed deals and plays another world"
    );
}

#[test]
fn a_fork_keeps_driven_chairs_and_reseeds_the_house_from_the_policy_seed() {
    let mut session = table(33);
    to_forkable_main(&mut session, 2);
    let mut fork = session
        .fork_for(SEARCHER, DeckKnowledge::Known, POLICY_SEED)
        .expect("the root forks");
    assert_eq!(fork.seat(), SEARCHER);
    assert!(matches!(
        fork.seat_kind(SEARCHER),
        Some(SeatKind::Driven(_))
    ));
    let house = PlayerId::new(1);
    assert!(matches!(fork.seat_kind(house), Some(SeatKind::Ai(_))));
    let debug = |kind: Option<&SeatKind>| format!("{kind:?}");
    assert_ne!(
        debug(fork.seat_kind(house)),
        debug(session.seat_kind(house)),
        "the fork's house plays on the policy seed's stream, not the table's"
    );
    let again = session
        .fork_for(SEARCHER, DeckKnowledge::Known, POLICY_SEED)
        .expect("the root forks");
    assert_eq!(debug(fork.seat_kind(house)), debug(again.seat_kind(house)));

    // The searcher is asked, so nothing is owed by the house until the
    // driven chair is handed back to the agent it kept.
    assert!(!fork.needs_pump());
    assert!(fork.view_for(SEARCHER).is_some());
    assert!(!fork.release(house), "only a driven chair is released");
    assert!(fork.release(SEARCHER));
    assert!(matches!(fork.seat_kind(SEARCHER), Some(SeatKind::Ai(_))));
    assert!(fork.needs_pump());
    assert_eq!(fork.pump_at_most(5), 5);
    assert!(
        matches!(session.seat_kind(SEARCHER), Some(SeatKind::Driven(_))),
        "releasing a fork's chair leaves the table's driven"
    );
}

#[test]
fn a_fork_takes_only_legal_answers_from_outside_and_only_for_driven_chairs() {
    let mut session = table(34);
    to_forkable_main(&mut session, 2);
    let mut fork = session
        .fork_for(SEARCHER, DeckKnowledge::Known, POLICY_SEED)
        .expect("the root forks");
    let before = fork.snapshot_hash();
    let house = PlayerId::new(1);
    assert!(
        fork.act_slice(house, PlayerAction::PassPriority, 0)
            .is_err()
    );
    assert!(
        fork.act_slice(SEARCHER, PlayerAction::MulliganKeep, 0)
            .is_err()
    );
    assert_eq!(
        fork.snapshot_hash(),
        before,
        "a refused answer moves nothing"
    );
    let seq = fork.decision_seq();
    let answered = fork
        .act_slice(SEARCHER, PlayerAction::PassPriority, 3)
        .expect("passing is legal");
    assert!(answered <= 3);
    assert!(fork.decision_seq() > seq);
}

#[test]
fn the_opening_hands_and_an_unknown_deck_are_refused() {
    let session = table(35);
    assert_eq!(
        session
            .fork_for(SEARCHER, DeckKnowledge::Known, POLICY_SEED)
            .err(),
        Some(ForkRefusal::OpeningHands)
    );
    let mut session = table(35);
    to_forkable_main(&mut session, 2);
    assert_eq!(
        session
            .fork_for(SEARCHER, DeckKnowledge::Unknown, POLICY_SEED)
            .err(),
        Some(ForkRefusal::UnknownDecks)
    );
}

#[test]
fn a_fork_shows_its_seat_what_the_table_shows_it() {
    let mut session = table(36);
    to_forkable_main(&mut session, 3);
    let fork = session
        .fork_for(SEARCHER, DeckKnowledge::Known, POLICY_SEED)
        .expect("the root forks");
    let (pending, view) = session.view_for_with_context(SEARCHER).expect("asked");
    let (fork_pending, fork_view) = fork.view_for_with_context(SEARCHER).expect("asked");
    // The seat's own hand and the public board keep their objects; what
    // the seat could not see was dealt again under new ids, so compare
    // what it can see.
    assert_eq!(fork_view.hand.len(), view.hand.len());
    assert_eq!(
        fork_view
            .hand
            .iter()
            .map(|card| card.id)
            .collect::<Vec<_>>(),
        view.hand.iter().map(|card| card.id).collect::<Vec<_>>()
    );
    assert_eq!(fork_view.battlefield.len(), view.battlefield.len());
    assert_eq!(fork_view.seats.len(), view.seats.len());
    for (a, b) in fork_view.seats.iter().zip(&view.seats) {
        assert_eq!((a.life, a.hand_count), (b.life, b.hand_count));
    }
    assert_eq!(
        format!("{fork_pending:?}"),
        format!("{pending:?}"),
        "the searcher is offered what the table offers it"
    );
    assert_eq!(
        fork.observe_view(SEARCHER).turn,
        session.observe_view(SEARCHER).turn
    );
}
