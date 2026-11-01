//! `cards/creatures/mv_3/dauthi_jackal.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn dauthi_jackal() -> CardIndex {
    card_index("8847b057-8896-4c72-a11f-3c183b1cc60e")
}

/// Dauthi Jackal (`8847b057-8896-4c72-a11f-3c183b1cc60e`) — {2}{B} Creature — Dauthi Jackal (Implemented):
/// "Shadow. {B}{B}, Sacrifice this creature: Destroy target blocking creature."
#[test]
fn dauthi_jackal_sacrifices_itself_to_destroy_target_blocking_creature() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[dauthi_jackal(), index::GRIZZLY_BEARS, swamp(), swamp()],
        )
        .battlefield(1, &[index::WALL_OF_STONE])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let jackal = on_battlefield(&engine, P0, dauthi_jackal()).unwrap();
    let attacker = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    let blocker = on_battlefield(&engine, P1, index::WALL_OF_STONE).unwrap();

    kit::declare_attack(&mut engine, &[attacker], P1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, attacker)],
            },
        )
        .unwrap();

    tap_all_mana(&mut engine, P0);
    kit::activate_object(&mut engine, P0, jackal, 0);
    let Pending::ChooseTargets { options, .. } = engine.pending() else {
        panic!("targets")
    };
    assert!(!options.contains(&attacker));
    aim_at(&mut engine, P0, blocker);
    kit::resolve_the_stack(&mut engine, |_, _| None);

    assert!(on_battlefield(&engine, P0, dauthi_jackal()).is_none());
    assert!(on_battlefield(&engine, P1, index::WALL_OF_STONE).is_none());
}

#[test]
fn shadow_limits_both_blocking_and_being_blocked() {
    use crate::engine::card_tests::legends_kit as lk;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let subject = card_index("8847b057-8896-4c72-a11f-3c183b1cc60e");
    let mut e = lk::game(
        SEED,
        &[subject, gray_ogre()],
        &[subject, grizzly_bears()],
        &[],
        &[],
    );
    let shadow = lk::obj(&e, p0, subject);
    let normal = lk::obj(&e, p0, gray_ogre());
    let enemy_shadow = lk::obj(&e, p1, subject);
    let enemy_normal = lk::obj(&e, p1, grizzly_bears());
    let offer = lk::attack(&mut e, &[shadow, normal]);
    assert!(lk::may_block(&offer, enemy_shadow, shadow));
    assert!(lk::may_block(&offer, enemy_normal, normal));
    assert!(!lk::may_block(&offer, enemy_normal, shadow));
    assert!(!lk::may_block(&offer, enemy_shadow, normal));
}
