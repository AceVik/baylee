//! `cards/enchantments/mv_3/mudslide.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn mudslide() -> CardIndex {
    card_index("ac1d7dfe-1454-46a9-acf6-66ae86b6a6c5")
}

fn zephyr_falcon() -> CardIndex {
    baylee_core::generated::index::ZEPHYR_FALCON
}

fn tapped_creatures_fixture() -> (Engine<RegistryLookup>, [ObjectId; 5]) {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mudslide(),
                grizzly_bears(),
                gray_ogre(),
                zephyr_falcon(),
                mountain(),
                mountain(),
            ],
        )
        .battlefield(
            1,
            &[grizzly_bears(), zephyr_falcon(), mountain(), mountain()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let p0_bears = on_battlefield(&engine, P0, grizzly_bears()).unwrap();
    let p0_ogre = on_battlefield(&engine, P0, gray_ogre()).unwrap();
    let p0_falcon = on_battlefield(&engine, P0, zephyr_falcon()).unwrap();
    let p1_bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    let p1_falcon = on_battlefield(&engine, P1, zephyr_falcon()).unwrap();

    {
        let state = engine.dev_state_mut(P0).unwrap();
        for id in [p0_bears, p0_ogre, p0_falcon, p1_bears, p1_falcon] {
            state.set_tapped(id, true);
        }
    }
    engine.refresh_offer();

    (engine, [p0_bears, p0_ogre, p0_falcon, p1_bears, p1_falcon])
}

/// Mudslide (ac1d7dfe-1454-46a9-acf6-66ae86b6a6c5) — {2}{R} Enchantment:
/// - "Creatures without flying don't untap during their controllers' untap steps."
/// - "At the beginning of each player's upkeep, that player may choose any number of tapped
///   creatures without flying they control and pay {2} for each creature chosen this way. If
///   the player does, untap those creatures."
#[test]
fn mudslide_keeps_non_fliers_tapped_while_fliers_untap_and_offers_two_mana_upkeep_untap() {
    let (mut engine, [p0_bears, p0_ogre, p0_falcon, p1_bears, p1_falcon]) =
        tapped_creatures_fixture();

    // 1. Advance to P1's upkeep: P1's flying Zephyr Falcon untapped in P1's untap step,
    // while P1's non-flying Grizzly Bears stayed tapped and is offered to P1.
    // P1 chooses nothing (declines).
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::Untap,
                ..
            }
        )
    });
    assert!(
        !is_tapped(&engine, p1_falcon),
        "creature with flying untaps normally during its controller's untap step"
    );
    assert!(
        is_tapped(&engine, p1_bears),
        "creature without flying does not untap during its controller's untap step"
    );
    let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseCards(Untap)");
    };
    assert_eq!(player, P1);
    assert_eq!(
        options,
        vec![p1_bears],
        "only P1's tapped non-flying creature is offered on P1's upkeep"
    );
    engine
        .apply(P1, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    assert!(
        is_tapped(&engine, p1_bears),
        "declining to choose leaves P1's non-flying creature tapped"
    );

    // 2. Advance to P0's upkeep: P0's flying Zephyr Falcon untapped in P0's untap step,
    // while P0's Grizzly Bears and Gray Ogre stayed tapped. P0 chooses Grizzly Bears and pays {2}.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && matches!(
                e.pending(),
                Pending::ChooseCards {
                    prompt: ChoicePrompt::Untap,
                    ..
                }
            )
    });
    assert!(
        !is_tapped(&engine, p0_falcon),
        "P0's flying Zephyr Falcon untapped in P0's untap step"
    );
    assert!(is_tapped(&engine, p0_bears) && is_tapped(&engine, p0_ogre));
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![p0_bears],
            },
        )
        .unwrap();
    assert!(matches!(
        engine.pending(),
        Pending::YesNo {
            prompt: YesNoPrompt::PayMana { .. },
            ..
        }
    ));
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    tap_all_mana(&mut engine, P0);
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::Upkeep && stack_is_empty(e)
    });

    assert!(
        !is_tapped(&engine, p0_bears),
        "paying {{2}} untaps the chosen non-flying creature"
    );
    assert!(
        is_tapped(&engine, p0_ogre),
        "unchosen non-flying creature remains tapped"
    );
}
