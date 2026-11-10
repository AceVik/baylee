//! `cards/creatures/artifacts/mv_4/skorpekh_destroyer.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn skorpekh_destroyer() -> CardIndex {
    card_index("f96fcb37-5f17-4231-b1a2-64f5fe45730b")
}

/// Attacks with the Destroyer into a Colossus of Sardia (9/9) that blocks it,
/// and plays combat out.
fn destroyer_into_colossus(ring_first: bool) -> Engine<RegistryLookup> {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[skorpekh_destroyer(), forest()])
        .battlefield(1, &[colossus_of_sardia()])
        .hand(0, &[sol_ring()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let destroyer = on_battlefield(&engine, p0, skorpekh_destroyer()).expect("seated");
    let colossus = on_battlefield(&engine, p1, colossus_of_sardia()).expect("seated");
    if ring_first {
        cast_from_hand(&mut engine, p0, sol_ring());
        pass_until(&mut engine, stack_is_empty);
        assert!(keywords(&engine, destroyer).contains(KeywordSet::FIRST_STRIKE));
    }
    let blocks = attack_and_collect_blocks(&mut engine, destroyer, p1);
    assert!(blocks.iter().any(|b| b.blocker == colossus));
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(colossus, destroyer)],
            },
        )
        .expect("the Colossus blocks");
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    engine
}

/// Skorpekh Destroyer — {2}{B}{B} 4/2 with deathtouch. Blocked by a 9/9
/// Colossus it kills it with 4 damage, and dies to the 9 in return.
#[test]
fn skorpekh_destroyer_kills_what_blocks_it_with_deathtouch() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let engine = destroyer_into_colossus(false);
    assert!(
        in_graveyard(&engine, p1, colossus_of_sardia()).is_some(),
        "four deathtouch damage kill a 9/9"
    );
    assert!(
        in_graveyard(&engine, p0, skorpekh_destroyer()).is_some(),
        "and the 4/2 takes the 9"
    );
}

/// "Whenever an artifact you control enters, this creature gains first strike
/// until end of turn." Casting a Sol Ring first gives it first strike, so
/// deathtouch lands before the Colossus strikes and the Destroyer lives.
#[test]
fn skorpekh_destroyer_gains_first_strike_when_an_artifact_enters() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let engine = destroyer_into_colossus(true);
    assert!(in_graveyard(&engine, p1, colossus_of_sardia()).is_some());
    assert!(
        on_battlefield(&engine, p0, skorpekh_destroyer()).is_some(),
        "first strike plus deathtouch: it struck first and was not struck back"
    );
}

/// The artifact has to be the controller's: an artifact entering on the
/// other side gives no first strike.
#[test]
fn skorpekh_destroyer_ignores_the_opponents_artifacts() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[skorpekh_destroyer()])
        .battlefield(1, &[forest()])
        .hand(1, &[sol_ring()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, sol_ring());
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p1, sol_ring()).is_some());
    let destroyer = on_battlefield(&engine, p0, skorpekh_destroyer()).expect("seated");
    assert!(!keywords(&engine, destroyer).contains(KeywordSet::FIRST_STRIKE));
}
