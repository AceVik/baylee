//! Oracle checklist: pay {1}{B} and {T}; player target (either seat); only that player loses 1 life; tap prevents another activation until untap.
#[allow(clippy::wildcard_imports)]
use super::*;
fn acolyte_of_xathrid() -> CardIndex {
    card_index("62dca41b-58b0-45eb-ada7-568e80a2abec")
}

#[test]
fn acolyte_pays_its_mana_and_targets_one_player_for_one_life() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[acolyte_of_xathrid(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut e);
    assert!(walk_to_own_main(&mut e, p0));
    let acolyte = on_battlefield(&e, p0, acolyte_of_xathrid()).unwrap();
    tap_all_mana(&mut e, p0);
    let life0 = e.state().players[0].life;
    let life1 = e.state().players[1].life;
    activate(&mut e, p0, acolyte_of_xathrid(), 0);
    let Pending::ChooseTargets {
        player_options,
        min,
        max,
        ..
    } = e.pending().clone()
    else {
        panic!("expected player target prompt: {:?}", e.pending());
    };
    assert_eq!((min, max), (1, 1));
    assert_eq!(player_options.len(), 2);
    assert!(player_options.contains(&p0));
    assert!(
        player_options.contains(&p1),
        "target player includes either seat"
    );
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![p1],
        },
    )
    .unwrap();
    assert!(is_tapped(&e, acolyte));
    assert_eq!(
        e.state().players[0].mana_pool.total(),
        0,
        "{{1}}{{B}} was paid"
    );
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[0].life, life0);
    assert_eq!(e.state().players[1].life, life1 - 1);
    let Pending::Priority { legal, .. } = e.pending().clone() else {
        panic!("expected priority");
    };
    assert!(
        !legal.abilities.iter().any(|(id, _)| *id == acolyte),
        "tap cost bars another activation"
    );
}
