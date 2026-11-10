//! `cards/artifacts/mv_3/disrupting_scepter.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Disrupting Scepter: "{3}, {T}: Target player discards a card. Activate
/// only during your turn." The discarding player chooses which card
/// (`Effect::DiscardForPlayers`), and the ability is withheld outside its
/// controller's own turn.
#[test]
fn disrupting_scepter_makes_a_target_player_discard_only_on_its_controllers_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[disrupting_scepter(), forest(), forest(), forest(), forest()],
        )
        .hand(1, &[swords_to_plowshares(), giant_growth()])
        .start();
    keep_mulligans(&mut engine);

    reach_their_main_phase(&mut engine, p1);
    let scepter = on_battlefield(&engine, p0, disrupting_scepter()).expect("seated");
    assert!(
        !priority_offer(&engine).abilities.contains(&(scepter, 0)),
        "not p0's turn, so the ability is withheld"
    );

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::FirstMain) && e.state().turn.active == p0
    });
    let before = engine.state().zones.list(ZoneLocation::Hand(p1)).len();
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, disrupting_scepter(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a player target, got {:?}", engine.pending())
    };
    assert!(options.is_empty(), "the target is a player, not an object");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("p1 is a legal target");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });

    let Pending::ChooseCards { player, .. } = engine.pending().clone() else {
        panic!(
            "the targeted player discards and chooses which card, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p1, "p1 discards, not p0");
    let hand = engine.state().zones.list(ZoneLocation::Hand(p1)).clone();
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![hand[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        before - 1,
        "one card fewer in p1's hand"
    );
}
