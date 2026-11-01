//! `cards/creatures/mv_7/wormfang_manta.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn wormfang_manta_skips_your_next_turn_on_etb_and_grants_an_extra_turn_on_ltb() {
    let manta = card_index("17abf8c6-70b6-4877-a0f2-a87b22fb2828");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));

    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
                mountain(),
            ],
        )
        .hand(0, &[manta, lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    for u in all_on_battlefield(&engine, p0, island()) {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: u })
            .unwrap();
    }
    let card = in_hand(&engine, p0, manta).unwrap();
    engine.apply(p0, PlayerAction::CastSpell { card }).unwrap();
    pass_until(&mut engine, stack_is_empty);

    let wm = on_battlefield(&engine, p0, manta).expect("Wormfang Manta on battlefield");
    assert!(keywords(&engine, wm).contains(KeywordSet::FLYING));
    assert_eq!(pt(&engine, wm), (6, 1));

    // P0's next turn is skipped, so after P1's turn ends, P1 gets another turn immediately.
    reach_their_main_phase(&mut engine, p1);
    let p1_turn = engine.state().turn.number;
    pass_until(&mut engine, |e| {
        e.state().turn.number > p1_turn && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert_eq!(
        engine.state().turn.active,
        p1,
        "P0 skipped their next turn, so P1 takes two consecutive turns"
    );

    // Advance back to P0's turn and destroy Wormfang Manta with Lightning Bolt so P0 takes an extra turn after this one.
    reach_their_main_phase(&mut engine, p0);
    let m = all_on_battlefield(&engine, p0, mountain())[0];
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: m })
        .unwrap();
    let bolt = in_hand(&engine, p0, lightning_bolt()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: bolt })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![wm],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let p0_turn = engine.state().turn.number;
    pass_until(&mut engine, |e| {
        e.state().turn.number > p0_turn && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert_eq!(
        engine.state().turn.active,
        p0,
        "leaving the battlefield gave P0 an extra turn immediately after their turn"
    );
}
