//! `cards/creatures/mv_5/cockatrice.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Cockatrice — Flying; "Whenever this creature blocks or becomes blocked
/// by a non-Wall creature, destroy that creature at end of combat." The
/// attacker is a 2/3 Hurloon Minotaur, not the 2/2 Pearled Unicorn: it
/// survives the Cockatrice's 2 combat damage on its own, so only the
/// delayed trigger — not the fight — can be what kills it.
#[test]
fn cockatrice_destroys_a_non_wall_creature_it_blocks_at_end_of_combat() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[cockatrice()])
        .battlefield(1, &[hurloon_minotaur()])
        .start();
    keep_mulligans(&mut engine);
    let bird = on_battlefield(&engine, p0, cockatrice()).expect("seated");
    assert!(keywords(&engine, bird).contains(KeywordSet::FLYING));
    reach_their_main_phase(&mut engine, p1);
    let attacker = on_battlefield(&engine, p1, hurloon_minotaur()).expect("seated");

    let blocks = attack_and_collect_blocks(&mut engine, attacker, p0);
    let pairing = blocks
        .iter()
        .find(|b| b.blocker == bird)
        .unwrap_or_else(|| panic!("the Cockatrice may block a ground creature: {blocks:?}"));
    assert!(pairing.attackers.contains(&attacker));
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bird, attacker)],
            },
        )
        .expect("the pairing came out of the list that offered it");
    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::CombatEnd
    });
    assert_eq!(
        engine.state().object(attacker).map(|o| o.zone),
        Some(Zone::Battlefield),
        "2 damage to a 2/3: the fight itself did not kill it"
    );
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    assert!(
        in_graveyard(&engine, p1, hurloon_minotaur()).is_some(),
        "destroyed at end of combat for having been blocked by the Cockatrice, not by the fight"
    );
    assert!(
        on_battlefield(&engine, p0, cockatrice()).is_some(),
        "the Cockatrice itself is untouched"
    );
}

/// The other direction: "…or becomes blocked by a non-Wall creature". The
/// Cockatrice attacks, a Giant Spider (reach, 2/4) is the one thing that can
/// block a flyer, and it survives the Cockatrice's 2 combat damage on its own
/// toughness: only the delayed trigger can be what destroys it.
#[test]
fn cockatrice_destroys_a_non_wall_creature_that_blocks_it() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[cockatrice()])
        .battlefield(1, &[giant_spider()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bird = on_battlefield(&engine, p0, cockatrice()).expect("seated");
    let spider = on_battlefield(&engine, p1, giant_spider()).expect("seated");

    let blocks = attack_and_collect_blocks(&mut engine, bird, p1);
    assert!(
        blocks.iter().any(|b| b.blocker == spider),
        "reach may block a flyer: {blocks:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(spider, bird)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::CombatEnd
    });
    assert_eq!(
        engine.state().object(spider).map(|o| o.zone),
        Some(Zone::Battlefield),
        "2 damage to a 2/4: the fight itself did not kill it"
    );
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    assert!(
        in_graveyard(&engine, p1, giant_spider()).is_some(),
        "destroyed at end of combat for blocking the Cockatrice"
    );
    assert!(on_battlefield(&engine, p0, cockatrice()).is_some());
}

/// The printed exception on the "becomes blocked" side: an Angelic Wall
/// (flying, 1/3, a Wall) blocks the Cockatrice and is still there afterwards.
#[test]
fn cockatrice_leaves_a_wall_that_blocks_it_alone() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[cockatrice()])
        .battlefield(1, &[angelic_wall()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bird = on_battlefield(&engine, p0, cockatrice()).expect("seated");
    let wall = on_battlefield(&engine, p1, angelic_wall()).expect("seated");

    let blocks = attack_and_collect_blocks(&mut engine, bird, p1);
    assert!(
        blocks.iter().any(|b| b.blocker == wall),
        "a flying Wall may block a flyer: {blocks:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(wall, bird)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    assert!(
        on_battlefield(&engine, p1, angelic_wall()).is_some(),
        "2 damage to a 1/3, and the trigger names non-Walls only"
    );
}
