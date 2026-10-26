//! `cards/creatures/mv_4/lesser_gargadon.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn lesser_gargadon() -> CardIndex {
    card_index("7e162cc1-9482-4812-8897-209ec2927ec0")
}

/// Lesser Gargadon ({2}{R}{R} 6/4 Creature — Beast):
/// "Whenever this creature attacks or blocks, sacrifice a land."
#[test]
fn lesser_gargadon_sacrifices_a_land_when_attacking() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[lesser_gargadon(), mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let gargadon = on_battlefield(&engine, p0, lesser_gargadon()).unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(gargadon, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. }) || stack_is_empty(e)
    });
    if let Pending::ChooseCards { options, .. } = engine.pending().clone() {
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![options[0]],
                },
            )
            .unwrap();
    }
    reach_their_main_phase(&mut engine, p1);

    assert_eq!(all_on_battlefield(&engine, p0, mountain()).len(), 1);
    assert!(in_graveyard(&engine, p0, mountain()).is_some());
    assert_eq!(engine.state().players[1].life, 14);
}

/// Lesser Gargadon also triggers and forces its controller to sacrifice a land
/// when it blocks.
#[test]
fn lesser_gargadon_sacrifices_a_land_when_blocking() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[lesser_gargadon(), mountain()])
        .battlefield(1, &[index::HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    let attacker = on_battlefield(&engine, p1, index::HILL_GIANT).unwrap();
    let gargadon = on_battlefield(&engine, p0, lesser_gargadon()).unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(p0))],
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
                blockers: vec![(gargadon, attacker)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. }) || stack_is_empty(e)
    });
    if let Pending::ChooseCards { options, .. } = engine.pending().clone() {
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![options[0]],
                },
            )
            .unwrap();
    }
    reach_their_main_phase(&mut engine, p0);

    assert!(on_battlefield(&engine, p0, mountain()).is_none());
    assert!(in_graveyard(&engine, p0, mountain()).is_some());
    assert!(on_battlefield(&engine, p1, index::HILL_GIANT).is_none());
    assert!(in_graveyard(&engine, p1, index::HILL_GIANT).is_some());
    assert!(on_battlefield(&engine, p0, lesser_gargadon()).is_some());
}
