//! Myr Prototype's per-counter attack and block payments.

use super::*;

fn myr_prototype() -> CardIndex {
    card_index("496e0fbb-cb01-41d6-9c56-23e3ae2bb4f7")
}

fn set_two_counters(e: &mut Engine<RegistryLookup>) {
    let p0 = PlayerId::new(0);
    let prototype = on_battlefield(e, p0, myr_prototype()).unwrap();
    e.dev_state_mut(p0)
        .unwrap()
        .object_mut(prototype)
        .unwrap()
        .counters
        .set(CounterKind::P1P1, 2);
    e.refresh_offer();
}

#[test]
fn myr_prototype_must_pay_two_to_attack() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[myr_prototype()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    set_two_counters(&mut e);
    let prototype = on_battlefield(&e, p0, myr_prototype()).unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = e.pending().clone() else {
        unreachable!()
    };
    assert!(!attackers.contains(&prototype));
    assert!(
        e.apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(prototype, Defender::Player(p1))],
            },
        )
        .is_err()
    );

    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[myr_prototype(), forest(), forest()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    set_two_counters(&mut e);
    let prototype = on_battlefield(&e, p0, myr_prototype()).unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    e.apply(
        p0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(prototype, Defender::Player(p1))],
        },
    )
    .unwrap();
    assert!(
        matches!(e.pending(), Pending::Priority { .. }),
        "a payment window opens"
    );
    tap_all_mana(&mut e, p0);
    e.apply(p0, PlayerAction::PassPriority).unwrap();
    assert!(e.state().combat.is_attacking(prototype));
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
}

#[test]
fn myr_prototype_must_pay_two_to_block() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[myr_prototype(), forest(), forest()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    set_two_counters(&mut e);
    let prototype = on_battlefield(&e, p0, myr_prototype()).unwrap();
    let ogre = on_battlefield(&e, p1, gray_ogre()).unwrap();
    reach_their_main_phase(&mut e, p1);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    e.apply(
        p1,
        PlayerAction::DeclareAttackers {
            attackers: vec![(ogre, Defender::Player(p0))],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    e.apply(
        p0,
        PlayerAction::DeclareBlockers {
            blockers: vec![(prototype, ogre)],
        },
    )
    .unwrap();
    tap_all_mana(&mut e, p0);
    e.apply(p0, PlayerAction::PassPriority).unwrap();
    assert!(e.state().combat.is_blocking(prototype, ogre));
}
