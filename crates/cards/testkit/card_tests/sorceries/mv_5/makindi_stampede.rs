//! `cards/sorceries/mv_5/makindi_stampede.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Makindi Stampede` // `Makindi Mesas` (`Coverage::Implemented`): "Creatures you control
/// get +2/+2 until end of turn. // This land enters tapped. {T}: Add {W}."
///
/// Marked `Coverage::Implemented`, casting the front face for `{3}{W}{W}` executes
/// `Effect::PumpFilter` over `Filter::YOUR_CREATURE`. The test verifies that controlled
/// creatures receive the +2/+2 buff until end of turn while an opponent's creature is
/// untouched, and confirms the spell resolves to the graveyard.
#[test]
fn makindi_stampede_pumps_controlled_creatures_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(478, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[makindi_stampede()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let my_elf = on_battlefield(&engine, p0, llanowar_elves()).expect("controlled elf");
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("opponent elf");
    assert_eq!(pt(&engine, my_elf), (1, 1));
    assert_eq!(pt(&engine, their_elf), (1, 1));

    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    cast_front_face(&mut engine, p0, makindi_stampede());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, my_elf),
        (3, 3),
        "controlled creature gets +2/+2 until end of turn"
    );
    assert_eq!(
        pt(&engine, their_elf),
        (1, 1),
        "opponent creature is not pumped"
    );
    assert!(
        in_graveyard(&engine, p0, makindi_stampede()).is_some(),
        "Makindi Stampede moved to graveyard after resolving"
    );
}
