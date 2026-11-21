//! Rekindling Phoenix and its Elemental, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn rekindling_phoenix_can_attack_the_turn_the_token_returns_it() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let card = card_index("f34be4b8-126d-4b5c-8ebb-23b5e5370012");
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[card])
        .battlefield(1, &[mountain()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    let phoenix = on_battlefield(&engine, p0, card).expect("the Phoenix is on the battlefield");
    assert!(keywords(&engine, phoenix).contains(KeywordSet::FLYING));

    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, lightning_bolt());
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![phoenix],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(in_graveyard(&engine, p0, card), Some(phoenix));
    let [elemental] = tokens_of(&engine, p0)[..] else {
        panic!("the death trigger creates an Elemental");
    };
    assert_eq!(pt(&engine, elemental), (0, 1));

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending()
    else {
        unreachable!()
    };
    assert_eq!(*player, p0);
    assert_eq!(options.as_slice(), &[phoenix]);
    assert_eq!(tokens_of(&engine, p0), [elemental]);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![phoenix],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        stack_is_empty(e) || matches!(e.pending(), Pending::ChooseCards { .. })
    });
    if let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending()
    {
        assert_eq!((*player, *min, *max), (p0, 1, 1));
        assert_eq!(options.as_slice(), &[elemental]);
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![elemental],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);
    assert!(tokens_of(&engine, p0).is_empty());
    assert_eq!(on_battlefield(&engine, p0, card), Some(phoenix));
    assert!(keywords(&engine, phoenix).contains(KeywordSet::HASTE));

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    let Pending::ChooseAttackers { attackers, .. } = engine.pending() else {
        unreachable!()
    };
    assert!(
        attackers.contains(&phoenix),
        "the creature returned in this turn's upkeep can attack"
    );
    reach_their_main_phase(&mut engine, p1);
    assert!(!keywords(&engine, phoenix).contains(KeywordSet::HASTE));
}
