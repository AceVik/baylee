//! `cards/creatures/mv_5/zirilan_of_the_claw.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn zirilan_of_the_claw() -> CardIndex {
    card_index("3aeb28ac-8ed2-4a90-a631-d84361ac6acc")
}

/// Zirilan of the Claw (`{3}{R}{R}`, `Coverage::Partial`): 3/4 Legendary
/// Lizard Shaman with `{1}{R}{R}, {T}: Search your library for a Dragon
/// permanent card, put that card onto the battlefield, then shuffle.`
#[test]
fn zirilan_of_the_claw_searches_library_for_dragon_onto_battlefield() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, shivan_dragon())
        .battlefield(
            0,
            &[zirilan_of_the_claw(), mountain(), mountain(), mountain()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let zirilan = on_battlefield(&engine, p0, zirilan_of_the_claw()).unwrap();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: zirilan,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!();
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, shivan_dragon()).is_some());
}
