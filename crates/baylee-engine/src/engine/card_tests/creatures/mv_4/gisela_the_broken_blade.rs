//! `cards/creatures/mv_4/gisela_the_broken_blade.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn gisela() -> CardIndex {
    card_index("f3e23d5e-bd88-4e7c-a3fb-db2a8cb05b22")
}

fn bruna() -> CardIndex {
    card_index("bce7e45d-b2c5-43aa-91a6-3a3c24f30850")
}

fn ready(engine: &mut Engine<RegistryLookup>) {
    let turn = engine.state().turn.number;
    pass_until(engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.phase == Phase::FirstMain
            && e.state().turn.number > turn
    });
}

#[test]
fn gisela_first_strike_kills_before_damage_and_lifelink_gains_four() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[gisela()])
        .battlefield(1, &[serra_angel()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    ready(&mut engine);
    let angel = on_battlefield(&engine, P0, gisela()).unwrap();
    let blocker = on_battlefield(&engine, P1, serra_angel()).unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P0),
    );
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(angel, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, angel)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(engine.state.object(angel).unwrap().damage, 0);
    assert_eq!(engine.state.object(blocker).unwrap().zone, Zone::Graveyard);
    assert_eq!(engine.state.players[0].life, 24);
    assert_eq!(engine.state.players[1].life, 20);
}

#[test]
fn gisela_does_not_meld_during_an_opponents_end_step() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(1, &[gisela(), bruna()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let blade = on_battlefield(&engine, P1, gisela()).unwrap();
    let light = on_battlefield(&engine, P1, bruna()).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0 && e.state().turn.step == Step::End
    });
    assert!(stack_is_empty(&engine));
    for id in [blade, light] {
        assert_eq!(engine.state.object(id).unwrap().zone, Zone::Battlefield);
    }
}

#[test]
fn gisela_and_bruna_meld_on_my_end_step_and_brisela_locks_only_opponents() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[gisela(), bruna()])
        .hand(0, &[lightning_bolt()])
        .hand(1, &[lightning_bolt(), gisela()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let blade = on_battlefield(&engine, P0, gisela()).unwrap();
    let light = on_battlefield(&engine, P0, bruna()).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0 && e.state().turn.step == Step::End && !stack_is_empty(e)
    });
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state.merged_components(blade).as_slice(),
        &[blade, light]
    );
    assert_eq!(pt(&engine, blade), (9, 10));
    let keywords = keywords(&engine, blade);
    for keyword in [
        KeywordSet::FLYING,
        KeywordSet::FIRST_STRIKE,
        KeywordSet::VIGILANCE,
        KeywordSet::LIFELINK,
    ] {
        assert!(keywords.contains(keyword));
    }

    engine.state.players[0].mana_pool.add(ManaColor::Red, 1);
    engine.refresh_offer();
    let own_bolt = in_hand(&engine, P0, lightning_bolt()).unwrap();
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority after melding");
    };
    assert!(
        legal.castable.contains(&own_bolt),
        "Brisela never locks its controller"
    );
    reach_their_main_phase(&mut engine, P1);
    // Mana pools empty as steps and phases end; add this in P1's main phase.
    engine.state.players[1].mana_pool.add(ManaColor::Red, 1);
    engine.state.players[1].mana_pool.add(ManaColor::White, 4);
    engine.refresh_offer();
    let bolt = in_hand(&engine, P1, lightning_bolt()).unwrap();
    let four = in_hand(&engine, P1, gisela()).unwrap();
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected opponent priority");
    };
    assert!(!legal.castable.contains(&bolt), "mana value 1 is forbidden");
    assert!(legal.castable.contains(&four), "mana value 4 remains legal");

    engine
        .state
        .move_object(
            blade,
            ZoneLocation::Hand(P0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    for id in [blade, light] {
        assert_eq!(engine.state.object(id).unwrap().zone, Zone::Hand);
        assert!(engine.state.object(id).unwrap().token.is_none());
    }
    assert_eq!(engine.state.object(blade).unwrap().base.power, Some(4));
    assert_eq!(engine.state.object(light).unwrap().base.power, Some(5));
}
