//! `cards/artifacts/mv_3/well_of_knowledge.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
fn well_of_knowledge() -> CardIndex {
    card_index("99e2b36b-1fab-4b09-b371-d97a6854dfbc")
}

/// `Well of Knowledge` ({3}, Artifact) lets any player pay `{2}` to draw a card
/// during their own draw step, and does not offer the ability outside their
/// draw step.
#[test]
fn well_of_knowledge_allows_any_player_to_draw_during_their_draw_step() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[well_of_knowledge(), forest(), forest()])
        .battlefield(1, &[forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let well = on_battlefield(&engine, p0, well_of_knowledge()).expect("Well on battlefield");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority");
    };
    assert!(
        !legal.abilities.contains(&(well, 0)),
        "Well of Knowledge cannot be activated during a main phase"
    );

    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::Draw
            && e.state().turn.active == p1
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
    });

    let p1_hand_before = engine.state().zones.list(ZoneLocation::Hand(p1)).len();
    tap_all_mana(&mut engine, p1);
    engine
        .apply(
            p1,
            PlayerAction::ActivateAbility {
                source: well,
                ability_index: 0,
            },
        )
        .expect("opponent activates Well of Knowledge during their draw step");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        p1_hand_before + 1
    );
}
