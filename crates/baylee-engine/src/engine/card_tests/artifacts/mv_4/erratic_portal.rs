//! `cards/artifacts/mv_4/erratic_portal.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn erratic_portal() -> CardIndex {
    card_index("4a85cd7e-625b-4043-b7f1-999def26eb36")
}

/// "{1}, {T}: Return target creature to its owner's hand unless its controller pays {1}."
#[test]
fn erratic_portal_returns_target_creature_unless_controller_pays_one() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for pay in [false, true] {
        let mut engine = Duel::new(SEED, forest())
            .battlefield(0, &[erratic_portal(), forest()])
            .battlefield(1, &[gray_ogre(), forest()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();
        if pay {
            engine
                .dev_state_mut(p1)
                .unwrap()
                .players[1]
                .mana_pool
                .add(ManaColor::Green, 1);
        }
        tap_all_mana(&mut engine, p0);
        activate(&mut engine, p0, erratic_portal(), 0);
        aim_at(&mut engine, p0, ogre);
        pass_until(&mut engine, |e| {
            matches!(
                e.pending(),
                Pending::YesNo {
                    prompt: YesNoPrompt::PayTax { .. },
                    ..
                }
            )
        });
        engine.apply(p1, PlayerAction::YesNo(pay)).unwrap();
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(on_battlefield(&engine, p1, gray_ogre()).is_some(), pay);
        assert_eq!(in_hand(&engine, p1, gray_ogre()).is_some(), !pay);
    }
}
