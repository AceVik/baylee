//! `cards/creatures/mv_3/quicksilver_wall.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn quicksilver_wall() -> CardIndex {
    card_index("82372776-bedd-4893-92ad-f1272f9c3250")
}

/// Quicksilver Wall ({2}{U} 1/6 Creature — Wall):
/// "Defender"
/// "{4}: Return this creature to its owner's hand. Any player may activate this ability."
#[test]
fn quicksilver_wall_cannot_attack_and_any_player_may_pay_four_to_return_it_to_owners_hand() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let subject = quicksilver_wall();
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[subject, gray_ogre()])
        .battlefield(1, &[island(), island(), island(), island()])
        .start();
    keep_mulligans(&mut engine);

    let wall = on_battlefield(&engine, p0, subject).unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!();
    };
    assert!(
        !attackers.contains(&wall),
        "Quicksilver Wall has defender and must not be offered as a legal attacker"
    );
    engine
        .apply(p0, PlayerAction::DeclareAttackers { attackers: vec![] })
        .unwrap();

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );
    tap_all_mana(&mut engine, p1);
    engine
        .apply(
            p1,
            PlayerAction::ActivateAbility {
                source: wall,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, subject).is_none());
    assert!(in_hand(&engine, p0, subject).is_some());
}
