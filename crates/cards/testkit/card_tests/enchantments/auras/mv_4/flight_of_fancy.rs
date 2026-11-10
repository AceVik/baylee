//! `cards/enchantments/auras/mv_4/flight_of_fancy.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Flight of Fancy` (`Coverage::Implemented`):
/// "Enchant creature. When this Aura enters, draw two cards. Enchanted creature has flying."
///
/// Verifies that casting `Flight of Fancy` grants flying to the enchanted creature
/// and triggers an enters-the-battlefield ability that draws two cards.
#[test]
fn flight_of_fancy_grants_flying_and_draws_two_cards_on_entering() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1312, forest())
        .battlefield(
            0,
            &[island(), island(), island(), island(), llanowar_elves()],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[flight_of_fancy()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my elf deployed");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their elf deployed");
    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(p0)).len(), 1);

    cast_from_hand(&mut engine, p0, flight_of_fancy());
    let Pending::ChooseTargets { .. } = engine.pending().clone() else {
        panic!(
            "expected target choice for Aura spell, got {:?}",
            engine.pending()
        )
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .unwrap();

    // Resolves the Aura and then resolves the ETB draw trigger
    pass_until(&mut engine, stack_is_empty);

    let aura = on_battlefield(&engine, p0, flight_of_fancy()).expect("Aura resolved");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(mine),
        "Aura attached to chosen elf"
    );
    assert!(
        keywords(&engine, mine).contains(KeywordSet::FLYING),
        "enchanted creature gains flying"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        2,
        "drew two cards from the enters-the-battlefield trigger"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FLYING),
        "opponent elf does not gain flying"
    );
}
