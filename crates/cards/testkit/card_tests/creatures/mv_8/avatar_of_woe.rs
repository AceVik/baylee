//! `cards/creatures/mv_8/avatar_of_woe.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn avatar_of_woe() -> CardIndex {
    card_index("2da5aa33-3794-4f83-936c-9d8640833d86")
}

#[test]
fn avatar_of_woe_has_fear_and_taps_to_destroy_bypassing_regeneration() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[avatar_of_woe()])
        .battlefield(0, &[swamp(); 8])
        .battlefield(
            1,
            &[
                swamp(),
                index::DRUDGE_SKELETONS,
                index::GRIZZLY_BEARS,
                index::BLACK_KNIGHT,
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, avatar_of_woe());
    pass_until(&mut engine, stack_is_empty);
    let avatar = on_battlefield(&engine, p0, avatar_of_woe()).unwrap();

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let skeletons = on_battlefield(&engine, p1, index::DRUDGE_SKELETONS).unwrap();
    let bears = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).unwrap();
    let knight = on_battlefield(&engine, p1, index::BLACK_KNIGHT).unwrap();

    // Tap Avatar of Woe to destroy Drudge Skeletons even if p1 regenerates it in response.
    activate(&mut engine, p0, avatar_of_woe(), 1);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![skeletons],
            },
        )
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );
    tap_all_mana(&mut engine, p1);
    activate(&mut engine, p1, index::DRUDGE_SKELETONS, 0);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p1, index::DRUDGE_SKELETONS).is_some(),
        "It can't be regenerated must bypass Drudge Skeletons' regeneration shield"
    );

    // Next turn, verify fear in combat.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(avatar, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    assert!(
        engine
            .apply(
                p1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(bears, avatar)],
                },
            )
            .is_err()
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(knight, avatar)],
            },
        )
        .unwrap();
}
