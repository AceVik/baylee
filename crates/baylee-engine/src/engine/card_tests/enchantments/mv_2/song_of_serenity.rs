#[allow(clippy::wildcard_imports)]
use super::*;

fn song_of_serenity() -> CardIndex {
    card_index("4c543799-1b72-4f32-a5be-475c0134ac0f")
}

fn holy_strength() -> CardIndex {
    card_index("9357de36-f8be-4f49-b2c8-9fe9eaf82b07")
}

#[test]
fn song_of_serenity_prevents_enchanted_creatures_from_attacking() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[song_of_serenity(), holy_strength()])
        .battlefield(0, &[forest(), plains(), plains(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    cast_from_hand(&mut engine, p0, song_of_serenity());
    pass_until(&mut engine, stack_is_empty);

    cast_from_hand(&mut engine, p0, holy_strength());
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![bears],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    // Grizzly Bears is now enchanted, so Song of Serenity stops it from attacking,
    // causing combat to skip the empty DeclareAttackers prompt and move to SecondMain.
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert!(!is_tapped(&engine, bears));
}
