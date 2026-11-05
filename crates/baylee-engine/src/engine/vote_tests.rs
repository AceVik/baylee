//! A vote is cast in turn order, and each ballot is public as it is cast
//! (CR 701.38a, 608.2d): asked as a vote, journaled as `GameEvent::Voted`,
//! and readable through `Engine::ballots` while the next player decides.
use super::testkit::*;
use super::*;
use crate::choice::ChoicePrompt;
use crate::event::GameEvent;
use crate::resolve::Ballot;
use baylee_core::ids::CardIndex;

fn council_s_judgment() -> CardIndex {
    card_index("acf4d685-a718-42f7-9a7a-dee069ae8db2")
}

fn plains() -> CardIndex {
    card_index("bc71ebf6-2056-41f7-be35-b2e5c34afa99")
}

fn sol_ring() -> CardIndex {
    card_index("6ad8011d-3471-4369-9d68-b264cc027487")
}

/// Who is asked now, and that the question is a vote.
#[track_caller]
fn voter(e: &Engine<RegistryLookup>) -> PlayerId {
    let Pending::ChooseCards { player, prompt, .. } = e.pending() else {
        panic!("a vote was expected, got {:?}", e.pending())
    };
    assert_eq!(
        *prompt,
        ChoicePrompt::Vote,
        "the question says it is a vote"
    );
    *player
}

fn votes(e: &Engine<RegistryLookup>, from: usize) -> Vec<(PlayerId, ObjectId)> {
    e.state().journal.entries()[from..]
        .iter()
        .filter_map(|entry| match entry.event {
            GameEvent::Voted { player, object } => Some((player, object)),
            _ => None,
        })
        .collect()
}

/// Three players vote on Council's Judgment. The caster, then each player in
/// turn order, is asked a `Vote`; every ballot is in the journal and in
/// `Engine::ballots` before the next player answers, and the ballots end
/// with the vote.
#[test]
fn each_ballot_is_public_before_the_next_player_votes() {
    let (p0, p1, p2) = (PlayerId::new(0), PlayerId::new(1), PlayerId::new(2));
    let mut e = Duel::table(7103, plains(), 3)
        .battlefield(0, &[plains(), plains(), plains()])
        .battlefield(1, &[sol_ring()])
        .battlefield(2, &[sol_ring()])
        .hand(0, &[council_s_judgment()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let first = on_battlefield(&e, p1, sol_ring()).unwrap();
    let second = on_battlefield(&e, p2, sol_ring()).unwrap();
    cast_from_hand(&mut e, p0, council_s_judgment());
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let from = e.state().journal.entries().len();
    assert!(e.ballots().is_empty(), "nobody has voted yet");

    let mut cast = Vec::new();
    for (expected, choice) in [(p0, first), (p1, second), (p2, first)] {
        assert_eq!(voter(&e), expected, "turn order from the caster");
        assert_eq!(
            e.ballots(),
            cast.as_slice(),
            "every earlier ballot, as this player decides"
        );
        e.apply(
            expected,
            PlayerAction::ChooseObjects {
                objects: vec![choice],
            },
        )
        .expect("a legal ballot");
        cast.push(Ballot {
            voter: expected,
            choice,
        });
        assert_eq!(
            votes(&e, from),
            cast.iter().map(|b| (b.voter, b.choice)).collect::<Vec<_>>(),
            "the ballot is announced as it is cast"
        );
    }
    assert!(e.ballots().is_empty(), "the vote is over");
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(first).unwrap().zone, Zone::Exile);
    assert!(
        on_battlefield(&e, p2, sol_ring()).is_some(),
        "one vote, not the most"
    );
}
