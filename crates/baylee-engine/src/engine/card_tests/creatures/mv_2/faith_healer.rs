#[allow(clippy::wildcard_imports)]
use super::*;

fn faith_healer() -> CardIndex {
    card_index("c657e110-c204-48df-a396-9822ac48798d")
}

#[test]
fn faith_healer_sacrifices_only_your_enchantment_to_gain_its_mana_value() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[faith_healer(), sylvan_library()])
        .battlefield(1, &[sylvan_library()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lib = on_battlefield(&engine, p0, sylvan_library()).expect("seated");
    activate(&mut engine, p0, faith_healer(), 0);
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("enchantment sacrifice");
    };
    assert_eq!(
        options,
        vec![lib],
        "opponent enchantment and own creature are excluded"
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![lib] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, sylvan_library()).is_none());
    assert_eq!(engine.state().players[0].life, 22);
}
