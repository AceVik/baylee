#[allow(clippy::wildcard_imports)]
use super::*;

fn discordant_dirge() -> CardIndex {
    card_index("edd4741b-3ee6-421a-8bac-d60736755356")
}

#[test]
fn discordant_dirge_sacrifices_to_reveal_target_opponent_hand() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[discordant_dirge()])
        .hand(1, &[grizzly_bears()])
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, discordant_dirge());
    pass_until(&mut engine, stack_is_empty);

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, discordant_dirge(), 1);
    if matches!(engine.pending(), Pending::ChoosePlayer { .. }) {
        engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
    } else if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![p1],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        min,
        max,
        options,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!(player, p0);
    assert_eq!((min, max), (0, 0));
    assert!(matches!(prompt, crate::choice::ChoicePrompt::LookAtHand));
    assert!(options.contains(&in_hand(&engine, p1, grizzly_bears()).unwrap()));
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_hand(&engine, p1, grizzly_bears()).is_some(),
        "zero verse counters cannot discard a card"
    );

    assert!(on_battlefield(&engine, p0, discordant_dirge()).is_none());
    assert!(in_graveyard(&engine, p0, discordant_dirge()).is_some());
}
