//! `cards/artifacts/mv_4/gauntlet_of_might.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Gauntlet of Might boosts red creatures on both sides and adds red only
/// for Mountains. Its triggered mana ability resolves at once (CR 605.1b).
#[test]
fn alpha_eval_gauntlet_of_might_boosts_both_sides_and_the_mountains_controller() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let gauntlet = card_index("d38ad188-515e-4865-a0ed-5d0fd4c7b453");
    let red_body = card_index("0b8e3f9b-a4da-49a3-8545-ce7a265e5856");
    let mut engine = Duel::new(1007, forest())
        .battlefield(
            0,
            &[gauntlet, red_body, llanowar_elves(), mountain(), forest()],
        )
        .battlefield(1, &[red_body, mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    for seat in [p0, p1] {
        assert_eq!(
            pt(&engine, on_battlefield(&engine, seat, red_body).unwrap()),
            (4, 5)
        );
    }
    assert_eq!(
        pt(
            &engine,
            on_battlefield(&engine, p0, llanowar_elves()).unwrap()
        ),
        (1, 1)
    );
    let green = on_battlefield(&engine, p0, forest()).unwrap();
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: green })
        .unwrap();
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "a Forest gets no bonus"
    );
    for seat in [p0, p1] {
        pass_until(&mut engine, |e| at_rest(e, seat));
        let land = on_battlefield(&engine, seat, mountain()).unwrap();
        engine
            .apply(seat, PlayerAction::ActivateManaAbility { source: land })
            .unwrap();
        assert!(stack_is_empty(&engine));
        assert_eq!(
            engine.state().players[usize::from(seat.get())]
                .mana_pool
                .available(ManaColor::Red),
            2
        );
    }
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        2,
        "their tap paid them, not us"
    );
}

/// Wizards' 2004-10-04 ruling explicitly includes either color of a dual
/// Mountain. Verify the choice boundary as well as the recipient.
/// <https://api.scryfall.com/cards/df3bc33f-23f8-4eb4-a70e-b8b7af5b40f6/rulings>
#[test]
fn alpha_eval_gauntlet_adds_red_when_taiga_chooses_green() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let gauntlet = card_index("d38ad188-515e-4865-a0ed-5d0fd4c7b453");
    let taiga = card_index("22e3cf1d-3559-4ce1-954c-8dc815342979");
    let mut engine = Duel::new(1008, forest())
        .battlefield(0, &[taiga])
        .battlefield(1, &[gauntlet])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let land = on_battlefield(&engine, p0, taiga).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 0,
            },
        )
        .unwrap();
    let Pending::ChooseColor { player, options } = engine.pending() else {
        panic!("Taiga must offer its colors: {:?}", engine.pending());
    };
    assert_eq!(*player, p0);
    assert!(options.contains(&ManaColor::Green));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "tapping alone has not produced mana yet"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();
    assert!(stack_is_empty(&engine), "the bonus is a mana ability");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1);
    assert_eq!(pool.available(ManaColor::Red), 1);
    assert_eq!(pool.total(), 2);
    assert_eq!(
        engine.state().players[usize::from(p1.get())]
            .mana_pool
            .total(),
        0,
        "the land's controller receives the bonus, not the artifact's"
    );
}

/// Independent copies add their bonuses; removing each source through a
/// real spell removes exactly its continuous and triggered contribution.
#[test]
fn alpha_eval_gauntlets_stack_and_stop_after_disenchant() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let gauntlet = card_index("d38ad188-515e-4865-a0ed-5d0fd4c7b453");
    let disenchant = card_index("a7e97fa9-4b72-4548-b854-5be5f18a6f1a");
    let mut engine = Duel::new(1009, forest())
        .battlefield(
            0,
            &[
                gauntlet,
                hill_giant(),
                mountain(),
                mountain(),
                mountain(),
                plains(),
                plains(),
                plains(),
                plains(),
            ],
        )
        .battlefield(1, &[gauntlet, hill_giant()])
        .hand(0, &[disenchant, disenchant])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let mountains = all_on_battlefield(&engine, p0, mountain());
    let white = all_on_battlefield(&engine, p0, plains());
    for (removed, land) in mountains.into_iter().enumerate() {
        for seat in [p0, p1] {
            let giant = on_battlefield(&engine, seat, hill_giant()).unwrap();
            let stat = 5 - i16::try_from(removed).unwrap();
            assert_eq!(pt(&engine, giant), (stat, stat));
        }
        let before = engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red);
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: land })
            .unwrap();
        assert!(stack_is_empty(&engine));
        assert_eq!(
            engine.state().players[0]
                .mana_pool
                .available(ManaColor::Red)
                - before,
            u32::try_from(3 - removed).unwrap(),
            "one extra per surviving Gauntlet"
        );
        let Some(owner) = [p1, p0].get(removed).copied() else {
            break;
        };
        for source in &white[removed * 2..removed * 2 + 2] {
            engine
                .apply(p0, PlayerAction::ActivateManaAbility { source: *source })
                .unwrap();
        }
        let artifact = on_battlefield(&engine, owner, gauntlet).unwrap();
        cast_with_floating(&mut engine, p0, disenchant);
        let Pending::ChooseTargets { options, .. } = engine.pending() else {
            panic!("Disenchant must ask for its target: {:?}", engine.pending());
        };
        assert!(options.contains(&artifact));
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![artifact],
                },
            )
            .unwrap();
        pass_until(&mut engine, |e| stack_is_empty(e) && at_rest(e, p0));
        assert!(in_graveyard(&engine, owner, gauntlet).is_some());
    }
}
