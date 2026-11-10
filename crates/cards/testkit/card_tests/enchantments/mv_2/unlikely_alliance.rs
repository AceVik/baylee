//! `cards/enchantments/mv_2/unlikely_alliance.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::HILL_GIANT;

#[test]
fn unlikely_alliance_pumps_noncombat_creature_and_rejects_attackers() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let card = card_index("f7ac18bc-ea63-4808-b709-92c1e69cf541");
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[card])
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                grizzly_bears(),
                HILL_GIANT,
            ],
        )
        .battlefield(1, &[plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, card);
    pass_until(&mut engine, stack_is_empty);

    let bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let giant = on_battlefield(&engine, p0, HILL_GIANT).unwrap();

    // In main phase, Grizzly Bears is nonattacking and nonblocking: +0/+2 -> 2/4.
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, card, 0);
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, bear), (2, 4));

    // In declare-attackers step, Hill Giant attacks while Grizzly Bears stays back.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(giant, Defender::Player(p1))],
            },
        )
        .unwrap();

    // Activate Unlikely Alliance during combat: attacking Hill Giant cannot be targeted,
    // while nonattacking Grizzly Bears can.
    engine.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::White, 2);
    engine.refresh_offer();
    activate(&mut engine, p0, card, 0);
    let Pending::ChooseTargets { options, .. } = engine.pending() else {
        panic!("expected ChooseTargets, got {:?}", engine.pending());
    };
    assert!(options.contains(&bear));
    assert!(!options.contains(&giant));
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, bear), (2, 6));
    // "Until end of turn": both +0/+2 are gone on the opponent's turn.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(pt(&engine, bear), (2, 2));
}

#[test]
fn unlikely_alliance_rejects_a_blocking_and_an_attacking_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let card = card_index("f7ac18bc-ea63-4808-b709-92c1e69cf541");
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[card, plains(), plains(), grizzly_bears(), HILL_GIANT])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    let bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let giant = on_battlefield(&engine, p0, HILL_GIANT).unwrap();
    let enemy = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    // p1 attacks with its Bears; Hill Giant blocks, Grizzly Bears stays back.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(enemy, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(giant, enemy)],
            },
        )
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, card, 0);
    let Pending::ChooseTargets { options, .. } = engine.pending() else {
        panic!("expected ChooseTargets, got {:?}", engine.pending());
    };
    assert!(options.contains(&bear), "neither attacking nor blocking");
    assert!(!options.contains(&giant), "a blocking creature");
    assert!(!options.contains(&enemy), "an attacking creature");
}
