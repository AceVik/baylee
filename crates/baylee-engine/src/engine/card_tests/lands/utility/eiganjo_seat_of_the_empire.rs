//! `cards/lands/utility/eiganjo_seat_of_the_empire.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Outside combat Channel has no target; the land face still taps for white.
#[test]
fn eiganjo_seat_of_the_empire_taps_for_white_and_channel_requires_combat() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[eiganjo_seat_of_the_empire()])
        .battlefield(0, &[plains(), plains(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Float {2}{W} from the lands on the battlefield.
    tap_all_mana(&mut engine, p0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 2);
    assert_eq!(pool.available(ManaColor::Green), 1);

    let card = in_hand(&engine, p0, eiganjo_seat_of_the_empire()).expect("eiganjo in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.iter().any(|(source, _)| *source == card),
        "Channel requires an attacking or blocking creature"
    );

    let land = play_land(&mut engine, p0, eiganjo_seat_of_the_empire());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, eiganjo_seat_of_the_empire(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 3);
    assert_eq!(pool.available(ManaColor::Green), 1);
    assert_eq!(pool.total(), 4);
    assert!(is_tapped(&engine, land));
}

#[test]
fn eiganjo_channel_hits_attackers_or_blockers_and_counts_only_your_legends() {
    use crate::engine::card_tests::legends_kit as kit;
    use baylee_core::generated::index;
    for target_blocker in [false, true] {
        let mut e = kit::game(
            973,
            &[index::EMRY_LURKER_OF_THE_LOCH, llanowar_elves()],
            &[serra_angel(), index::EMRY_LURKER_OF_THE_LOCH],
            &[eiganjo_seat_of_the_empire()],
            &[],
        );
        let attacker = kit::obj(&e, kit::P0, llanowar_elves());
        let blocker = kit::obj(&e, kit::P1, serra_angel());
        kit::attack(&mut e, &[attacker]);
        kit::block(&mut e, &[(blocker, attacker)]);
        kit::float(&mut e, kit::P0, &[(ManaColor::White, 1)]);
        let card = in_hand(&e, kit::P0, eiganjo_seat_of_the_empire()).unwrap();
        assert!(!priority_offer(&e).abilities.contains(&(card, 1)));
        kit::float(&mut e, kit::P0, &[(ManaColor::Colorless, 1)]);
        assert!(priority_offer(&e).abilities.contains(&(card, 1)));
        e.apply(
            kit::P0,
            PlayerAction::ActivateAbility {
                source: card,
                ability_index: 1,
            },
        )
        .unwrap();
        let target = if target_blocker { blocker } else { attacker };
        kit::aim(&mut e, &[target], &[]);
        assert_eq!(e.state.object(card).unwrap().zone, Zone::Graveyard);
        assert_eq!(kit::pool_total(&e, kit::P0), 0);
        pass_until(&mut e, stack_is_empty);
        assert_eq!(e.state.object(target).unwrap().zone, Zone::Graveyard);
    }
}
