//! `cards/creatures/mv_1/benevolent_bodyguard.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn benevolent_bodyguard_sacrifices_itself_to_grant_target_creature_you_control_protection_from_chosen_color()
 {
    let bodyguard = card_index("c8c38570-0b42-4bfc-896f-f57f9a11ba9d");
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), grizzly_bears()])
        .battlefield(1, &[mountain(), gray_ogre()])
        .hand(0, &[bodyguard])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, bodyguard);
    pass_until(&mut engine, stack_is_empty);

    let bg = on_battlefield(&engine, p0, bodyguard).expect("Benevolent Bodyguard entered");
    let bear = on_battlefield(&engine, p0, grizzly_bears()).expect("Grizzly Bears in play");
    let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("opponent creature in play");

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: bg,
                ability_index: 0,
            },
        )
        .expect("sacrifice cost activates immediately");
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets");
    };
    assert!(
        options.contains(&bear) && !options.contains(&ogre),
        "only creatures you control are legal targets: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bear],
                players: vec![],
            },
        )
        .unwrap();
    assert!(
        in_graveyard(&engine, p0, bodyguard).is_some(),
        "sacrificed as cost"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .expect("choose red");
    pass_until(&mut engine, stack_is_empty);

    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, p1, lightning_bolt());
    let Pending::ChooseTargets {
        options: bolt_targets,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseTargets");
    };
    assert!(
        !bolt_targets.contains(&bear),
        "Grizzly Bears has protection from red until end of turn: {bolt_targets:?}"
    );
}
