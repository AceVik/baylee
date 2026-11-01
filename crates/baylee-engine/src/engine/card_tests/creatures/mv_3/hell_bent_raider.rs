//! `cards/creatures/mv_3/hell_bent_raider.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn hell_bent_raider_has_first_strike_and_haste_and_discards_at_random_for_protection_from_white() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let card = card_index("ee8ab29b-0749-463a-abbd-eb0b9013aec8");
    let mut game = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .battlefield(1, &[index::SAVANNAH_LIONS])
        .hand(0, &[card, forest()])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);

    tap_all_mana(&mut game, p0);
    cast_with_floating(&mut game, p0, card);
    let rest = drive_to_rest(&mut game, p0);
    assert!(matches!(rest, Rest::Reached), "{rest:?}");

    let raider = on_battlefield(&game, p0, card).unwrap();
    let lions = on_battlefield(&game, p1, index::SAVANNAH_LIONS).unwrap();
    assert_eq!(pt(&game, raider), (2, 2));
    assert!(keywords(&game, raider).contains(KeywordSet::FIRST_STRIKE));
    assert!(keywords(&game, raider).contains(KeywordSet::HASTE));

    let hand_before = game.state().zones.list(ZoneLocation::Hand(p0)).len();
    activate(&mut game, p0, card, 0);
    let rest = drive_to_rest(&mut game, p0);
    assert!(matches!(rest, Rest::Reached), "{rest:?}");
    assert_eq!(
        game.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 1
    );

    // Attack on the turn it entered (haste) and verify white Savannah Lions cannot block it.
    pass_until(&mut game, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    game.apply(
        p0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(raider, Defender::Player(p1))],
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
                blockers: vec![(lions, raider)],
            },
        )
        .is_err(),
        "protection from white prevents white Savannah Lions from blocking Hell-Bent Raider"
    );
}
