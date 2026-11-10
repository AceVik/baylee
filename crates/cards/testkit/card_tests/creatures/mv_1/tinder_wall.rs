//! Tinder Wall — `{G}` 0/3 Creature — Plant Wall (Ice Age)
//!
//! Defender
//! Sacrifice Tinder Wall: Add `{R}{R}`.
//! `{R}`, Sacrifice Tinder Wall: Tinder Wall deals 2 damage to target creature
//! it's blocking.

#[allow(clippy::wildcard_imports)]
use super::*;

fn tinder_wall() -> CardIndex {
    card_index("dd2f698c-b3b4-4dc8-9950-9be03ca7dc9e")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

#[test]
fn tinder_wall_has_defender_and_sacrifices_for_two_red_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[tinder_wall()])
        .battlefield(0, &[forest(), tinder_wall(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Seat 0 already controls one Tinder Wall (not summoning sick) and casts a second.
    let existing_wall = on_battlefield(&engine, p0, tinder_wall()).unwrap();
    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let hand_wall = in_hand(&engine, p0, tinder_wall()).unwrap();
    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: hand_wall })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    let cast_wall = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|&id| {
            id != existing_wall
                && engine
                    .state()
                    .object(id)
                    .unwrap()
                    .card
                    .is_some_and(|c| c.index == tinder_wall())
        })
        .unwrap();

    // Ability 0: Sacrifice Tinder Wall: Add {R}{R}.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: cast_wall,
                ability_index: 0,
            },
        )
        .unwrap();
    assert_eq!(
        engine.state().object(cast_wall).unwrap().zone,
        Zone::Graveyard,
        "Tinder Wall is sacrificed as the mana ability cost"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        2,
        "sacrificing Tinder Wall adds {{R}}{{R}}"
    );

    // Advance to combat: existing_wall has defender and cannot attack, while
    // Grizzly Bears can, so the menu is really open.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        panic!("expected ChooseAttackers");
    };
    assert!(
        attackers.contains(&bears),
        "Grizzly Bears may attack: the attack menu is open"
    );
    assert!(
        !attackers.contains(&existing_wall),
        "Tinder Wall has defender and cannot attack"
    );
}

#[test]
fn tinder_wall_sacrifices_to_deal_two_damage_only_to_creature_it_is_blocking() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[tinder_wall(), mountain()])
        .battlefield(1, &[grizzly_bears(), hill_giant()])
        .start();
    keep_mulligans(&mut engine);

    let wall = on_battlefield(&engine, p0, tinder_wall()).unwrap();
    let bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let giant = on_battlefield(&engine, p1, hill_giant()).unwrap();

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(p0)), (giant, Defender::Player(p0))],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    // Tinder Wall blocks Grizzly Bears, leaving Hill Giant unblocked.
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(wall, bears)],
            },
        )
        .unwrap();

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: wall,
                ability_index: 1,
            },
        )
        .unwrap();

    // Only Grizzly Bears (blocked by Tinder Wall) is a legal target; unblocked Hill Giant is excluded.
    let Pending::ChooseTargets { options, .. } = engine.pending() else {
        panic!(
            "expected ChooseTargets for Tinder Wall ability 1, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(
        options,
        &[bears],
        "only the creature Tinder Wall is blocking is a legal target"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(wall).unwrap().zone,
        Zone::Graveyard,
        "Tinder Wall was sacrificed"
    );
    assert_eq!(
        engine.state().object(bears).unwrap().zone,
        Zone::Graveyard,
        "blocked Grizzly Bears took 2 lethal damage from Tinder Wall"
    );
}
