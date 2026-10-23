//! `cards/sorceries/mv_1/lava_burst.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn lava_burst() -> CardIndex {
    card_index("f7be3da5-55b2-46f2-a5aa-277dee242b94")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Lava Burst (f7be3da5-55b2-46f2-a5aa-277dee242b94) — {X}{R} Sorcery:
/// "Lava Burst deals X damage to any target."
#[test]
fn lava_burst_deals_x_damage_to_any_target() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .hand(0, &[lava_burst()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    tap_all_mana_but(&mut engine, P0, None);
    cast_with_floating(&mut engine, P0, lava_burst());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseNumber { .. })
    });
    engine.apply(P0, PlayerAction::ChooseNumber(3)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 17);
}

#[test]
fn lava_burst_deals_x_damage_to_a_creature_and_spares_the_players() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[lava_burst()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    tap_all_mana_but(&mut engine, P0, None);
    cast_with_floating(&mut engine, P0, lava_burst());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseNumber { .. })
    });
    engine.apply(P0, PlayerAction::ChooseNumber(2)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let options = aim_at(&mut engine, P0, bears);
    assert!(
        options.contains(&bears),
        "a creature is a legal target of Lava Burst"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, P1, grizzly_bears()).is_some(),
        "X = 2 damage kills the 2/2"
    );
    assert_eq!(engine.state().players[1].life, 20);
    assert_eq!(engine.state().players[0].life, 20);
}
