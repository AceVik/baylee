use super::*;

fn reconnaissance() -> CardIndex {
    card_index("32599d7c-243b-447b-9d5d-e8e29830ac61")
}

#[test]
fn reconnaissance_removes_attacking_creature_from_combat_and_untaps_it() {
    let mut e = table(SEED, &[], &[reconnaissance(), gray_ogre()], &[]);
    let ogre = on_battlefield(&e, P0, gray_ogre()).unwrap();

    pass_until(
        &mut e,
        |eng| matches!(eng.pending(), Pending::ChooseAttackers { player, .. } if *player == P0),
    );
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(ogre, Defender::Player(P1))],
        },
    )
    .unwrap();

    assert!(is_tapped(&e, ogre));
    activate(&mut e, P0, reconnaissance(), 0);
    e.apply(
        P0,
        PlayerAction::ChooseTargets {
            objects: vec![ogre],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert!(!is_tapped(&e, ogre));
}
