//! `cards/creatures/mv_3/vodalian_illusionist.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
fn vodalian_illusionist() -> CardIndex {
    card_index("ac935639-ba30-4b04-86f6-363527e85a8b")
}

/// `Vodalian Illusionist` ({2}{U}, 2/2 Merfolk Wizard) pays `{U}{U}, {T}` to
/// phase out target creature.
#[test]
fn vodalian_illusionist_phases_out_target_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[vodalian_illusionist(), island(), island()])
        .battlefield(1, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let illusionist = on_battlefield(&engine, p0, vodalian_illusionist()).expect("Illusionist");
    let target = on_battlefield(&engine, p1, quiet_creature()).expect("target creature");

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, vodalian_illusionist(), 0);
    aim_at(&mut engine, p0, target);
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, illusionist));
    assert!(
        engine
            .state()
            .object(target)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT)
    );
}
