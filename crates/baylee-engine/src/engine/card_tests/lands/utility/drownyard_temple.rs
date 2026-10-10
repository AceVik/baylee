//! `cards/lands/utility/drownyard_temple.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The mana ability works on the battlefield; the return ability works only in the graveyard.
#[test]
fn drownyard_temple_taps_for_colorless_but_cannot_return_from_the_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(124, forest())
        .battlefield(0, &[drownyard_temple()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let temple = on_battlefield(&engine, p0, drownyard_temple()).expect("Temple deployed");

    activate(&mut engine, p0, drownyard_temple(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, temple));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal
            .abilities
            .iter()
            .any(|(s, ai)| *s == temple && *ai == 1),
        "no second ability is offered"
    );
}

#[test]
fn temple_returns_tapped_without_using_a_land_play_and_cannot_follow_a_zone_change() {
    use crate::engine::card_tests::legends_kit as kit;
    for exile_in_response in [false, true] {
        let mut e = kit::game(972, &[drownyard_temple()], &[], &[], &[]);
        let card = kit::obj(&e, kit::P0, drownyard_temple());
        e.state
            .move_object(
                card,
                ZoneLocation::Graveyard(kit::P0),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
        kit::float(&mut e, kit::P0, &[(ManaColor::Colorless, 3)]);
        assert!(priority_offer(&e).abilities.contains(&(card, 1)));
        let land_plays = e.state.players[0].lands_played_this_turn;
        e.apply(
            kit::P0,
            PlayerAction::ActivateAbility {
                source: card,
                ability_index: 1,
            },
        )
        .unwrap();
        if exile_in_response {
            e.state
                .move_object(
                    card,
                    ZoneLocation::Exile(kit::P0),
                    ZonePosition::Top,
                    Cause::Effect,
                )
                .unwrap();
        }
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            e.state.object(card).unwrap().zone,
            if exile_in_response {
                Zone::Exile
            } else {
                Zone::Battlefield
            }
        );
        if !exile_in_response {
            assert!(is_tapped(&e, card));
        }
        assert_eq!(e.state.players[0].lands_played_this_turn, land_plays);
        assert_eq!(kit::pool_total(&e, kit::P0), 0);
    }
}
