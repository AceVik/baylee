//! `cards/creatures/mv_2/mystic_familiar.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn mystic_familiar_has_flying_and_gains_plus_one_and_protection_from_black_at_threshold() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let card = card_index("9d3931a2-baba-4e6e-b923-44df00900e51");
    let mut board = vec![card];
    board.extend_from_slice(&[plains(); 8]);
    let mut game = Duel::new(SEED, plains())
        .battlefield(0, &board)
        .battlefield(1, &[index::SENGIR_VAMPIRE])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);

    let familiar = on_battlefield(&game, p0, card).unwrap();
    let vampire = on_battlefield(&game, p1, index::SENGIR_VAMPIRE).unwrap();
    assert_eq!(pt(&game, familiar), (1, 2));
    assert!(keywords(&game, familiar).contains(KeywordSet::FLYING));

    let lands = all_on_battlefield(&game, p0, plains());
    bury(&mut game, &lands[..6]);
    game.refresh_offer();
    assert_eq!(pt(&game, familiar), (1, 2));

    bury(&mut game, &[lands[6]]);
    game.refresh_offer();
    tap_all_mana(&mut game, p0);
    assert_eq!(pt(&game, familiar), (2, 3));

    // Attack and verify black Sengir Vampire (flying) cannot block Mystic Familiar (protection from black).
    pass_until(&mut game, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    game.apply(
        p0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(familiar, Defender::Player(p1))],
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
                blockers: vec![(vampire, familiar)],
            },
        )
        .is_err(),
        "protection from black at threshold prevents black Sengir Vampire from blocking Mystic Familiar"
    );
}
