//! `cards/enchantments/auras/mv_2/stupefying_touch.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn stupefying_touch_draws_on_etb_and_stops_enchanted_creature_from_activating_abilities() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let card = card_index("566eafea-c8c2-43d6-b491-f20de276cdb6");
    let mut game = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), index::PRODIGAL_SORCERER])
        .battlefield(1, &[index::PRODIGAL_SORCERER])
        .hand(0, &[card])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);

    let p0_sorc = on_battlefield(&game, p0, index::PRODIGAL_SORCERER).unwrap();
    let p1_sorc = on_battlefield(&game, p1, index::PRODIGAL_SORCERER).unwrap();
    let hand_before = game.state().zones.list(ZoneLocation::Hand(p0)).len();

    tap_all_mana(&mut game, p0);
    cast_with_floating(&mut game, p0, card);
    game.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![p0_sorc],
            players: vec![],
        },
    )
    .unwrap();
    let rest = drive_to_rest(&mut game, p0);
    assert!(matches!(rest, Rest::Reached), "{rest:?}");

    // Cast 1 card, drew 1 card on ETB -> hand size unchanged.
    assert_eq!(
        game.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before
    );
    let Pending::Priority { legal, .. } = game.pending() else {
        panic!("expected priority");
    };
    assert!(
        !legal.abilities.contains(&(p0_sorc, 0)),
        "enchanted Prodigal Sorcerer's activated ability cannot be activated"
    );
    let _ = p1_sorc;
}
