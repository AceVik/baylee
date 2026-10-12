//! `cards/instants/mv_3/teferi_s_protection.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The complete spell creates its two rules, phases permanents, and exiles itself.
#[test]
fn teferis_protection_exiles_itself_and_protects_its_controller() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), llanowar_elves()])
        .hand(0, &[teferis_protection()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is on the table");
    let effects_before = engine.state().effects.iter().count();
    let life_before = engine.state().players[0].life;

    cast_from_hand(&mut engine, p0, teferis_protection());
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p0))
            .iter()
            .any(|id| {
                engine
                    .state()
                    .object(*id)
                    .is_some_and(|o| o.card.is_some_and(|c| c.index == teferis_protection()))
            }),
        "the spell stays exiled"
    );

    assert_eq!(
        engine.state().effects.iter().count(),
        effects_before + 2,
        "both continuous player rules are registered"
    );
    assert_eq!(
        engine.state().players[0].life,
        life_before,
        "and nothing touched the life total the first of them is about"
    );
    assert!(
        engine
            .state()
            .object(elf)
            .is_some_and(|o| o.status.contains(Status::PHASED_OUT)),
        "the Elf is phased out"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "and it is still on the battlefield, where a phased-out permanent \
         would also be"
    );
}
