//! `cards/creatures/mv_7/luminous_angel.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Luminous Angel` is a 4/4 flying creature under `Coverage::Implemented` that triggers at the beginning of its controller's upkeep.
/// The upkeep trigger presents an optional choice through `YesNoPrompt::MayDo` to create a 1/1 white Spirit token with flying.
/// When accepted, a 1/1 flying Spirit creature token enters the battlefield under its controller's control.
#[test]
fn luminous_angel_triggers_at_upkeep_to_create_a_spirit_token() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
            ],
        )
        .hand(0, &[luminous_angel()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, luminous_angel());
    pass_until(&mut engine, stack_is_empty);

    let angel = on_battlefield(&engine, p0, luminous_angel()).expect("Angel resolved");
    assert_eq!(pt(&engine, angel), (4, 4), "Luminous Angel is 4/4");
    assert!(
        keywords_of(&engine, angel).contains(KeywordSet::FLYING),
        "Luminous Angel has flying"
    );
    assert_eq!(tokens_of(&engine, p0).len(), 0, "no tokens created yet");

    // Pass until the beginning of p0's next upkeep when the trigger asks its MayDo question.
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });

    let Pending::YesNo { player, prompt, .. } = engine.pending().clone() else {
        panic!("expected MayDo choice, got {:?}", engine.pending());
    };
    assert_eq!(player, p0, "controller of the Angel makes the choice");
    assert_eq!(prompt, YesNoPrompt::MayDo);

    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one Spirit token created");
    let spirit = tokens[0];
    assert_eq!(pt(&engine, spirit), (1, 1), "Spirit token is 1/1");
    assert!(
        keywords_of(&engine, spirit).contains(KeywordSet::FLYING),
        "Spirit token has flying"
    );
}
