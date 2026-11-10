#[allow(clippy::wildcard_imports)] // Shared card-test harness.
use super::*;
use baylee_core::generated::index::TWIDDLE;

fn subject() -> CardIndex {
    card_index("2625fa9c-638c-46d3-966b-1166f440186f")
}

// Scenario: flash permits casting in opponent's main phase. Seven mana,
// tap and sacrifice destroy any targeted permanent, including a land.
#[test]
fn goblin_firebomb_flashes_and_can_destroy_a_land() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .battlefield(1, &[forest(), llanowar_elves(), quiet_artifact()])
        .hand(0, &[subject()])
        .start();
    keep_mulligans(&mut e);
    reach_their_main_phase(&mut e, p1);
    assert_eq!(e.state().turn.active, p1);
    e.apply(p1, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut e, p0, subject());
    pass_until(&mut e, stack_is_empty);
    let firebomb = on_battlefield(&e, p0, subject()).unwrap();
    assert_eq!(e.state().turn.active, p1);
    // The next own main phase can pay the seven-mana activation.
    assert!(walk_to_own_main(&mut e, p0));
    let land = on_battlefield(&e, p1, forest()).unwrap();
    let creature = on_battlefield(&e, p1, llanowar_elves()).unwrap();
    let artifact = on_battlefield(&e, p1, quiet_artifact()).unwrap();
    tap_mana_except(&mut e, p0, firebomb);
    activate(&mut e, p0, subject(), 0);
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("expected permanent target")
    };
    assert!(options.contains(&land) && options.contains(&creature) && options.contains(&artifact));
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![land],
        },
    )
    .unwrap();
    assert!(in_graveyard(&e, p0, subject()).is_some());
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, p1, forest()).is_none());
    assert!(in_graveyard(&e, p1, forest()).is_some());
    assert!(on_battlefield(&e, p1, llanowar_elves()).is_some());
    assert!(on_battlefield(&e, p1, quiet_artifact()).is_some());
}

#[test]
fn goblin_firebomb_destroys_each_kind_of_permanent() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for target_card in [
        llanowar_elves(),
        quiet_artifact(),
        their_enchantment(),
        jace_the_mind_sculptor(),
    ] {
        let mut e = Duel::new(SEED, mountain())
            .battlefield(
                0,
                &[
                    subject(),
                    mountain(),
                    mountain(),
                    mountain(),
                    mountain(),
                    mountain(),
                    mountain(),
                    mountain(),
                ],
            )
            .battlefield(1, &[target_card])
            .start();
        keep_mulligans(&mut e);
        assert!(walk_to_own_main(&mut e, p0));
        let target = on_battlefield(&e, p1, target_card).expect("opponent owns target");
        tap_all_mana_but(&mut e, p0, Some(subject()));
        activate(&mut e, p0, subject(), 0);
        let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
            panic!("expected target")
        };
        assert!(options.contains(&target));
        e.apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![target],
            },
        )
        .unwrap();
        pass_until(&mut e, stack_is_empty);
        assert!(on_battlefield(&e, p1, target_card).is_none());
        assert!(in_graveyard(&e, p1, target_card).is_some());
        assert!(in_graveyard(&e, p0, subject()).is_some());
    }
}

#[test]
fn goblin_firebomb_requires_seven_mana_and_untapped_source() {
    let p0 = PlayerId::new(0);
    let mut short = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                subject(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut short);
    assert!(walk_to_own_main(&mut short, p0));
    let source = on_battlefield(&short, p0, subject()).unwrap();
    tap_all_mana_but(&mut short, p0, Some(subject()));
    assert_eq!(short.state().players[0].mana_pool.total(), 6);
    let before = short.snapshot_hash();
    assert!(
        short
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source,
                    ability_index: 0
                }
            )
            .is_err()
    );
    assert_eq!(short.snapshot_hash(), before);
    assert!(on_battlefield(&short, p0, subject()).is_some());

    let mut tapped = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                subject(),
                island(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .battlefield(1, &[forest()])
        .hand(0, &[TWIDDLE])
        .start();
    keep_mulligans(&mut tapped);
    assert!(walk_to_own_main(&mut tapped, p0));
    let source = on_battlefield(&tapped, p0, subject()).unwrap();
    cast_from_hand(&mut tapped, p0, TWIDDLE);
    tapped
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![source],
            },
        )
        .unwrap();
    pass_until(&mut tapped, |game| {
        matches!(game.pending(), Pending::YesNo { .. })
    });
    tapped.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut tapped, stack_is_empty);
    assert!(is_tapped(&tapped, source));
    assert_eq!(tapped.state().players[0].mana_pool.total(), 7);
    let before = tapped.snapshot_hash();
    assert!(
        tapped
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source,
                    ability_index: 0
                }
            )
            .is_err()
    );
    assert_eq!(tapped.snapshot_hash(), before);
    assert!(on_battlefield(&tapped, p0, subject()).is_some());
}
