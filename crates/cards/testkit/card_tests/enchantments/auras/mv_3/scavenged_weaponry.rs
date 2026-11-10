//! `cards/enchantments/auras/mv_3/scavenged_weaponry.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Scavenged Weaponry` (`Coverage::Implemented`):
/// "Enchant creature. When this Aura enters, draw a card. Enchanted creature gets +1/+1."
///
/// Verifies that casting `Scavenged Weaponry` attaches to the targeted creature, gives it
/// +1/+1, and triggers an enters-the-battlefield ability that draws a card.
#[test]
fn scavenged_weaponry_pumps_and_draws_a_card_on_entering() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1307, forest())
        .battlefield(0, &[swamp(), swamp(), swamp(), llanowar_elves()])
        .hand(0, &[scavenged_weaponry()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("elf deployed");
    assert_eq!(pt(&engine, elf), (1, 1));
    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(p0)).len(), 1);

    cast_from_hand(&mut engine, p0, scavenged_weaponry());
    let Pending::ChooseTargets { .. } = engine.pending().clone() else {
        panic!(
            "expected target choice for Aura spell, got {:?}",
            engine.pending()
        )
    };
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();

    // Resolves the Aura and then resolves the enters-the-battlefield draw trigger
    pass_until(&mut engine, stack_is_empty);

    let aura = on_battlefield(&engine, p0, scavenged_weaponry()).expect("Aura resolved");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(elf),
        "Aura attached to chosen elf"
    );
    assert_eq!(pt(&engine, elf), (2, 2), "1/1 elf gets +1/+1 to become 2/2");
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        1,
        "drew a card from the enters-the-battlefield trigger"
    );
}
