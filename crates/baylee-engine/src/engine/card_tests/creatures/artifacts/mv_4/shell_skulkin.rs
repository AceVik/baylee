//! `cards/creatures/artifacts/mv_4/shell_skulkin.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn shell_skulkin() -> CardIndex {
    card_index("82df38bc-f29b-49f8-b734-e4df6d0f645a")
}

/// Shell Skulkin — {4} 3/2: "{3}: Target blue creature gains shroud until
/// end of turn." Among a Giant Tortoise (blue), a Grizzly Bears (green) and
/// the Skulkin itself (colorless) only the Tortoise is a target; paid, the
/// Tortoise has shroud and loses it at the end of the turn.
#[test]
fn shell_skulkin_gives_a_blue_creature_shroud() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                shell_skulkin(),
                giant_tortoise(),
                grizzly_bears(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let tortoise = on_battlefield(&engine, p0, giant_tortoise()).expect("seated");
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    let skulkin = on_battlefield(&engine, p0, shell_skulkin()).expect("seated");
    assert!(!keywords(&engine, tortoise).contains(KeywordSet::SHROUD));

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, shell_skulkin(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the ability targets: {:?}", engine.pending());
    };
    assert_eq!(options, vec![tortoise], "only the blue creature");
    assert!(!options.contains(&bears) && !options.contains(&skulkin));
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![tortoise],
                players: vec![],
            },
        )
        .expect("the Tortoise is a legal target");
    pass_until(&mut engine, stack_is_empty);
    assert!(keywords(&engine, tortoise).contains(KeywordSet::SHROUD));
    assert!(!keywords(&engine, bears).contains(KeywordSet::SHROUD));
    assert_eq!(engine.state().players[0].mana_pool.total(), 0, "{{3}} paid");

    pass_until(&mut engine, |e| e.state().turn.number >= 2);
    assert!(
        !keywords(&engine, tortoise).contains(KeywordSet::SHROUD),
        "until end of turn"
    );
}
