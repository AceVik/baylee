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
