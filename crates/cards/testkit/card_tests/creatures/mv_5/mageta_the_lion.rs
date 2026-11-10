//! `cards/creatures/mv_5/mageta_the_lion.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn mageta_the_lion() -> CardIndex {
    card_index("cd170b1c-879b-414e-9f0e-5e7fb099e7bb")
}

/// Mageta the Lion ({3}{W}{W} 3/3 Legendary Creature — Human Spellshaper):
/// "{2}{W}{W}, {T}, Discard two cards: Destroy all creatures except for Mageta. Those creatures can't be regenerated."
#[test]
fn mageta_the_lion_destroys_all_other_creatures_and_spares_itself() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                mageta_the_lion(),
                grizzly_bears(),
                plains(),
                plains(),
                plains(),
                plains(),
            ],
        )
        .hand(0, &[plains(), forest()])
        .battlefield(1, &[index::HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, mageta_the_lion(), 0);
    for _ in 0..2 {
        let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
            panic!(
                "expected ChooseCards for discard cost, got {:?}",
                engine.pending()
            );
        };
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![options[0]],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, mageta_the_lion()).is_some());
    assert!(on_battlefield(&engine, p0, grizzly_bears()).is_none());
    assert!(in_graveyard(&engine, p0, grizzly_bears()).is_some());
    assert!(on_battlefield(&engine, p1, index::HILL_GIANT).is_none());
    assert!(in_graveyard(&engine, p1, index::HILL_GIANT).is_some());
}

/// Creatures destroyed by Mageta the Lion's ability cannot be regenerated.
#[test]
fn mageta_the_lion_prevents_regeneration_on_other_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[mageta_the_lion(), plains(), plains(), plains(), plains()],
        )
        .hand(0, &[plains(), forest()])
        .battlefield(1, &[index::DRUDGE_SKELETONS, swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, mageta_the_lion(), 0);
    for _ in 0..2 {
        let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
            panic!(
                "expected ChooseCards for discard cost, got {:?}",
                engine.pending()
            );
        };
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![options[0]],
                },
            )
            .unwrap();
    }
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );
    tap_all_mana(&mut engine, p1);
    activate(&mut engine, p1, index::DRUDGE_SKELETONS, 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, mageta_the_lion()).is_some());
    assert!(on_battlefield(&engine, p1, index::DRUDGE_SKELETONS).is_none());
    assert!(in_graveyard(&engine, p1, index::DRUDGE_SKELETONS).is_some());
}

#[test]
fn mageta_the_lion_old_ability_destroys_its_blinked_new_incarnation() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let blink = card_index("0fd57894-b917-41c8-a394-360d1d31b236");
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                mageta_the_lion(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
            ],
        )
        .hand(0, &[plains(), forest(), blink])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mageta = on_battlefield(&engine, p0, mageta_the_lion()).unwrap();
    let original_version = engine.state().object(mageta).unwrap().version;
    let discards = [
        in_hand(&engine, p0, plains()).unwrap(),
        in_hand(&engine, p0, forest()).unwrap(),
    ];
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, mageta_the_lion(), 0);
    for card in discards {
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![card],
                },
            )
            .unwrap();
    }
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    cast_with_floating(&mut engine, p0, blink);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mageta],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state()
            .object(mageta)
            .is_some_and(|object| object.version != original_version)
    });
    assert_eq!(
        engine.state().object(mageta).unwrap().zone,
        Zone::Battlefield
    );
    assert!(
        !stack_is_empty(&engine),
        "Mageta's original ability still waits"
    );

    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p0, mageta_the_lion()).is_some());
    assert!(in_graveyard(&engine, p1, grizzly_bears()).is_some());
}
