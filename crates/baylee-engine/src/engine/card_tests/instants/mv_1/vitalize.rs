//! `cards/instants/mv_1/vitalize.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn vitalize() -> CardIndex {
    card_index("cd3226c6-bd47-4246-8ab3-01316c5d1809")
}

/// `Vitalize` ({G}, Instant) untaps all creatures its caster controls.
#[test]
fn vitalize_untaps_all_creatures_you_control() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[quiet_creature(), GRIZZLY_BEARS, forest()])
        .battlefield(1, &[GRIZZLY_BEARS])
        .hand(0, &[vitalize()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = on_battlefield(&engine, p0, forest()).expect("own land");
    let elf = on_battlefield(&engine, p0, quiet_creature()).expect("elf");
    let own_bear = on_battlefield(&engine, p0, GRIZZLY_BEARS).expect("own bear");
    let opp_bear = on_battlefield(&engine, p1, GRIZZLY_BEARS).expect("opp bear");
    for id in [elf, own_bear, opp_bear] {
        engine
            .dev_state_mut(p0)
            .unwrap()
            .object_mut(id)
            .unwrap()
            .status
            .insert(Status::TAPPED);
    }

    cast_from_hand(&mut engine, p0, vitalize());
    pass_until(&mut engine, stack_is_empty);

    assert!(!is_tapped(&engine, elf));
    assert!(!is_tapped(&engine, own_bear));
    assert!(is_tapped(&engine, opp_bear));
    assert!(
        is_tapped(&engine, land),
        "the Forest used to cast Vitalize stays tapped"
    );
}
