//! `cards/lands/horizon/horizon_of_progress.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn horizon_of_progress_taps_for_land_color_at_one_life_and_draws_for_one_generic() {
    let p0 = PlayerId::new(0);

    {
        let mut engine = Duel::new(51, forest())
            .battlefield(0, &[horizon_of_progress(), forest(), island()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);

        let horizon =
            on_battlefield(&engine, p0, horizon_of_progress()).expect("the Horizon is out");
        let life_before = engine.state().players[0].life;

        let Pending::Priority { legal, .. } = engine.pending().clone() else {
            panic!("expected priority, got {:?}", engine.pending())
        };
        assert!(
            legal.abilities.contains(&(horizon, 0)),
            "the Horizon's mana ability is offered in legal.abilities: {:?}",
            legal.abilities
        );

        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: horizon,
                    ability_index: 0,
                },
            )
            .expect("the Horizon has two Forests beside it");

        let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
            panic!(
                "expected a colour choice from the land-colour mana ability, got {:?}",
                engine.pending()
            )
        };
        assert!(
            options.contains(&ManaColor::Green) && options.contains(&ManaColor::Blue),
            "the Forest and the Island are both types a land you control \
             could produce: {options:?}"
        );
        engine
            .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
            .unwrap();

        assert_eq!(
            engine.state().players[0]
                .mana_pool
                .available(ManaColor::Green),
            1,
            "one {{G}} in the pool"
        );
        assert_eq!(
            engine.state().players[0].life,
            life_before - 1,
            "the life payment was taken"
        );
        assert!(is_tapped(&engine, horizon), "the {{T}} tapped the land");
        assert!(
            stack_is_empty(&engine),
            "a mana ability never uses the stack (CR 605.3b)"
        );
    }

    {
        let mut engine = Duel::new(52, forest())
            .battlefield(0, &[horizon_of_progress(), forest()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);

        let horizon =
            on_battlefield(&engine, p0, horizon_of_progress()).expect("the Horizon is out");
        tap_mana_except(&mut engine, p0, horizon);

        let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

        activate(&mut engine, p0, horizon_of_progress(), 1);
        pass_until(&mut engine, stack_is_empty);

        assert_eq!(
            engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
            hand_before + 1,
            "the draw put one card into hand"
        );
        assert!(
            on_battlefield(&engine, p0, horizon_of_progress()).is_none(),
            "the Horizon sacrificed itself as part of the cost"
        );
        assert!(
            in_graveyard(&engine, p0, horizon_of_progress()).is_some(),
            "a sacrificed permanent goes to its owner's graveyard"
        );
    }
}

#[test]
fn horizon_puts_a_land_tapped_without_playing_it_or_triggering_city_of_traitors() {
    let p0 = PlayerId::new(0);
    let city = baylee_core::generated::index::CITY_OF_TRAITORS;
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[horizon_of_progress(), forest(), forest(), forest(), city],
        )
        .hand(0, &[plains(), llanowar_elves()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let land = in_hand(&e, p0, plains()).unwrap();
    let creature = in_hand(&e, p0, llanowar_elves()).unwrap();
    tap_all_mana_but(&mut e, p0, Some(horizon_of_progress()));
    activate(&mut e, p0, horizon_of_progress(), 2);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        options, min, max, ..
    } = e.pending().clone()
    else {
        unreachable!()
    };
    assert!(options.contains(&land));
    assert!(!options.contains(&creature));
    assert_eq!((min, max), (0, 1));
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![land],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(land).unwrap().zone, Zone::Battlefield);
    assert!(is_tapped(&e, land));
    assert_eq!(e.state().players[0].lands_played_this_turn, 0);
    assert!(on_battlefield(&e, p0, city).is_some());
}
