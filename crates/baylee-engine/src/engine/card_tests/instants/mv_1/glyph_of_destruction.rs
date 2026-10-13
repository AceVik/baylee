//! Printed behavior and adversarial resolution cases.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn glyph_prevents_damage_for_the_turn_then_destroys_its_remembered_wall() {
    use baylee_core::ids::Defender;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::WALL_OF_WOOD, index::MOUNTAIN])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .hand(0, &[index::GLYPH_OF_DESTRUCTION])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let wall = on_battlefield(&e, p0, index::WALL_OF_WOOD).unwrap();
    let bear = on_battlefield(&e, p1, index::GRIZZLY_BEARS).unwrap();
    pass_until(
        &mut e,
        |e| matches!(e.pending(),Pending::ChooseAttackers {player,..} if *player==p1),
    );
    e.apply(
        p1,
        PlayerAction::DeclareAttackers {
            attackers: vec![(bear, Defender::Player(p0))],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    e.apply(
        p0,
        PlayerAction::DeclareBlockers {
            blockers: vec![(wall, bear)],
        },
    )
    .unwrap();
    pass_until(
        &mut e,
        |e| matches!(e.pending(),Pending::Priority {player,..} if *player==p0),
    );
    cast_from_hand(&mut e, p0, index::GLYPH_OF_DESTRUCTION);
    aim_at(&mut e, p0, wall);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, wall), (10, 3));
    let state = e.dev_state_mut(p0).unwrap();
    let mut work = crate::damage::DamageWork::new(
        state,
        vec![crate::damage::Assignment {
            source: bear,
            source_version: Some(state.object(bear).unwrap().version),
            recipient: crate::event::DamageTarget::Object(wall),
            amount: 100,
            is_combat: false,
        }],
    );
    assert!(work.advance(state).is_none());
    assert_eq!(work.dealt(), 0);
    pass_until(&mut e, |e| {
        e.state().turn.phase == Phase::Ending && stack_is_empty(e)
    });
    assert_eq!(e.state().object(wall).unwrap().zone, Zone::Graveyard);
}

/// "Target blocking Wall you control": before the Wall blocks nothing can be
/// targeted; once it does, the Wall is the one target, and neither the
/// attacking Bears, the non-Wall creature blocking beside it, nor the
/// caster's Wall that is not blocking are offered.
#[test]
fn glyph_targets_only_a_wall_that_is_blocking() {
    use crate::engine::card_tests::legends_kit as kit;
    let glyph = index::GLYPH_OF_DESTRUCTION;
    let mut e = kit::game(
        957,
        &[index::GRIZZLY_BEARS],
        &[
            index::WALL_OF_WOOD,
            index::WALL_OF_WOOD,
            index::LLANOWAR_ELVES,
        ],
        &[],
        &[glyph],
    );
    let bear = kit::obj(&e, kit::P0, index::GRIZZLY_BEARS);
    let walls = all_on_battlefield(&e, kit::P1, index::WALL_OF_WOOD);
    assert_eq!(walls.len(), 2);
    let (blocking_wall, idle_wall) = (walls[0], walls[1]);
    let elves = kit::obj(&e, kit::P1, index::LLANOWAR_ELVES);

    let able = kit::attackers_offered(&mut e);
    assert!(able.contains(&bear));
    e.apply(
        kit::P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(bear, Defender::Player(kit::P1))],
        },
    )
    .unwrap();
    kit::pass_once(&mut e);
    assert!(matches!(e.pending(), Pending::Priority { player, .. } if *player == kit::P1));
    kit::float(&mut e, kit::P1, &[(ManaColor::Red, 1)]);
    assert!(
        !kit::castable(&e, kit::P1, glyph),
        "no Wall is blocking yet, so there is nothing to target"
    );

    kit::pass_once(&mut e);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    kit::block(&mut e, &[(blocking_wall, bear), (elves, bear)]);
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == kit::P1),
    );
    kit::float(&mut e, kit::P1, &[(ManaColor::Red, 1)]);
    assert!(kit::castable(&e, kit::P1, glyph), "a Wall is blocking now");
    cast_with_floating(&mut e, kit::P1, glyph);
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("expected a target choice, got {:?}", e.pending())
    };
    assert_eq!(
        options,
        vec![blocking_wall],
        "only the blocking Wall: not the attacking Bears {bear:?}, the blocking Elves \
         {elves:?} or the idle Wall {idle_wall:?}"
    );
}

/// "Target blocking Wall you control": a Wall the opponent controls that
/// blocks is not the caster's.
#[test]
fn glyph_cannot_target_an_opponents_blocking_wall() {
    use crate::engine::card_tests::legends_kit as kit;
    let glyph = index::GLYPH_OF_DESTRUCTION;
    let mut e = kit::game(
        958,
        &[index::GRIZZLY_BEARS],
        &[index::WALL_OF_WOOD],
        &[glyph],
        &[],
    );
    let bear = kit::obj(&e, kit::P0, index::GRIZZLY_BEARS);
    let wall = kit::obj(&e, kit::P1, index::WALL_OF_WOOD);
    kit::attack(&mut e, &[bear]);
    kit::block(&mut e, &[(wall, bear)]);
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == kit::P0),
    );
    kit::float(&mut e, kit::P0, &[(ManaColor::Red, 1)]);
    assert!(
        e.state().players[0].mana_pool.available(ManaColor::Red) >= 1,
        "the {{R}} is floating, so only the target is missing"
    );
    assert!(
        !kit::castable(&e, kit::P0, glyph),
        "the only blocking Wall is the opponent's, so there is no target"
    );
}
