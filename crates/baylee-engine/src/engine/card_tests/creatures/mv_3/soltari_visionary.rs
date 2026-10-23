#[allow(clippy::wildcard_imports)]
use super::*;

fn soltari_visionary() -> CardIndex {
    card_index("7971f5c2-97b7-441f-ad77-954f43ede0a1")
}

fn song_of_serenity() -> CardIndex {
    card_index("4c543799-1b72-4f32-a5be-475c0134ac0f")
}

#[test]
fn soltari_visionary_destroys_opponent_enchantment_on_damage_to_player() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[soltari_visionary()])
        .battlefield(1, &[song_of_serenity()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let visionary = on_battlefield(&engine, p0, soltari_visionary()).expect("seated");
    let ench = on_battlefield(&engine, p1, song_of_serenity()).expect("seated");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(visionary, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ench],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(engine.state().players[1].life, 18);
    assert!(on_battlefield(&engine, p1, song_of_serenity()).is_none());
    assert!(in_graveyard(&engine, p1, song_of_serenity()).is_some());
}
