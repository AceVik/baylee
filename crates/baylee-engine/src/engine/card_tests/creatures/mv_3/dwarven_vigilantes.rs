//! `cards/creatures/mv_3/dwarven_vigilantes.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn dwarven_vigilantes() -> CardIndex {
    card_index("b49baf80-237c-41ee-8a22-2b6a68a313e7")
}

#[test]
fn dwarven_vigilantes_unblocked_deals_power_damage_to_target_creature_and_no_combat_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[dwarven_vigilantes()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let opp_bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    engine.state.set_tapped(opp_bear, true);

    cast_from_hand(&mut engine, p0, dwarven_vigilantes());
    pass_until(&mut engine, stack_is_empty);

    let dwarf = on_battlefield(&engine, p0, dwarven_vigilantes()).unwrap();
    assert_eq!(pt(&engine, dwarf), (2, 2));

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    engine.state.set_tapped(opp_bear, true);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(dwarf, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim_at(&mut engine, p0, opp_bear);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    reach_their_main_phase(&mut engine, p1);

    assert!(in_graveyard(&engine, p1, grizzly_bears()).is_some());
    assert_eq!(engine.state().players[1].life, 20);
}
