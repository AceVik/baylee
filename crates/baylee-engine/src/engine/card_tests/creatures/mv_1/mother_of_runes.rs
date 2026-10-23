#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn mother_of_runes() -> CardIndex {
    card_index("60433b48-d27f-413c-905c-43839b1943f1")
}

#[test]
fn mother_of_runes_grants_protection_from_chosen_color() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), mother_of_runes()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mom = on_battlefield(&engine, p0, mother_of_runes()).unwrap();

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: mom,
                ability_index: 0,
            },
        )
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![mom],
                    players: vec![],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. }) || stack_is_empty(e)
    });
    if matches!(engine.pending(), Pending::ChooseColor { .. }) {
        engine
            .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, mom));
}

#[test]
fn mother_of_runes_protection_restricts_red_targets_and_expires_next_turn() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = lk::game(
        SEED,
        &[mother_of_runes(), grizzly_bears()],
        &[gray_ogre()],
        &[],
        &[lightning_bolt(), lightning_bolt()],
    );
    let mother = lk::obj(&engine, p0, mother_of_runes());
    let bear = lk::obj(&engine, p0, grizzly_bears());
    let ogre = lk::obj(&engine, p1, gray_ogre());
    activate(&mut engine, p0, mother_of_runes(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending() else {
        unreachable!()
    };
    assert!(
        !options.contains(&ogre),
        "only your creature can receive protection"
    );
    lk::aim(&mut engine, &[bear], &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();
    lk::settle(&mut engine);
    lk::pass_once(&mut engine);
    lk::float(&mut engine, p1, &[(ManaColor::Red, 1)]);
    cast_with_floating(&mut engine, p1, lightning_bolt());
    let Pending::ChooseTargets { options, .. } = engine.pending() else {
        unreachable!()
    };
    assert!(!options.contains(&bear));
    assert!(
        options.contains(&mother),
        "another creature has no protection"
    );
    lk::aim(&mut engine, &[mother], &[]);
    lk::settle(&mut engine);
    assert!(on_battlefield(&engine, p0, grizzly_bears()).is_some());
    reach_their_main_phase(&mut engine, p1);
    lk::float(&mut engine, p1, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut engine, p1, lightning_bolt(), &[bear], &[]);
    assert!(in_graveyard(&engine, p0, grizzly_bears()).is_some());
}
