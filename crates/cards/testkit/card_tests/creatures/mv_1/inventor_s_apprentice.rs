//! Oracle checklist: Apprentice is 1/2 without an artifact; an own artifact enables exactly +1/+1, but opponent artifacts do not; the bonus ceases when the last own artifact leaves.
#[allow(clippy::wildcard_imports)]
use super::*;

fn inventors_apprentice() -> CardIndex {
    card_index("ed8d54fa-ec02-45d6-b70e-4c7e1c66efd6")
}

#[test]
fn apprentice_checks_its_own_controllers_artifacts_as_they_enter() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[inventors_apprentice()])
        .battlefield(1, &[sol_ring()])
        .hand(0, &[sol_ring(), sol_ring(), naturalize(), naturalize()])
        .start();
    keep_mulligans(&mut e);
    assert!(walk_to_own_main(&mut e, p0));
    let apprentice = on_battlefield(&e, p0, inventors_apprentice()).unwrap();
    assert_eq!(
        pt(&e, apprentice),
        (1, 2),
        "the opponent's artifact does not qualify"
    );

    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 2);
    e.refresh_offer();
    cast_from_hand(&mut e, p0, sol_ring());
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        pt(&e, apprentice),
        (2, 3),
        "an own artifact turns on the bonus"
    );
    cast_from_hand(&mut e, p0, sol_ring());
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        pt(&e, apprentice),
        (2, 3),
        "a second artifact does not add a second bonus"
    );
    assert!(on_battlefield(&e, p1, sol_ring()).is_some());

    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Green, 4);
    e.refresh_offer();
    let first_ring = all_on_battlefield(&e, p0, sol_ring())[0];
    cast_with_floating(&mut e, p0, naturalize());
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("Naturalize target")
    };
    assert!(options.contains(&first_ring));
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![first_ring],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        pt(&e, apprentice),
        (2, 3),
        "one own artifact still qualifies"
    );

    let last_ring = all_on_battlefield(&e, p0, sol_ring())[0];
    cast_with_floating(&mut e, p0, naturalize());
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("Naturalize target")
    };
    assert!(options.contains(&last_ring));
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![last_ring],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        pt(&e, apprentice),
        (1, 2),
        "the bonus ends with the last own artifact"
    );
    assert!(
        on_battlefield(&e, p1, sol_ring()).is_some(),
        "an opponent artifact remains"
    );
}
