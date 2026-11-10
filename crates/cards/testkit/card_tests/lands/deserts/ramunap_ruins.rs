//! `cards/lands/deserts/ramunap_ruins.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ramunap Ruins: "{T}: Add {C}." / "{T}, Pay 1 life: Add {R}." / "{2}{R}{R}, {T}, Sacrifice a Desert: This land deals 2 damage to each opponent."
/// Paid with four Mountains, activating ability 2 prompts to sacrifice a Desert as cost.
/// Sacrificing Ramunap Ruins itself deals 2 damage to the opponent upon resolution, reducing their life from 20 to 18.
#[test]
fn ramunap_ruins_sacrifices_desert_to_deal_damage_to_opponent() {
    let (p0, _p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(127, mountain())
        .battlefield(
            0,
            &[
                ramunap_ruins(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ruins = on_battlefield(&engine, p0, ramunap_ruins()).expect("Ruins deployed");
    tap_mana_except(&mut engine, p0, ruins);

    activate(&mut engine, p0, ramunap_ruins(), 2);

    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected sacrifice choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ruins), "Ramunap Ruins is a Desert");

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ruins],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 18);
    assert!(in_graveyard(&engine, p0, ramunap_ruins()).is_some());
}
