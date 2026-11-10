//! `cards/creatures/artifacts/mv_2/buzz_bots.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn buzz_bots() -> CardIndex {
    card_index("434e720f-2bfa-49b6-a5ac-fe0c0b24764d")
}

/// Buzz Bots: a 1/1 with "Flying, vigilance". Attacking into a Bears and a
/// Storm Crow, only the Crow may block; it stays untapped; one damage dealt.
#[test]
fn buzz_bots_flies_and_attacks_without_tapping() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table(&[], &[buzz_bots()], &[grizzly_bears(), storm_crow()]);
    let bots = on_battlefield(&engine, p0, buzz_bots()).expect("seated");
    assert_eq!(pt(&engine, bots), (1, 1));
    assert!(keywords(&engine, bots).contains(KeywordSet::FLYING));
    assert!(keywords(&engine, bots).contains(KeywordSet::VIGILANCE));
    let bears = on_battlefield(&engine, p1, grizzly_bears()).expect("seated");
    let crow = on_battlefield(&engine, p1, storm_crow()).expect("seated");
    let options = attack_and_collect_blocks(&mut engine, bots, p1);
    assert!(options.iter().any(|o| o.blocker == crow));
    assert!(!options.iter().any(|o| o.blocker == bears));
    assert!(!is_tapped(&engine, bots), "vigilance");
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(life_of(&engine, 1), 19);
}

/// "When this creature dies, draw a card": its controller draws exactly one.
#[test]
fn buzz_bots_draws_a_card_when_it_dies() {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[], &[buzz_bots()], &[]);
    let bots = on_battlefield(&engine, p0, buzz_bots()).expect("seated");
    let hand = hand_count(&engine, 0);
    let library = library_size(&engine, p0);
    let theirs = hand_count(&engine, 1);
    kill(&mut engine, bots);
    assert!(in_graveyard(&engine, p0, buzz_bots()).is_some());
    assert_eq!(hand_count(&engine, 0), hand + 1);
    assert_eq!(library_size(&engine, p0), library - 1);
    assert_eq!(hand_count(&engine, 1), theirs, "only its controller draws");
}
