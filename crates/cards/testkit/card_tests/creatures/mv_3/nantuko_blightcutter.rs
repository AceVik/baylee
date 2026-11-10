//! `cards/creatures/mv_3/nantuko_blightcutter.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn nantuko_blightcutter_has_protection_from_black_and_scales_with_opponents_black_permanents_at_threshold()
 {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let card = card_index("c40bd057-d1be-45e2-9ebf-4ab176195f17");
    let mut board = vec![card, index::DRUDGE_SKELETONS];
    board.extend_from_slice(&[forest(); 8]);
    let mut game = Duel::new(SEED, forest())
        .battlefield(0, &board)
        .battlefield(
            1,
            &[
                index::DRUDGE_SKELETONS,
                index::SENGIR_VAMPIRE,
                index::GRIZZLY_BEARS,
            ],
        )
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);

    let cutter = on_battlefield(&game, p0, card).unwrap();
    let skel1 = on_battlefield(&game, p1, index::DRUDGE_SKELETONS).unwrap();
    assert_eq!(pt(&game, cutter), (2, 2));

    let lands = all_on_battlefield(&game, p0, forest());
    bury(&mut game, &lands[..6]);
    game.refresh_offer();
    assert_eq!(
        pt(&game, cutter),
        (2, 2),
        "below threshold, opponent's black permanents do not pump Nantuko Blightcutter"
    );

    bury(&mut game, &[lands[6]]);
    game.refresh_offer();
    tap_all_mana(&mut game, p0);
    assert_eq!(
        pt(&game, cutter),
        (4, 4),
        "at threshold, gets +1/+1 for each of the 2 black permanents opponents control (ignoring controller's own black permanents and opponent's green permanents)"
    );

    // Attack and verify opponent's black Drudge Skeletons cannot block Nantuko Blightcutter (protection from black).
    pass_until(&mut game, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    game.apply(
        p0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(cutter, Defender::Player(p1))],
        },
    )
    .unwrap();
    pass_until(&mut game, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    assert!(
        game.apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(skel1, cutter)],
            },
        )
        .is_err(),
        "protection from black prevents black Drudge Skeletons from blocking"
    );
}
