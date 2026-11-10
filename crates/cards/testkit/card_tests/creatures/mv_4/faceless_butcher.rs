//! `cards/creatures/mv_4/faceless_butcher.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn faceless_butcher_exiles_another_creature_on_etb_and_returns_it_when_leaving() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let card = card_index("5237a105-60c4-4597-b24e-84c12ce7b01f");
    let mut game = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .battlefield(1, &[grizzly_bears(), mountain(), mountain()])
        .hand(0, &[card])
        .hand(1, &[index::INCINERATE])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);

    let bear = on_battlefield(&game, p1, grizzly_bears()).unwrap();
    tap_all_mana(&mut game, p0);
    cast_with_floating(&mut game, p0, card);
    pass_until(&mut game, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });

    let butcher = on_battlefield(&game, p0, card).unwrap();
    assert_eq!(pt(&game, butcher), (2, 3));
    let Pending::ChooseTargets { options, .. } = game.pending().clone() else {
        panic!("expected ChooseTargets for ETB trigger");
    };
    assert!(
        !options.contains(&butcher),
        "Faceless Butcher says 'another target creature' and cannot target itself"
    );
    assert_eq!(options, vec![bear]);

    game.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![bear],
            players: vec![],
        },
    )
    .unwrap();
    let rest = drive_to_rest(&mut game, p0);
    assert!(matches!(rest, Rest::Reached), "{rest:?}");
    assert!(on_battlefield(&game, p1, grizzly_bears()).is_none());
    assert_eq!(game.state().object(bear).map(|o| o.zone), Some(Zone::Exile));

    // Destroy Faceless Butcher and verify Grizzly Bears returns to the battlefield under p1's control.
    game.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut game, p1);
    cast_with_floating(&mut game, p1, index::INCINERATE);
    game.apply(
        p1,
        PlayerAction::ChooseTargets {
            objects: vec![butcher],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(&mut game, stack_is_empty);

    assert!(on_battlefield(&game, p0, card).is_none());
    assert!(on_battlefield(&game, p1, grizzly_bears()).is_some());
}
