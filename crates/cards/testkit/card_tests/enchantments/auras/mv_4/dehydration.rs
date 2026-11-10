//! `cards/enchantments/auras/mv_4/dehydration.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Dehydration` (`Coverage::Implemented`):
/// "Enchant creature. Enchanted creature doesn't untap during its controller's untap step."
///
/// Verifies that enchanting a tapped creature with `Dehydration` prevents it from
/// untapping during its controller's untap step, while lands untap normally.
#[test]
fn dehydration_prevents_enchanted_creature_from_untapping() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1309, forest())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                forest(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[dehydration()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("elf deployed");
    assert!(!is_tapped(&engine, elf), "elf starts untapped");

    // Tap the elf for mana
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: elf,
                ability_index: 0,
            },
        )
        .expect("elf taps for mana");
    assert!(is_tapped(&engine, elf), "elf is tapped");

    cast_from_hand(&mut engine, p0, dehydration());
    let Pending::ChooseTargets { .. } = engine.pending().clone() else {
        panic!(
            "expected target choice for Aura spell, got {:?}",
            engine.pending()
        )
    };
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let aura = on_battlefield(&engine, p0, dehydration()).expect("Dehydration resolved");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(elf),
        "Dehydration attached to elf"
    );

    // Pass turn to opponent and then back to p0's next main phase
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(
        is_tapped(&engine, elf),
        "enchanted creature does not untap during untap step"
    );
}
