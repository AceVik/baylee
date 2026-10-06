//! `cards/lands/deserts/lazotep_quarry.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Lazotep Quarry: "{T}: Add {C}." / "{T}, Sacrifice a creature: Add one mana of any color." / "{X}{2}, {T}, Sacrifice a Desert..."
/// Under `Coverage::Partial`, the `{X}{2}` reanimation ability is omitted because no filter compares mana values against X.
/// Activating ability 1 sacrifices a creature you control and adds one mana of any chosen color.
#[test]
fn lazotep_quarry_sacrifices_creature_to_produce_colored_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(307, forest())
        .battlefield(0, &[lazotep_quarry(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("elf deployed");
    activate(&mut engine, p0, lazotep_quarry(), 1);

    let Pending::ChooseCards {
        options, prompt, ..
    } = engine.pending().clone()
    else {
        panic!("expected sacrifice choice, got {:?}", engine.pending());
    };
    assert_eq!(prompt, crate::choice::ChoicePrompt::CostSacrifice);
    assert!(options.contains(&elf));

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();

    let Pending::ChooseColor { .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .unwrap();

    assert!(in_graveyard(&engine, p0, llanowar_elves()).is_some());
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 1);
    let quarry = on_battlefield(&engine, p0, lazotep_quarry()).expect("quarry on battlefield");
    assert!(is_tapped(&engine, quarry));
}
